use gloo_net::http::Request;
use serde::Serialize;

use crate::types::*;

/// 從目前頁面網址 `?mg_key=` 讀取管理金鑰（與伺服器 `MANAGEMENT_KEY` 對應）；未設定時為空（僅在伺服器未設金鑰時可用）。
fn mg_key_from_url() -> String {
    let search = match web_sys::window().and_then(|w| w.location().search().ok()) {
        Some(s) if !s.is_empty() && s != "?" => s,
        _ => return String::new(),
    };
    let query = if search.starts_with('?') {
        search
    } else {
        format!("?{search}")
    };
    web_sys::UrlSearchParams::new_with_str(&query)
        .ok()
        .and_then(|p| p.get("mg_key"))
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}

fn encode_uri_component(s: &str) -> String {
    js_sys::encode_uri_component(s)
        .as_string()
        .unwrap_or_else(|| s.to_string())
}

fn api_url(path: &str) -> String {
    let mk = mg_key_from_url();
    if mk.is_empty() {
        path.to_string()
    } else {
        let enc = encode_uri_component(&mk);
        format!("{path}?mg_key={enc}")
    }
}

fn http_err(resp: &gloo_net::http::Response, fallback: &str) -> String {
    let code = resp.status();
    if code == 403 {
        return "403：請在網址加上 ?mg_key=（與伺服器 MANAGEMENT_KEY 相同）".to_string();
    }
    format!("{fallback} HTTP {code}")
}

pub async fn load_grid() -> Result<GridResponse, String> {
    let resp = Request::get(&api_url("/api/hex/grid"))
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(http_err(&resp, "載入"));
    }
    resp.json::<GridResponse>()
        .await
        .map_err(|e| format!("JSON 解析失敗：{e}"))
}


pub async fn put_cell(cell: &CellPutReq) -> Result<(), String> {
    let resp = Request::put(&api_url("/api/hex/cell"))
        .json(cell)
        .map_err(|e| format!("序列化失敗：{e}"))?
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(http_err(&resp, "寫入格子"));
    }
    Ok(())
}

pub async fn delete_cell(q: i32, r: i32) -> Result<(), String> {
    let resp = Request::delete(&api_url(&format!("/api/hex/cell/{q}/{r}")))
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(http_err(&resp, "刪除格子"));
    }
    Ok(())
}

pub async fn put_cells(cells: &[CellPutReq]) -> Result<usize, String> {
    let resp = Request::put(&api_url("/api/hex/cells"))
        .json(cells)
        .map_err(|e| format!("序列化失敗：{e}"))?
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(http_err(&resp, "批次寫入"));
    }
    #[derive(serde::Deserialize)]
    struct Resp {
        count: usize
    }
    let r: Resp = resp.json().await.map_err(|e| format!("JSON：{e}"))?;
    Ok(r.count)
}

pub async fn save_grid() -> Result<usize, String> {
    let resp = Request::post(&api_url("/api/hex/save"))
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(http_err(&resp, "儲存"));
    }
    #[derive(serde::Deserialize)]
    struct Resp { count: usize }
    let r: Resp = resp.json().await.map_err(|e| format!("JSON：{e}"))?;
    Ok(r.count)
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RevealResponse {
    pub already_revealed: bool,
    pub cell: crate::types::HexCell,
}

pub async fn reveal_cell(q: i32, r: i32) -> Result<RevealResponse, String> {
    #[derive(Serialize)]
    struct Body {
        q: i32,
        r: i32,
    }
    let resp = Request::post(&api_url("/api/hex/reveal"))
        .json(&Body { q, r })
        .map_err(|e| format!("序列化失敗：{e}"))?
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(http_err(&resp, "揭露"));
    }
    resp.json::<RevealResponse>()
        .await
        .map_err(|e| format!("JSON：{e}"))
}

pub async fn reveal_region(center_q: i32, center_r: i32, radius: i32) -> Result<(usize, usize), String> {
    #[derive(Serialize)]
    struct Body {
        center_q: i32,
        center_r: i32,
        radius: i32,
    }
    #[derive(serde::Deserialize)]
    struct Resp {
        new_cells: usize,
        total_cells: usize,
    }
    let resp = Request::post(&api_url("/api/hex/reveal-region"))
        .json(&Body {
            center_q,
            center_r,
            radius,
        })
        .map_err(|e| format!("序列化失敗：{e}"))?
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(http_err(&resp, "區域揭露"));
    }
    let r: Resp = resp.json().await.map_err(|e| format!("JSON：{e}"))?;
    Ok((r.new_cells, r.total_cells))
}

pub async fn put_world_seed(world_seed: u64) -> Result<(), String> {
    #[derive(Serialize)]
    struct Body {
        world_seed: u64,
    }
    let resp = Request::put(&api_url("/api/hex/world-seed"))
        .json(&Body { world_seed })
        .map_err(|e| format!("序列化失敗：{e}"))?
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(http_err(&resp, "套用種子"));
    }
    Ok(())
}

pub async fn reload_grid() -> Result<usize, String> {
    let resp = Request::post(&api_url("/api/hex/reload"))
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(http_err(&resp, "重載"));
    }
    #[derive(serde::Deserialize)]
    struct Resp { count: usize }
    let r: Resp = resp.json().await.map_err(|e| format!("JSON：{e}"))?;
    Ok(r.count)
}

#[derive(Debug, Clone, Serialize)]
pub struct CellPutReq {
    pub q: i32,
    pub r: i32,
    pub terrain: Terrain,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub zone: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
}
