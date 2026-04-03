use gloo_net::http::Request;
use serde::Serialize;

use crate::types::*;

const MG_KEY: &str = "";

fn api_url(path: &str) -> String {
    format!("{path}?mg_key={MG_KEY}")
}

pub async fn load_grid() -> Result<GridResponse, String> {
    let resp = Request::get(&api_url("/api/hex/grid"))
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
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
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

pub async fn delete_cell(q: i32, r: i32) -> Result<(), String> {
    let resp = Request::delete(&api_url(&format!("/api/hex/cell/{q}/{r}")))
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
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
        return Err(format!("HTTP {}", resp.status()));
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
        return Err(format!("HTTP {}", resp.status()));
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
        return Err(format!("HTTP {}", resp.status()));
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
        return Err(format!("HTTP {}", resp.status()));
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
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

pub async fn reload_grid() -> Result<usize, String> {
    let resp = Request::post(&api_url("/api/hex/reload"))
        .send()
        .await
        .map_err(|e| format!("fetch 失敗：{e}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
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
