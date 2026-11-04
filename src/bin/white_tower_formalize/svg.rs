use std::fmt::Write;

use super::compile::{CompiledFormalization, Point, SettlementRecord};

const BACKGROUND: &str =
    "../../data/world/white_tower_azgaar_v1/white_tower_peninsula_v1_biomes.png";

pub fn render(compiled: &CompiledFormalization) -> String {
    let width = compiled.width;
    let height = compiled.height;
    let mut svg = String::new();
    writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 {width} {height}" width="{width}" height="{height}">"##
    )
    .unwrap();
    svg.push_str(
        r##"<defs><filter id="shadow"><feDropShadow dx="0" dy="1" stdDeviation="1.5" flood-opacity=".35"/></filter></defs>
<rect width="100%" height="100%" fill="#e8e1cf"/>
"##,
    );
    writeln!(
        svg,
        r##"<image href="{BACKGROUND}" xlink:href="{BACKGROUND}" x="0" y="0" width="{width}" height="{height}" opacity="0.86"/>"##
    )
    .unwrap();

    svg.push_str(r##"<g fill="none" stroke-linecap="round" stroke-linejoin="round">"##);
    for route in &compiled.routes {
        let (stroke, opacity, width) = match route.source_group.as_str() {
            "roads" => ("#b44d24", 0.72, 1.7),
            "searoutes" => ("#0f5c86", 0.50, 1.35),
            _ => ("#594d3f", 0.22, 0.75),
        };
        let points = route
            .points
            .iter()
            .map(|point| {
                format!(
                    "{:.2},{:.2}",
                    point.x * compiled.width,
                    point.y * compiled.height
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        writeln!(
            svg,
            r##"<polyline points="{points}" stroke="{stroke}" stroke-width="{width}" opacity="{opacity}" vector-effect="non-scaling-stroke"/>"##
        )
        .unwrap();
    }
    svg.push_str("</g>\n");

    svg.push_str(r##"<g stroke="#fff7e8" stroke-width="7" opacity=".88" fill="none">"##);
    for conflict in &compiled.hard_conflicts {
        line(
            &mut svg,
            &conflict.from,
            &conflict.to,
            compiled.width,
            compiled.height,
        );
    }
    svg.push_str("</g>\n");

    svg.push_str(r##"<g stroke="#7e1111" stroke-width="3.2" stroke-dasharray="7 5" opacity=".98" fill="none">"##);
    for conflict in &compiled.hard_conflicts {
        line(
            &mut svg,
            &conflict.from,
            &conflict.to,
            compiled.width,
            compiled.height,
        );
    }
    svg.push_str("</g>\n");

    svg.push_str(r##"<g stroke="#fff7e8" stroke-width=".7">"##);
    for settlement in &compiled.settlements {
        settlement_dot(&mut svg, settlement, compiled.width, compiled.height);
    }
    svg.push_str("</g>\n");

    svg.push_str(r##"<g font-family="Noto Sans TC, sans-serif" font-size="12" font-weight="700" fill="#15110c">"##);
    for anchor in &compiled.anchors {
        let x = anchor.point.x * compiled.width;
        let y = anchor.point.y * compiled.height;
        writeln!(
            svg,
            r##"<path d="M {x:.1} {y_minus:.1} L {x_plus:.1} {y:.1} L {x:.1} {y_plus:.1} L {x_minus:.1} {y:.1} Z" fill="#fff8d8" stroke="#111" stroke-width="1.8" filter="url(#shadow)"/><text x="{text_x:.1}" y="{text_y:.1}" paint-order="stroke" stroke="#fff8e8" stroke-width="3">{label}</text>"##,
            y_minus = y - 6.0,
            x_plus = x + 6.0,
            y_plus = y + 6.0,
            x_minus = x - 6.0,
            text_x = x + 9.0,
            text_y = y + 4.0,
            label = xml(&anchor.label)
        )
        .unwrap();
    }
    svg.push_str("</g>\n");

    svg.push_str(
        r##"<g font-family="Noto Sans TC, sans-serif">
<rect x="18" y="18" width="510" height="140" rx="10" fill="#fffdf5" opacity=".94" stroke="#292016" stroke-width="1.5" filter="url(#shadow)"/>
<text x="38" y="45" font-size="22" font-weight="800" fill="#20170f">白塔半島正式化 v1｜作者檢查圖</text>
<text x="38" y="67" font-size="13" fill="#46382b">既有 Azgaar 資料：1,666 聚落、1,244 原生路線、13 個已映射接受錨點</text>
<text x="38" y="86" font-size="13" fill="#46382b">底圖與 305 物理區是空間底盤，不是玩家地圖，也不是 305 個內容節點</text>
<circle cx="46" cy="108" r="5" fill="#7b1e1e"/><text x="59" y="113" font-size="12">區域核心種子</text>
<circle cx="170" cy="108" r="4" fill="#d26024"/><text x="181" y="113" font-size="12">主要城市種子</text>
<circle cx="300" cy="108" r="3" fill="#4e3f35"/><text x="310" y="113" font-size="12">城鎮／地方聚落</text>
<line x1="46" y1="136" x2="80" y2="136" stroke="#7e1111" stroke-width="3" stroke-dasharray="7 5"/><text x="90" y="140" font-size="12">大圳頭直通青籠關衝突（8 條，不得升格）</text>
</g>
"##,
    );
    svg.push_str("</svg>\n");
    svg
}

fn settlement_dot(svg: &mut String, settlement: &SettlementRecord, width: f64, height: f64) {
    let x = settlement.coordinates.x * width;
    let y = settlement.coordinates.y * height;
    let (radius, fill) = match settlement.source_group.as_str() {
        "capital" => (5.0, "#7b1e1e"),
        "city" => (3.9, "#d26024"),
        "fort" => (3.6, "#6a3d78"),
        "village" => (1.8, "#807063"),
        _ => (2.5, "#4e3f35"),
    };
    writeln!(
        svg,
        r##"<circle cx="{x:.2}" cy="{y:.2}" r="{radius}" fill="{fill}"><title>{title}</title></circle>"##,
        title = xml(&format!(
            "{}｜{}｜{} 人（工作配置）",
            settlement.source_name, settlement.working_class, settlement.working_population
        ))
    )
    .unwrap();
}

fn line(svg: &mut String, from: &Point, to: &Point, width: f64, height: f64) {
    writeln!(
        svg,
        r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}"/>"##,
        from.x * width,
        from.y * height,
        to.x * width,
        to.y * height
    )
    .unwrap();
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
