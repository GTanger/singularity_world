#![forbid(unsafe_code)]
//! 將既有 Azgaar 白塔作者包編譯成可追溯的「正式化工作層」。
//!
//! 這個工具不重抽地圖、不改寫作者包，也不把生成名稱或政治升格正典。它只把
//! 計算空間、真正場所、路線、人口權重與仍待審的內容分層，並輸出作者驗收圖。

#[path = "white_tower_formalize/compile.rs"]
mod compile;
#[path = "white_tower_formalize/layers.rs"]
mod layers;
#[path = "white_tower_formalize/svg.rs"]
mod svg;

use std::fs;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let compiled = compile::compile()?;
    compile::validate(&compiled)?;

    let json = serde_json::to_string_pretty(&compiled.document)? + "\n";
    fs::write(compile::OUTPUT_JSON, json)
        .with_context(|| format!("寫入 {} 失敗", compile::OUTPUT_JSON))?;

    let author_svg = svg::render(&compiled);
    fs::write(compile::OUTPUT_SVG, author_svg)
        .with_context(|| format!("寫入 {} 失敗", compile::OUTPUT_SVG))?;

    println!("white_tower_formalize: ok");
    println!(
        "meaningful_places={} settlements={} native_corridors={} markers={} zones={} hard_route_conflicts={}",
        compiled.topology_node_count + compiled.settlements.len(),
        compiled.settlements.len(),
        compiled.routes.len(),
        compiled.marker_count,
        compiled.zone_count,
        compiled.hard_conflicts.len()
    );
    println!("json={}", compile::OUTPUT_JSON);
    println!("svg={}", compile::OUTPUT_SVG);
    Ok(())
}
