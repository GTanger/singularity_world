//! 創作者書頁式人物列傳與跨區關係閱讀器 API（`/chronicle`、`/api/chronicle/*`）。
//!
//! 存取邊界（對齊 `/world_atlas` 慣例，支援手機外網直連 `https://sw.ygggt.com/chronicle`）：
//! - 玩家主介面 `web/index.html` 不暴露 `/chronicle` 連結，且嚴格排除真實玩家（`pl:`）進入列傳。
//! - 放行自有網域（`sw.ygggt.com` / `*.ygggt.com`）、`localhost` 與內網 IP（手機 5G／Wi-Fi 經 Cloudflare Tunnel 可直接開啟閱讀）；同時攔截惡意外部 `Host`/`Origin`（防 DNS Rebinding）與 `cross-site` CSRF。
//! - 若環境變數另設非空 `SW_CREATOR_TOKEN`，則額外要求攜帶該憑證。

use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static EXPORT_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

use axum::extract::{ConnectInfo, Query};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::json;

use crate::grid::outer_island::atlas_observatory::{
    build_atlas_node_detail_json, build_atlas_snapshot_json,
};
use crate::grid::outer_island::chronicle_export::{
    build_chronicle_actor_json, build_chronicle_article_json, build_chronicle_index_json,
    export_markdown_chronicles_staged_atomic,
};
use crate::grid::outer_island::global_island;

const CHRONICLE_PAGE_HTML: &str = include_str!("chronicle_page.html");

#[derive(Debug, Default, Deserialize)]
pub struct ChronicleQuery {
    pub id: Option<String>,
    pub token: Option<String>,
}

pub async fn world_atlas_page_get(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ChronicleQuery>,
) -> Response {
    if !check_creator_access(peer.ip(), &headers, q.token.as_deref()) {
        return forbidden_response();
    }
    match tokio::fs::read_to_string("web/world_atlas.html").await {
        Ok(content) => (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (
                    header::CACHE_CONTROL,
                    "no-cache, max-age=0, must-revalidate",
                ),
                (header::REFERRER_POLICY, "no-referrer"),
                (header::X_FRAME_OPTIONS, "DENY"),
            ],
            Html(content),
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

pub async fn world_atlas_snapshot_get(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ChronicleQuery>,
) -> Response {
    if !check_creator_access(peer.ip(), &headers, q.token.as_deref()) {
        return forbidden_response();
    }
    let eng = global_island();
    let payload = match eng.delta.read() {
        Ok(guard) => build_atlas_snapshot_json(&eng.doc, &guard),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "ok": false, "error": "世界讀鎖忙碌" })),
            )
                .into_response();
        }
    };
    Json(payload).into_response()
}

pub async fn world_atlas_node_get(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ChronicleQuery>,
) -> Response {
    if !check_creator_access(peer.ip(), &headers, q.token.as_deref()) {
        return forbidden_response();
    }
    let Some(target_id) = q.id.as_deref().filter(|s| !s.trim().is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "缺少場所或地塊代號 id" })),
        )
            .into_response();
    };
    let eng = global_island();
    let payload_opt = match eng.delta.read() {
        Ok(guard) => build_atlas_node_detail_json(&eng.doc, &guard, target_id),
        Err(_) => None,
    };
    match payload_opt {
        Some(val) => Json(val).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "ok": false, "error": "查無此場所或地塊" })),
        )
            .into_response(),
    }
}

fn is_private_or_loopback_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || (v4.octets()[0] == 100 && (64..=127).contains(&v4.octets()[1]))
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6
                    .to_ipv4_mapped()
                    .is_some_and(|v4| is_private_or_loopback_ip(IpAddr::V4(v4)))
        }
    }
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn is_allowed_host_authority(authority: &str) -> bool {
    let trimmed = authority.trim();
    if trimmed.is_empty() {
        return false;
    }
    let host = if let Some(rest) = trimmed.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else {
        trimmed.split(':').next().unwrap_or("")
    };
    let lower = host.to_ascii_lowercase();
    lower == "localhost"
        || lower == "sw.ygggt.com"
        || lower.ends_with(".ygggt.com")
        || host.parse::<IpAddr>().is_ok_and(is_private_or_loopback_ip)
}

pub fn check_creator_access(
    _peer_ip: IpAddr,
    headers: &HeaderMap,
    query_token: Option<&str>,
) -> bool {
    if let Ok(expected) = std::env::var("SW_CREATOR_TOKEN") {
        let exp = expected.trim();
        if !exp.is_empty() {
            let query_ok = query_token.is_some_and(|t| ct_eq(t.as_bytes(), exp.as_bytes()));
            let hdr_ok = headers
                .get("x-creator-token")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|t| ct_eq(t.as_bytes(), exp.as_bytes()));
            if !query_ok && !hdr_ok {
                return false;
            }
        }
    }

    // 防 DNS Rebinding 與跨站偷讀／CSRF：Host 與 Origin（若有）必須為 sw.ygggt.com、localhost 或內網 IP，且拒絕 cross-site
    if let Some(host_val) = headers.get(header::HOST) {
        let Ok(host_str) = host_val.to_str() else {
            return false;
        };
        if !is_allowed_host_authority(host_str) {
            return false;
        }
    }
    if headers
        .get("sec-fetch-site")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.eq_ignore_ascii_case("cross-site"))
    {
        return false;
    }
    if let Some(orig_val) = headers.get(header::ORIGIN) {
        let Ok(orig_str) = orig_val.to_str() else {
            return false;
        };
        let without_scheme = orig_str
            .split_once("://")
            .map_or(orig_str, |(_, rest)| rest)
            .split('/')
            .next()
            .unwrap_or("");
        if !is_allowed_host_authority(without_scheme) {
            return false;
        }
    }

    true
}

fn forbidden_response() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "ok": false,
            "error": "拒絕跨站或未知網域存取人物列傳閱讀器。"
        })),
    )
        .into_response()
}

pub async fn chronicle_page_get(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ChronicleQuery>,
) -> Response {
    if !check_creator_access(peer.ip(), &headers, q.token.as_deref()) {
        return forbidden_response();
    }
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (
                header::CACHE_CONTROL,
                "no-cache, max-age=0, must-revalidate",
            ),
            (header::REFERRER_POLICY, "no-referrer"),
            (header::X_FRAME_OPTIONS, "DENY"),
        ],
        Html(CHRONICLE_PAGE_HTML),
    )
        .into_response()
}

pub async fn chronicle_index_get(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ChronicleQuery>,
) -> Response {
    if !check_creator_access(peer.ip(), &headers, q.token.as_deref()) {
        return forbidden_response();
    }
    let eng = global_island();
    let payload = match eng.delta.read() {
        Ok(guard) => build_chronicle_index_json(&eng.doc, &guard),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "ok": false, "error": "世界讀鎖忙碌" })),
            )
                .into_response();
        }
    };
    Json(payload).into_response()
}

pub async fn chronicle_actor_get(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ChronicleQuery>,
) -> Response {
    if !check_creator_access(peer.ip(), &headers, q.token.as_deref()) {
        return forbidden_response();
    }
    let Some(target_id) = q.id.as_deref().filter(|s| !s.trim().is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "缺少人物代號 id" })),
        )
            .into_response();
    };
    let eng = global_island();
    let payload_opt = match eng.delta.read() {
        Ok(guard) => build_chronicle_actor_json(&eng.doc, &guard, target_id.trim()),
        Err(_) => None,
    };
    match payload_opt {
        Some(val) => Json(val).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "ok": false, "error": "查無此常住居民列傳" })),
        )
            .into_response(),
    }
}

pub async fn chronicle_article_get(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ChronicleQuery>,
) -> Response {
    if !check_creator_access(peer.ip(), &headers, q.token.as_deref()) {
        return forbidden_response();
    }
    let Some(target_id) = q.id.as_deref().filter(|s| !s.trim().is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "缺少卷篇代號 id" })),
        )
            .into_response();
    };
    let eng = global_island();
    let payload_opt = match eng.delta.read() {
        Ok(guard) => build_chronicle_article_json(&eng.doc, &guard, target_id.trim()),
        Err(_) => None,
    };
    match payload_opt {
        Some(val) => Json(val).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "ok": false, "error": "查無此方輿或戰史卷篇" })),
        )
            .into_response(),
    }
}

pub async fn chronicle_export_md_post(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ChronicleQuery>,
) -> Response {
    if !check_creator_access(peer.ip(), &headers, q.token.as_deref()) {
        return forbidden_response();
    }
    // F2 防無預檢跨站 POST CSRF：強制要求自訂標頭 `x-chronicle-action: export`
    if headers
        .get("x-chronicle-action")
        .and_then(|v| v.to_str().ok())
        != Some("export")
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "缺少 x-chronicle-action: export 確認標頭" })),
        )
            .into_response();
    }
    if EXPORT_IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "ok": false, "error": "列傳正在匯出中，請稍候" })),
        )
            .into_response();
    }
    struct ExportFlightGuard;
    impl Drop for ExportFlightGuard {
        fn drop(&mut self) {
            EXPORT_IN_FLIGHT.store(false, Ordering::Release);
        }
    }
    let _flight_guard = ExportFlightGuard;
    let eng = global_island();
    let delta_snap = match eng.delta.read() {
        Ok(guard) => guard.clone(),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "ok": false, "error": "世界讀鎖忙碌" })),
            )
                .into_response();
        }
    };
    let doc_ref = &eng.doc;
    let res = tokio::task::spawn_blocking(move || {
        export_markdown_chronicles_staged_atomic(
            doc_ref,
            &delta_snap,
            Path::new("docs/chronicles/銀鷗島人物列傳"),
        )
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|inner| inner);
    drop(_flight_guard);
    match res {
        Ok(written) => Json(json!({
            "ok": true,
            "written_actors": written,
            "dir": "docs/chronicles/銀鷗島人物列傳"
        }))
        .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": err })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn creator_access_allows_phone_tunnel_and_blocks_rebound_or_cross_site() {
        let empty = HeaderMap::new();
        assert!(check_creator_access(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &empty,
            None
        ));

        // 手機 5G / 外網經 Cloudflare Tunnel 連入 `sw.ygggt.com` 必須暢通放行！
        let mut phone_cf = HeaderMap::new();
        phone_cf.insert(header::HOST, HeaderValue::from_static("sw.ygggt.com"));
        phone_cf.insert("cf-connecting-ip", HeaderValue::from_static("203.0.113.42"));
        assert!(check_creator_access(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &phone_cf,
            None
        ));

        // DNS Rebinding：Host 為惡意外部網域時必須拒絕！
        let mut dns_rebind = HeaderMap::new();
        dns_rebind.insert(header::HOST, HeaderValue::from_static("evil.example:1721"));
        assert!(!check_creator_access(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &dns_rebind,
            None
        ));

        let mut local_host = HeaderMap::new();
        local_host.insert(header::HOST, HeaderValue::from_static("127.0.0.1:1721"));
        assert!(check_creator_access(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &local_host,
            None
        ));

        let mut evil_origin = HeaderMap::new();
        evil_origin.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://evil.example"),
        );
        assert!(!check_creator_access(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &evil_origin,
            None
        ));

        let mut cross_site = HeaderMap::new();
        cross_site.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        assert!(!check_creator_access(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &cross_site,
            None
        ));

        assert!(check_creator_access(
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            &empty,
            None
        ));
    }
}
