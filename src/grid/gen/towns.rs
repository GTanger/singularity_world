/// 建立依地理環境客製化之城鎮格局（10F + 30R = 40 棟建築）
fn build_town_40(
    map: &mut SettlementCellMap,
    cx: i32,
    cy: i32,
    town_name: &str,
    bldg_types: &[(Terrain, &'static str)],
) {
    assert_eq!(
        bldg_types.len(),
        40,
        "Town must have exactly 40 buildings (10F + 30R)"
    );

    // 10 棟核心地標專屬座標（確保緊鄰幹道、水岸或階道）
    let key_slots = match town_name {
        "風舶嶼" => [
            (3, 1),   // 0: 外澳領航公會
            (2, 1),   // 1: 外澳海員旅社
            (3, 2),   // 2: 島嶼修船塢
            (3, -1),  // 3: 外澳水產集散行
            (1, 1),   // 4: 外澳港務脈絡診所
            (2, 0),   // 5: 外澳海島信使中繼站
            (4, 1),   // 6: 外澳石砌碼頭
            (1, 0),   // 7: 風舶嶼公所
            (2, -1),  // 8: 外澳海味食堂
            (1, -1),  // 9: 潮音溫水浴堂
        ],
        "大圳頭" => [
            (1, 0),   // 0: 大圳農牧總社
            (-1, 0),  // 1: 大圳頭車馬聯運站
            (-4, 0),  // 2: 大圳農具與百工坊（緊鄰水車西橋）
            (1, 1),   // 3: 大圳頭脈絡診所
            (0, -1),  // 4: 大圳頭農產行
            (-1, 1),  // 5: 大圳頭商旅社
            (0, -2),  // 6: 大圳頭農產與糧油市集
            (2, 0),   // 7: 田園風味大食堂
            (3, 1),   // 8: 圳水溫和浴堂
            (4, 0),   // 9: 大圳頭・周家莊園（緊鄰東糧倉道）
        ],
        "半嶺鎮" => [
            (1, 0),   // 0: 半嶺試招場
            (-1, 0),  // 1: 半嶺山泉旅社
            (3, 1),   // 2: 寒水玉切石工坊（緊鄰東嶺採石階）
            (0, 1),   // 3: 半嶺脈絡診所
            (0, -1),  // 4: 半嶺山貨特產行
            (1, 1),   // 5: 半嶺鎮公所
            (2, 4),   // 6: 半嶺氣象觀測所（高台階地）
            (-2, 1),  // 7: 青脊石湯溫水浴場
            (-1, -1), // 8: 半嶺山野茶屋食堂
            (-3, 0),  // 9: 半嶺鎮・岑家二房別院
        ],
        _ => [ // 南哨
            (0, 1),   // 0: 南哨防務公議廳
            (1, 1),   // 1: 南哨傳訊信使站
            (-1, 1),  // 2: 重型獵具工坊
            (1, 0),   // 3: 南哨脈絡解毒診所
            (-1, 0),  // 4: 入林嚮導公會
            (0, -2),  // 5: 高棧坪獵人旅社
            (1, -2),  // 6: 黑沼素材收購行
            (-1, -2), // 7: 南哨原野食堂
            (2, -1),  // 8: 高棧坪溫水浴場
            (-2, -1), // 9: 高棧坪・羅家宅院
        ],
    };
    let key_slots_set: HashSet<(i32, i32)> = key_slots.iter().copied().collect();
    let mut inner_slots = Vec::new();

    for dx in -7i32..=7i32 {
        for dy in -7i32..=7i32 {
            let coord = SquareCoord::new(cx + dx, cy + dy);

            // ── 風舶嶼：海島外港與東側石碼頭 ──
            if town_name == "風舶嶼" {
                let is_east_pier = dy == 0 && (4..=7).contains(&dx);
                let is_harbor_water = dx >= 5 && ((1..=3).contains(&dy.abs()));
                let is_quay = dx == 4 && (-4..=4).contains(&dy);
                let is_outer_beach = (dx.abs() + dy.abs() > 10 || dx < -5 || dy.abs() > 5) && !(dy == 0 && (-7..=-5).contains(&dx));
                let is_seawall = (dx.abs() + dy.abs() == 10 || dx == -5 || dy.abs() == 5) && !is_east_pier && !is_harbor_water && !is_quay && !(dy == 0 && (-7..=-5).contains(&dx));

                if is_east_pier {
                    let name = if dx == 7 { "外澳東港門" } else { "外澳長碼頭" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, name));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}連通跨海長橋的深水石砌長碼頭，海風浩蕩。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_harbor_water {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(town_name, "外澳港池"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}平靜的海島泊船水面，停靠往來穿梭的渡海快舟。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_quay {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, "避風港岸石階"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}面朝外澳海灣的避風石階碼頭道，起卸水產物資。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_seawall {
                    let mut cell = GridCell::new(coord, Terrain::Fence, format_settlement_cell_name(town_name, "海防石柵"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}抵擋海浪與狂風的重型海防石木圍柵。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_outer_beach {
                    let mut cell = GridCell::new(coord, Terrain::Beach, format_settlement_cell_name(town_name, "礁岩潮岸"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}外圍經受浪濤沖刷的玄武礁石潮間帶。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }

                let is_main_street = (dy == 0 && dx >= -7 && dx <= 4) || (dx == 0 && dy.abs() <= 4)
                    || (dy == 2 && dx >= -3 && dx <= 3) || (dy == -2 && dx >= -3 && dx <= 3)
                    || (dx == -3 && dy.abs() <= 3);
                if is_main_street {
                    let street_name = if dx == 0 && dy == 0 { "島心石板街" } else if dy == 0 { "外澳海港直街" } else { "外澳石道" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, street_name));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}平整的海島石街，通連各方避風石屋。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if !key_slots_set.contains(&(dx, dy)) {
                    inner_slots.push((dx, dy, coord));
                }
                continue;
            }

            // ── 大圳頭：東西向貫穿大圳主渠與三座跨渠石橋 ──
            if town_name == "大圳頭" {
                let is_canal = dy == 2 && dx.abs() <= 7;
                let is_bridge = dy == 2 && (dx == 0 || dx == -4 || dx == 4);
                let is_bank_street = (dy == 1 || dy == 3) && dx.abs() <= 7;
                let is_cross_street = (dx == 0 && dy.abs() <= 7) || (dx == -4 && dy.abs() <= 6) || (dx == 4 && dy.abs() <= 6)
                    || (dy == -3 && dx.abs() <= 6);
                let is_perimeter = dx.abs() == 7 || dy.abs() == 7;

                if is_bridge {
                    let bridge_name = if dx == 0 { "大圳中橋" } else if dx < 0 { "水車西橋" } else { "水車東橋" };
                    let mut cell = GridCell::new(coord, Terrain::Bridge, format_settlement_cell_name(town_name, bridge_name));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}跨越西南大圳主渠的堅固平整條石橋，橋下活水奔流。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_canal {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(town_name, "西南大圳主渠"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}引古河水脈灌溉八百頃沃野的西南大圳主水渠，清波盪漾。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_perimeter {
                    if (dx == 0 && dy.abs() == 7) || (dy == 1 && dx.abs() == 7) {
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, "鎮口道"));
                        cell.zone = town_name.to_string();
                        cell.description = format!("{}對外的平坦出入口，設有水利管制石碑與路標。", town_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else {
                        let mut cell = GridCell::new(coord, Terrain::Fence, format_settlement_cell_name(town_name, "木柵圍欄"));
                        cell.zone = town_name.to_string();
                        cell.description = format!("{}外圍緊密排設的堅固木柵，防護野獸侵擾。", town_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    }
                    continue;
                }
                if is_bank_street || is_cross_street {
                    let street_name = if dy == 1 { "大圳南岸街" } else if dy == 3 { "大圳北岸街" } else if dx == 0 { "市鎮中心大街" } else if dx < 0 { "西磨坊道" } else if dx > 0 { "東糧倉道" } else { "南農商街" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, street_name));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}繁忙通達的農商大街，水車轉動聲不絕於耳。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if !key_slots_set.contains(&(dx, dy)) {
                    inner_slots.push((dx, dy, coord));
                }
                continue;
            }

            // ── 半嶺鎮：東側青脊絕壁與採石坑、階梯台地街網 ──
            if town_name == "半嶺鎮" {
                let is_cliff = dx >= 6 && dy.abs() <= 7;
                let is_quarry = dx == 6 && (dy == 2 || dy == -2);
                let is_mountain_barrier = (dx == -7 || ((dy == 7 || dy == -7) && dx <= 5)) && !(dx == -7 && dy == 0) && !(dx == 0 && (dy == 7 || dy == -7));
                let is_gate = (dx == -7 && dy == 0) || (dx == 0 && (dy == 7 || dy == -7));

                if is_quarry {
                    let mut cell = GridCell::new(coord, Terrain::MinePass, format_settlement_cell_name(town_name, "寒水玉採石坑"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}東壁開鑿的優質寒水玉石採掘坑階，礦質細膩晶瑩。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_cliff {
                    let mut cell = GridCell::new(coord, Terrain::Mountain, format_settlement_cell_name(town_name, "青脊絕壁"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}東側連綿聳立的青脊山絕壁，岩層裸露，阻擋東方烈風。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_gate {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, "鎮口道"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}開鑿於山隘階地的出入口，青石階道平整。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_mountain_barrier {
                    let mut cell = GridCell::new(coord, Terrain::Fence, format_settlement_cell_name(town_name, "山隘石柵"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}依山勢梯次構建的厚重防獸石柵。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }

                let is_stepped_street = (dx == 0 && dy.abs() < 7) || (dx == -4 && dy.abs() < 7) || (dx == 3 && dy.abs() < 7)
                    || (dy == 0 && dx > -7 && dx <= 5) || (dy == 4 && dx > -7 && dx <= 5) || (dy == -4 && dx > -7 && dx <= 5);
                if is_stepped_street {
                    let street_name = if dx == 0 { "半嶺中心階道" } else if dx < 0 { "西麓石坊道" } else if dx == 3 { "東嶺切石階" } else { "半嶺市階" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, street_name));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}層疊向上的石板階道，山風清爽，視野開闊。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if !key_slots_set.contains(&(dx, dy)) {
                    inner_slots.push((dx, dy, coord));
                }
                continue;
            }

            // ── 南哨：南向收窄的黑沼防禦堡壘與高棧木廊 ──
            let is_south_swamp = dy < 0 && dx.abs() > 5;
            let is_bastion_wall = (dy >= 0 && (dx.abs() == 7 || dy == 7))
                || (dy < 0 && (dx.abs() == 5 || dy == -7));
            let is_south_gate = dy == -7 && dx == 0;
            let is_north_gate = dy == 7 && dx == 0;
            let is_side_gate = dy == 0 && dx.abs() == 7;

            if is_south_swamp {
                let mut cell = GridCell::new(coord, Terrain::Swamp, format_settlement_cell_name(town_name, "棧下泥澤"));
                cell.zone = town_name.to_string();
                cell.description = format!("{}堡壘南方外圍的黑沼泥濘水窪，蘆葦深密，險隘難行。", town_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if is_south_gate || is_north_gate || is_side_gate {
                let name = if is_south_gate { "南哨沼口門" } else { "哨口木道" };
                let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, name));
                cell.zone = town_name.to_string();
                cell.description = format!("{}連通外圍泥澤與林道的高架木排路口，設有哨位瞭望棚。", town_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if is_bastion_wall {
                let mut cell = GridCell::new(coord, Terrain::Fence, format_settlement_cell_name(town_name, "防木重柵"));
                cell.zone = town_name.to_string();
                cell.description = format!("{}以黑沼耐水硬木構建的高大防禦重柵，抵禦瘴獸奔突。", town_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            let is_boardwalk_bridge = dy == -3 && dx.abs() <= 4;
            if is_boardwalk_bridge {
                let mut cell = GridCell::new(coord, Terrain::Bridge, format_settlement_cell_name(town_name, "高棧坪木橋"));
                cell.zone = town_name.to_string();
                cell.description = format!("{}橫跨南方窪澤的高架厚木橋廊，獵人與斥候行走平穩。", town_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            let is_boardwalk_street = (dx == 0 && dy.abs() < 7) || (dy == 0 && dx.abs() < 7)
                || (dy == 3 && dx.abs() < 7) || (dx.abs() == 3 && dy.abs() < 6);
            if is_boardwalk_street {
                let street_name = if dx == 0 && dy < 0 { "高棧木廊大道" } else if dx == 0 { "北防務石街" } else if dy == 0 { "哨鎮東西大街" } else { "棧道巡道" };
                let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, street_name));
                cell.zone = town_name.to_string();
                cell.description = format!("{}架空乾燥的巡查防務木廊，通達堡壘各個哨所與庫房。", town_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if !key_slots_set.contains(&(dx, dy)) {
                inner_slots.push((dx, dy, coord));
            }
        }
    }

    // 先填入 10 棟核心地標建築
    for (idx, &(kdx, kdy)) in key_slots.iter().enumerate() {
        let coord = SquareCoord::new(cx + kdx, cy + kdy);
        let (terrain, bldg_name) = bldg_types[idx];
        let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(town_name, bldg_name));
        cell.zone = town_name.to_string();
        cell.description = generate_settlement_building_desc(town_name, bldg_name, terrain);
        cell.explored = true;
        map.insert(coord, cell);
    }

    // 再填入 30 棟住宅
    inner_slots.sort_by_key(|&(dx, dy, _)| (dx.abs().max(dy.abs()), dx.abs() + dy.abs(), dy, dx));
    for (idx, (dx, dy, coord)) in inner_slots.into_iter().enumerate() {
        if idx < 30 {
            let (terrain, bldg_name) = bldg_types[10 + idx];
            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(town_name, bldg_name));
            cell.zone = town_name.to_string();
            cell.description = generate_settlement_building_desc(town_name, bldg_name, terrain);
            cell.explored = true;
            map.insert(coord, cell);
        } else {
            let slot_idx = (idx + (dx.abs() * 3 + dy.abs() * 5) as usize) % 8;
            let (t, name) = match town_name {
                "大圳頭" => [
                    (Terrain::Road, "曬穀石坪"),
                    (Terrain::FarmField, "阡陌水田"),
                    (Terrain::Road, "農具棚架"),
                    (Terrain::Grassland, "草料堆場"),
                    (Terrain::HerbGarden, "育苗菜畦"),
                    (Terrain::Road, "打穀圓場"),
                    (Terrain::Road, "穀倉前坪"),
                    (Terrain::HerbGarden, "分畦藥圃"),
                ][slot_idx],
                "風舶嶼" => [
                    (Terrain::Road, "曬網石坪"),
                    (Terrain::Road, "礁石望台"),
                    (Terrain::Road, "避風石巷"),
                    (Terrain::Road, "海鹽曬場"),
                    (Terrain::Road, "船帆晾架"),
                    (Terrain::Road, "錨石堆場"),
                    (Terrain::HerbGarden, "耐風草圃"),
                    (Terrain::Road, "燈號石坪"),
                ][slot_idx],
                "半嶺鎮" => [
                    (Terrain::Road, "切石石坪"),
                    (Terrain::Road, "山泉清槽"),
                    (Terrain::Road, "柴薪儲場"),
                    (Terrain::HerbGarden, "高山藥畦"),
                    (Terrain::Road, "觀雲石台"),
                    (Terrain::MinePass, "採石坡道"),
                    (Terrain::Road, "試招石坪"),
                    (Terrain::Road, "晾石前坪"),
                ][slot_idx],
                _ => [
                    (Terrain::Road, "高架木坪"),
                    (Terrain::HerbGarden, "曬藥竹場"),
                    (Terrain::Road, "獵具檢修坪"),
                    (Terrain::Road, "篝火石坪"),
                    (Terrain::HerbGarden, "防瘴艾圃"),
                    (Terrain::Road, "棧道木坪"),
                    (Terrain::Road, "巡哨前坪"),
                    (Terrain::Road, "獸皮晾架"),
                ][slot_idx],
            };
            let mut cell = GridCell::new(coord, t, format_settlement_cell_name(town_name, name));
            cell.zone = town_name.to_string();
            cell.description = format!("{}鋪展整潔的{}，市鎮坊民日常勞作休歇於此。", town_name, name);
            cell.explored = true;
            map.insert(coord, cell);
        }
    }
}

