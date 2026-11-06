fn build_city_feng_bo_jiao_55(
    map: &mut SettlementCellMap,
    cx: i32,
    cy: i32,
    city_name: &str,
    bldg_types: &[(Terrain, &'static str)],
) {
        let mut inner_slots = Vec::new();

        // 預留西港 15 棟核心海事地標的專屬臨水與幹道座標（確保全部緊鄰碼頭、長堤與主街）
        let key_slots = [
            (-1, 0),   // 0: 風舶海事商盟總會
            (-1, 1),   // 1: 海事總商行與船貨契據所
            (-1, -1),  // 2: 風舶航海圖誌館
            (-3, -4),  // 3: 林氏大船塢
            (-3, -3),  // 4: 馭風帆具與船材工坊
            (-3, -2),  // 5: 港務脈絡診所
            (-3, 3),   // 6: 海風海鮮食堂
            (-3, 2),   // 7: 港灣溫水浴場
            (-3, 1),   // 8: 海員旅社
            (4, 0),    // 9: 港尾車馬聯運站
            (3, 0),    // 10: 海陸聯運貨運行
            (-3, 0),   // 11: 深水泊位與卸貨碼頭
            (-3, 4),   // 12: 港區魚獲水產交易行
            (-3, 5),   // 13: 築港頭・許家宅邸
            (-3, -5),  // 14: 大船塢・林家宅院
        ];
        let key_slots_set: HashSet<(i32, i32)> = key_slots.iter().copied().collect();

        for dx in -10i32..=10i32 {
            for dy in -10i32..=10i32 {
                let coord = SquareCoord::new(cx + dx, cy + dy);

                // 1. 西側外海敞開港灣水體與三座向西突出的長石堤（徹底廢除西側實心死牆）
                let is_center_pier = dy == 0 && (-10..=-4).contains(&dx);
                let is_north_pier = dy == 4 && (-10..=-4).contains(&dx);
                let is_south_pier = dy == -4 && (-10..=-4).contains(&dx);
                let is_quay = dx == -4 && (-7..=7).contains(&dy);
                let is_west_basin = (-10..=-5).contains(&dx)
                    && ((1..=3).contains(&dy) || (5..=7).contains(&dy) || (-3..=-1).contains(&dy) || (-7..=-5).contains(&dy));

                if is_center_pier {
                    let name = if dx == -10 { "西海跨海橋引道" } else { "西港深水引堤道" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, name));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}向西延伸入港灣的寬闊條石長堤，連通跨海長橋與泊船水域。", city_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_north_pier {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "北港長石堤"));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}北港突出的堅固防浪石堤，外側浪花拍擊，內側泊滿商貨帆船。", city_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_south_pier {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "南港長石堤"));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}南港延伸入海的石條長堤，直抵大船塢外泊位。", city_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_quay {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "港前臨海石道"));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}面朝內澳深水港池的潮潤石階碼頭大道，水手挑夫往來穿梭。", city_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_west_basin {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(city_name, "內澳深水港池"));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}三面受長石堤庇護的天然深水泊船港灣，碧波蕩漾，停靠大噸位海船。", city_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }

                // 2. 陸側外圍城牆（僅包覆北、東、南陸側，西側完全向港灣開放）
                let is_landward_wall = (dy == 10 && dx >= -7) || (dy == -10 && dx >= -7) || (dx == 10)
                    || (dx <= -8 && (dy == 10 || dy == -10));
                if is_landward_wall {
                    if (dx == 0 && (dy == 10 || dy == -10)) || (dx == 10 && dy == 0) {
                        let gate_dir = if dy == 10 {
                            "北城門"
                        } else if dy == -10 {
                            "南城門"
                        } else {
                            "東城門"
                        };
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, gate_dir));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}厚重雄偉之{}，守衛森嚴，商流行人穿梭不絕。", city_name, gate_dir);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else {
                        let mut cell = GridCell::new(coord, Terrain::Wall, format_settlement_cell_name(city_name, "實心城牆"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}高大堅固之條石城牆，上覆防禦機關與哨堞，固若金湯。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    }
                    continue;
                }

                // 3. 市區幹道網絡
                let is_cross_east = dy == 0 && dx > -4 && dx < 10;
                let is_main_north_south = dx == 0 && dy.abs() < 10;
                let is_east_avenue = dx == 5 && dy.abs() < 10;
                let is_north_street = dy == 5 && dx > -4 && dx < 10;
                let is_south_street = dy == -5 && dx > -4 && dx < 10;
                let is_dock_inner = dx == -2 && (-7..=7).contains(&dy);
                let is_mid_lane = dx == 2 && dy.abs() < 9;

                if is_cross_east || is_main_north_south || is_east_avenue || is_north_street || is_south_street || is_dock_inner || is_mid_lane {
                    let (terrain, street_name, desc_suffix) = if dx == 0 && dy == 0 {
                        (Terrain::Bridge, "海事中心樞紐橋", "貫穿港城核心的寬闊石橋，橋下海浪輕湧，四方船貨與商旅匯聚於此。")
                    } else if is_dock_inner {
                        (Terrain::Road, "內港泊位街", "緊鄰西側深水泊位的繁華石道，船匠與水手在此歇腳會商。")
                    } else if is_cross_east {
                        (Terrain::Road, "海陸聯運通街", "連接車馬總站的開闊石道，雙輪大車往來絡繹不絕，裝卸行裝井然有序。")
                    } else if is_main_north_south {
                        (Terrain::Road, "港城中心大街", "貫穿港城南北的大道，兩側商行林立，海風清爽。")
                    } else if is_east_avenue {
                        (Terrain::Road, "東聯運大街", "通往東城門的繁忙運輸通道，運載來自內陸的百年雲杉與生活物資。")
                    } else if is_north_street {
                        (Terrain::Road, "北港商街", "築港頭商貿繁忙的青石長街，商行林立，空氣中飄散著桐油與海風氣息。")
                    } else if is_south_street {
                        (Terrain::Road, "南船塢大街", "鄰近造船巨木區的厚重石街，大車運載百年雲杉與深海巨藻纜繩，熱氣騰騰。")
                    } else {
                        (Terrain::Road, "港中市街", "繁華緊湊的港埠街巷，店面與民居櫛比鱗次，商客與水手往來穿梭。")
                    };
                    let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, street_name));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}，{}", city_name, desc_suffix);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }

                // 4. 建築與庭院候選格子
                if !key_slots_set.contains(&(dx, dy)) {
                    inner_slots.push((dx, dy, coord));
                }
            }
        }

        // 先填入 15 棟核心海事地標
        for (idx, &(kdx, kdy)) in key_slots.iter().enumerate() {
            let coord = SquareCoord::new(cx + kdx, cy + kdy);
            let (terrain, bldg_name) = bldg_types[idx];
            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, bldg_name));
            cell.zone = city_name.to_string();
            cell.description = generate_settlement_building_desc(city_name, bldg_name, terrain);
            cell.explored = true;
            map.insert(coord, cell);
        }

        // 再填入 40 棟港埠與水手住宅
        inner_slots.sort_by_key(|&(dx, dy, _)| (dx.abs().max(dy.abs()), dx.abs() + dy.abs(), dy, dx));
        for (idx, (dx, dy, coord)) in inner_slots.into_iter().enumerate() {
            if idx < 40 {
                let (terrain, bldg_name) = bldg_types[15 + idx];
                let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, bldg_name));
                cell.zone = city_name.to_string();
                cell.description = generate_settlement_building_desc(city_name, bldg_name, terrain);
                cell.explored = true;
                map.insert(coord, cell);
            } else {
                let (terrain, place_label, desc_suffix) = match (idx + (dx.abs() * 3 + dy.abs() * 5) as usize) % 12 {
                    0 => (Terrain::Road, "曬網石坪", "風舶角臨海鋪設的寬闊青石曬網坪，水手與漁婦在此攤開巨型深海漁網曝曬修補。"),
                    1 => (Terrain::Bridge, "引潮石橋", "風舶角跨越引海水通潮石渠的堅固平橋，潮水隨海風漲落拍岸。"),
                    2 => (Terrain::HerbGarden, "避風花圃", "風舶角以厚石矮牆圍護的避風花圃，栽種耐鹽海桐與防風香草。"),
                    3 => (Terrain::Road, "錨鏈儲場", "風舶角碼頭後側整齊盤放粗重烏石錨與鐵索的石板儲場。"),
                    4 => (Terrain::Road, "船木晾場", "風舶角大船塢旁架高通風的百年雲杉與樟木船材自然陰乾場。"),
                    5 => (Terrain::Road, "貝灰曬坪", "風舶角鋪滿雪白海貝殼與防水灰漿原料的平整石坪。"),
                    6 => (Terrain::Road, "纜繩工棚", "風舶角絞製深海巨藻韌纜與粗麻船索的長條石柱敞棚。"),
                    7 => (Terrain::Road, "望海石欄", "風舶角面朝外澳海灣的青石護欄平台，可遠眺跨海長橋與風舶嶼帆影。"),
                    8 => (Terrain::Road, "卸貨石坪", "風舶角供雙輪貨車與碼頭挑夫交接海產乾貨箱籠的厚石空坪。"),
                    9 => (Terrain::HerbGarden, "海風茶亭", "風舶角街區轉角供船員歇腳飲茶、聽取潮汐鐘聲的石砌涼亭。"),
                    10 => (Terrain::Bridge, "活水渠橋", "風舶角引流山泉活水供港區洗滌淡化海鹽的清澈石渠小橋。"),
                    _ => (Terrain::Road, "街坊石坪", "風舶角兩側防浪石屋圍合的街坊中庭石坪，海風清爽。"),
                };
                let desc = format!("{}，{}", city_name, desc_suffix);
                let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, place_label));
                cell.zone = city_name.to_string();
                cell.description = desc;
                cell.explored = true;
                map.insert(coord, cell);
            }
        }
    }


fn build_city_55(
    map: &mut SettlementCellMap,
    cx: i32,
    cy: i32,
    city_name: &str,
    bldg_types: &[(Terrain, &'static str)],
) {
    assert_eq!(
        bldg_types.len(),
        55,
        "City must have exactly 55 buildings"
    );

    if city_name == "文曜" {
        let mut inner_slots = Vec::new();
        let mut outer_slots = Vec::new();
        build_wen_yao_outer_perimeter(map, cx, cy, city_name, &mut outer_slots);
        build_wen_yao_inner_core(map, cx, cy, city_name, &mut inner_slots);
        populate_wen_yao_slots(map, city_name, bldg_types, inner_slots, outer_slots);
    } else {
        build_city_feng_bo_jiao_55(map, cx, cy, city_name, bldg_types);
    }
}
