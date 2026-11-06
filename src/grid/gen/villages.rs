/// 建立依地理環境客製化之村落格局（5F + 10R = 15 棟建築）
fn build_village_15(
    map: &mut SettlementCellMap,
    cx: i32,
    cy: i32,
    vil_name: &str,
    bldg_types: &[(Terrain, &'static str)],
) {
    assert_eq!(
        bldg_types.len(),
        15,
        "Village must have exactly 15 buildings (5F + 10R)"
    );

    // 15 棟建築之專屬幾何座標配置（確保 100% 緊鄰可通行道路或木橋）
    let vil_slots = match vil_name {
        "老堰頭" => [
            (1, 0),   // 0: 老堰頭農產公議所 (鄰 0, 0 堰東巡水街)
            (1, 1),   // 1: 老堰頭脈絡草木診所 (鄰 0, 1)
            (-3, 0),  // 2: 水輪石磨工坊 (鄰 -2, 0 陸家舊堰石橋)
            (1, -1),  // 3: 堰渠水畔食堂 (鄰 0, -1)
            (-3, 1),  // 4: 水門茶憩旅社 (鄰 -2, 1 堰西水磨道)
            (-3, -1), // 5: 水畔碾米磨藥工匠居 (鄰 -2, -1)
            (-3, 2),  // 6: 老堰頭水門守閘水工舍 (鄰 -2, 2)
            (1, 2),   // 7: 陸氏宗族老宅 (鄰 0, 2)
            (-3, -2), // 8: 老水車維修木匠居 (鄰 -2, -2)
            (1, -2),  // 9: 古渠引水石匠居 (鄰 0, -2)
            (-3, 3),  // 10: 堰西磨坊長工宿所 (鄰 -2, 3)
            (1, 3),   // 11: 水磨麵粉倉儲家屋 (鄰 0, 3)
            (1, -3),  // 12: 晨霧茶農小舍 (鄰 0, -3)
            (1, 4),   // 13: 青苔石堰巡查居 (鄰 0, 4)
            (-3, -3), // 14: 老堰頭渡水擺渡人舍 (鄰 -2, -3)
        ],
        "北嶺腳" => [
            (1, 0),   // 0: 北嶺腳山林旅社 (鄰 0, 0 嶺腳主徑)
            (1, 1),   // 1: 北嶺腳脈絡急救診所 (鄰 0, 1)
            (1, -1),  // 2: 北嶺山徑補給行 (鄰 0, -1)
            (-1, 0),  // 3: 嶺腳暖火食堂 (鄰 0, 0)
            (-1, -1), // 4: 北嶺腳公事房 (鄰 0, -1)
            (-1, 1),  // 5: 馬家鐵齒登山釘鞋工坊宅 (鄰 0, 1)
            (1, 2),   // 6: 硬木藤背架老匠居 (鄰 0, 2)
            (-1, 2),  // 7: 松脂暖手爐銅匠舍 (鄰 0, 2)
            (1, -2),  // 8: 高嶺採藥嚮導居 (鄰 0, -2)
            (-1, -2), // 9: 防風厚毛氈編織屋 (鄰 0, -2)
            (1, 3),   // 10: 雪嶺山道挑夫客舍 (鄰 0, 3)
            (-1, 3),  // 11: 獵犬飼育訓獸舍 (鄰 0, 3)
            (1, -3),  // 12: 石坎梯田農家小院 (鄰 0, -3)
            (-1, -3), // 13: 寒林乾燥木料小舍 (鄰 0, -3)
            (-1, 4),  // 14: 背風向陽石板居 (鄰 0, 4)
        ],
        "霜林口" => [
            (1, 0),   // 0: 霜林護林瞭望所 (鄰 0, 0 霜林中道)
            (1, 1),   // 1: 霜林脈絡診所 (鄰 0, 1)
            (-1, 0),  // 2: 原木水車鋸木坊 (鄰 0, 0)
            (1, -1),  // 3: 高山木料調配站 (鄰 0, -1)
            (-1, -1), // 4: 林緣溫暖旅社 (鄰 0, -1)
            (1, 2),   // 5: 老鋸木師傅長居宅 (鄰 0, 2)
            (-1, 2),  // 6: 大口徑圓鋸保養工坊宅 (鄰 0, 2)
            (1, 3),   // 7: 重型拖木巨索編造居 (鄰 0, 3)
            (-1, 3),  // 8: 雲杉油防腐浸料工寮 (鄰 0, 3)
            (1, 4),   // 9: 山間運木雪橇修理家 (鄰 0, 4)
            (-1, 4),  // 10: 巡山伐木領班木樓 (鄰 0, 4)
            (1, 5),   // 11: 鋸木學徒集體宿所 (鄰 0, 5)
            (-1, 5),  // 12: 松木防潮木屋一號 (鄰 0, 5)
            (1, -3),  // 13: 松木防潮木屋二號 (鄰 0, -3)
            (-1, -3), // 14: 林口柴炭自給居 (鄰 0, -3)
        ],
        "風櫃尾" => [
            (-3, 0),  // 0: 風櫃尾燈塔 (突出西端海岬高地，鄰 -2, 0 觀海橫街)
            (1, 1),   // 1: 風櫃尾脈絡草藥診所 (鄰 0, 1 避風街)
            (-1, 1),  // 2: 水產冷燻工坊 (鄰 0, 1)
            (1, 2),   // 3: 風櫃尾海風食堂 (鄰 0, 2)
            (-1, 2),  // 4: 風櫃尾巨藻晾曬場 (鄰 0, 2)
            (1, -1),  // 5: 看守燈塔長工宿所 (鄰 0, -1)
            (-1, -1), // 6: 防海霧鹽蝕厚石大宅 (鄰 0, -1)
            (1, -2),  // 7: 礁岩拾貝採螺老嫗居 (鄰 0, -2)
            (-1, -2), // 8: 深海魚油熬煉作坊宅 (鄰 0, -2)
            (1, 3),   // 9: 浪濤回音聽潮小築 (鄰 0, 3)
            (-1, 3),  // 10: 海風吹拂堅石居一號 (鄰 0, 3)
            (1, 4),   // 11: 海風吹拂堅石居二號 (鄰 0, 4)
            (-1, 4),  // 12: 海風吹拂堅石居三號 (鄰 0, 4)
            (1, -3),  // 13: 海風吹拂堅石居四號 (鄰 0, -3)
            (-1, -3), // 14: 風櫃尾避潮土石居 (鄰 0, -3)
        ],
        "烏石鼻" => [
            (-3, 0),  // 0: 烏石鼻漁船泊位 (黑石漁灣畔石階泊位，鄰 -2, 0 漁港碼頭道)
            (1, 1),   // 1: 烏石鼻脈絡草木診所 (鄰 0, 1 烏石街)
            (-1, 1),  // 2: 玄武岩鑿石工坊 (鄰 0, 1)
            (1, 2),   // 3: 烏石水產集散行 (鄰 0, 2)
            (-1, 2),  // 4: 烏石鼻海貝灰窯 (鄰 0, 2)
            (1, -1),  // 5: 烏石鼻採石工領班宅 (鄰 0, -1)
            (-1, -1), // 6: 打石鐵錘鍛造老鐵匠居 (鄰 0, -1)
            (1, -2),  // 7: 黑石漁船泊位值班房 (鄰 0, -2)
            (-1, -2), // 8: 耐鹽海草灰熬煉作坊居 (鄰 0, -2)
            (1, 3),   // 9: 玄武岩條石雕鑿學徒舍 (鄰 0, 3)
            (-1, 3),  // 10: 黑石防浪厚牆居一號 (鄰 0, 3)
            (1, 4),   // 11: 黑石防浪厚牆居二號 (鄰 0, 4)
            (-1, 4),  // 12: 黑石防浪厚牆居三號 (鄰 0, 4)
            (1, -3),  // 13: 黑石防浪厚牆居四號 (鄰 0, -3)
            (-1, -3), // 14: 烏石鼻依山向海居 (鄰 0, -3)
        ],
        "牛埔" => [
            (3, 0),   // 0: 牛埔牧業公議所 (鄰 2, 0 環牧石板道)
            (3, 1),   // 1: 草甸脈絡調理診所 (鄰 2, 1)
            (-3, 1),  // 2: 羊毛與乳酪精製坊 (鄰 -2, 1)
            (3, -1),  // 3: 牛埔牧畜交易市集 (鄰 2, -1)
            (-3, -1), // 4: 牧野歇腳旅社 (鄰 -2, -1)
            (3, 2),   // 5: 老牧牛人家族大宅 (鄰 2, 2)
            (-3, 0),  // 6: 乳酪木桶箍桶匠居 (鄰 -2, 0)
            (-3, 2),  // 7: 原野剪羊毛大快剪老手舍 (鄰 -2, 2)
            (3, -2),  // 8: 草料青貯防潮圓倉家屋 (鄰 2, -2)
            (-3, -2), // 9: 牛犢接生調養獸醫居 (鄰 -2, -2)
            (0, 3),   // 10: 長風草甸放牧巡哨木屋 (鄰 0, 2)
            (1, 3),   // 11: 苜蓿乾草堆疊工棚舍 (鄰 1, 2)
            (-1, 3),  // 12: 草甸牧家夯土石屋一號 (鄰 -1, 2)
            (0, -3),  // 13: 草甸牧家夯土石屋二號 (鄰 0, -2)
            (-1, -3), // 14: 牛埔晨露向陽安居宅 (鄰 -1, -2)
        ],
        "紅土崁" => [
            (-1, 1),  // 0: 紅土崁選礦場 (鄰 -2, 1 西窯道)
            (1, 1),   // 1: 紅土崁脈絡復原診所 (鄰 2, 1 東選礦道)
            (-1, 2),  // 2: 紅土百工鐵器坊 (鄰 -2, 2)
            (1, 2),   // 3: 鐵礦原料調撥處 (鄰 2, 2)
            (-1, -1), // 4: 紅土崁・方家宅院 (鄰 -2, -1)
            (1, -1),  // 5: 紅土窯老窯頭主宅 (鄰 2, -1)
            (-3, 0),  // 6: 優質耐火紅磚成型作坊 (鄰 -2, 0)
            (-1, 3),  // 7: 選礦跳汰木槽工匠居 (鄰 -2, 3)
            (1, 3),   // 8: 粗選赤鐵礦石過篩工舍 (鄰 2, 3)
            (-1, -2), // 9: 燒窯柴炭堆疊防潮倉宅 (鄰 -2, -2)
            (1, -2),  // 10: 紅土高溫磚窯修護工屋 (鄰 2, -2)
            (-1, 4),  // 11: 礦砂運輸雙輪木車修理居 (鄰 -2, 4)
            (1, 4),   // 12: 紅土崁耐火泥工小舍 (鄰 2, 4)
            (-1, -3), // 13: 階地梯級礦工連排石屋一號 (鄰 -2, -3)
            (1, -3),  // 14: 階地梯級礦工連排石屋二號 (鄰 2, -3)
        ],
        "三號坑口" => [
            (-5, 0),  // 0: 三號坑口礦務所 (主坑口，鄰 -4, 0 礦車鐵軌道)
            (1, 1),   // 1: 三號坑口脈絡急救診所 (鄰 0, 1 坑口中街)
            (-1, 1),  // 2: 坑口台車維修棚 (鄰 0, 1)
            (1, 2),   // 3: 礦山生活物資行 (鄰 0, 2)
            (-1, 2),  // 4: 坑口礦工旅社 (鄰 0, 2)
            (1, -1),  // 5: 深坑通風木管與風箱匠居 (鄰 0, -1)
            (-1, -1), // 6: 坑道烏木支架大木工宅 (鄰 0, -1)
            (1, -2),  // 7: 礦坑提昇絞盤機索工舍 (鄰 0, -2)
            (-1, -2), // 8: 採礦鐵鎬淬火鍛造坊舍 (鄰 0, -2)
            (1, 3),   // 9: 礦工油脂探照燈配發房宅 (鄰 0, 3)
            (-1, 3),  // 10: 三號主坑口老礦頭值班居 (鄰 0, 3)
            (1, 4),   // 11: 礦石品位檢驗帳房宅 (鄰 0, 4)
            (-1, 4),  // 12: 坑道湧水排吸唧筒水工舍 (鄰 0, 4)
            (1, -3),  // 13: 厚實礦岩避震石屋一號 (鄰 0, -3)
            (-1, -3), // 14: 厚實礦岩避震石屋二號 (鄰 0, -3)
        ],
        _ => [ // 水尾寮
            (2, 0),   // 0: 水尾寮平底竹筏埠 (水生竹筏水道邊碼頭，鄰 0, 0 木棧道 與 0, -2 棧橋)
            (1, 1),   // 1: 濕地脈絡防瘴診所 (鄰 0, 1 高腳烏木棧橋)
            (-1, 1),  // 2: 蘆葦與竹編工棚 (鄰 0, 1)
            (1, 2),   // 3: 黑沼生鮮水產行 (鄰 0, 2)
            (-1, 2),  // 4: 水尾浮木旅社 (鄰 0, 2)
            (1, -2),  // 5: 阮家高腳木屋與平底竹筏匠老宅 (鄰 0, -2)
            (-1, -2), // 6: 水尾寮沼生烏木打樁水工舍 (鄰 0, -2)
            (1, 3),   // 7: 水尾寮架高避水木屋一號 (鄰 0, 3)
            (-1, 3),  // 8: 水尾寮架高避水木屋二號 (鄰 0, 3)
            (1, -3),  // 9: 黑沼泥炭提煉乾燥小築 (鄰 0, -3)
            (-1, -3), // 10: 水生菱角採摘漁娘居 (鄰 0, -3)
            (1, 4),   // 11: 耐水防腐桐油浸泡工棚居 (鄰 0, 4)
            (-1, 4),  // 12: 深水立網防鱷護寮人舍 (鄰 0, 4)
            (1, 5),   // 13: 水尾寮避水高樁木舍一號 (鄰 0, 5)
            (-1, 5),  // 14: 水尾寮避水高樁木舍二號 (鄰 0, 5)
        ],
    };
    let vil_slots_set: HashSet<(i32, i32)> = vil_slots.iter().copied().collect();

    for dx in -6i32..=6i32 {
        for dy in -6i32..=6i32 {
            let coord = SquareCoord::new(cx + dx, cy + dy);

            // ── 1. 老堰頭：南北貫穿清泉水脈與陸家舊堰石橋 ──
            if vil_name == "老堰頭" {
                let is_bridge = dy == 0 && (-2..=0).contains(&dx);
                let is_river = dx == -1 && dy.abs() <= 6;
                let is_east_bank = dx == 0 && dy.abs() <= 6;
                let is_west_bank = dx == -2 && dy.abs() <= 5;
                let is_cross_lane = dy == 0 && (dx.abs() <= 6);

                if is_bridge {
                    let mut cell = GridCell::new(coord, Terrain::Bridge, format_settlement_cell_name(vil_name, "陸家舊堰石橋"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}橫跨清泉水脈的厚實舊堰條石拱橋，溪水奔流拍擊古堰石基。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_river {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(vil_name, "清泉水脈"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}自白塔雪巔奔流而下的清澈水脈，帶動溪畔連綿水車碾磨。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_east_bank || is_west_bank || is_cross_lane {
                    let name = if is_east_bank { "堰東巡水街" } else if is_west_bank { "堰西水磨道" } else { "老堰石渠路" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}沿著水渠鋪展的清爽石板步道，活水淙淙，寧靜祥和。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                // 周邊田地
                let (t, name) = if (-5..=-3).contains(&dx) && (2..=5).contains(&dy) {
                    (Terrain::FarmField, "阡陌水田")
                } else if (3..=5).contains(&dx) && (-5..=-2).contains(&dy) {
                    (Terrain::FarmField, "阡陌水田")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 3 == 0 {
                    (Terrain::HerbGarden, "分畦藥圃")
                } else {
                    (Terrain::DryField, "平整旱田")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 2. 北嶺腳：向陽山谷階梯碎石路與防風林（乾谷地形） ──
            if vil_name == "北嶺腳" {
                let is_mountain_slope = dx >= 4 && dy >= -2;
                let is_main_path = (dx == 0 && dy.abs() <= 6) || (dy == 0 && dx >= -6 && dx <= 3);
                let is_branch_path = dx == -2 && dy.abs() <= 3;

                if is_mountain_slope {
                    let t = if dx == 6 || dy == 6 { Terrain::Mountain } else { Terrain::MinePass };
                    let name = if t == Terrain::Mountain { "嶺上陡崖" } else { "坡地碎石" };
                    let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}東北側聳立的青脊山高陡岩壁，阻擋來自雪巔的高山寒風。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_main_path || is_branch_path {
                    let name = if dx == 0 && dy == 0 { "山村石徑" } else if dx == 0 { "嶺腳主徑" } else if dy == 0 { "橫嶺山徑" } else { "杉木西巷" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}依山谷平整鋪設的乾燥碎石山徑，空氣清冽乾燥。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (-5..=-3).contains(&dx) && (2..=5).contains(&dy) {
                    (Terrain::Forest, "雲杉苗圃")
                } else if (-5..=-3).contains(&dx) && (-5..=-2).contains(&dy) {
                    (Terrain::HerbGarden, "山草藥畦")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 2 == 0 {
                    (Terrain::Road, "儲木石坪")
                } else {
                    (Terrain::Forest, "護林松坡")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 3. 霜林口：東西橫貫融雪急流與鋸木水車木橋 ──
            if vil_name == "霜林口" {
                let is_creek_bridge = dy == -2 && dx == 0;
                let is_snowmelt_creek = dy == -2 && dx.abs() <= 6;
                let is_streets = (dx == 0 && dy.abs() <= 6) || (dy == -1 && dx.abs() <= 6) || (dy == 2 && dx.abs() <= 5);

                if is_creek_bridge {
                    let mut cell = GridCell::new(coord, Terrain::Bridge, format_settlement_cell_name(vil_name, "鋸木水車橋"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}跨越融雪急流的厚重硬木橋，橋下冰涼白浪奔騰激盪。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_snowmelt_creek {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(vil_name, "融雪急流"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}奔流而過的清澈高山融雪急流，推動巨型雲杉水車旋轉帶鋸。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_streets {
                    let name = if dx == 0 && dy == 0 { "山村石徑" } else if dx == 0 { "霜林中道" } else if dy == -1 { "鋸木溪前街" } else { "杉林北道" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}鋪滿清香松針與堅實碎石的山村道路，伐木大車往來其上。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (2..=5).contains(&dx) && (-5..=-3).contains(&dy) {
                    (Terrain::Road, "儲木石坪")
                } else if (-4..=-2).contains(&dx) && (2..=5).contains(&dy) {
                    (Terrain::Forest, "雲杉林圃")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 2 == 0 {
                    (Terrain::HerbGarden, "山草藥畦")
                } else {
                    (Terrain::Forest, "護林松坡")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 4. 風櫃尾：突出西海之海岬崖壁、燈塔與避風街 ──
            if vil_name == "風櫃尾" {
                let is_sea_cliff = dx <= -4 && dy.abs() <= 6;
                let is_promontory = dx == -3 && dy == 0;
                let is_streets = (dx == 0 && dy.abs() <= 6) || (dy == 0 && dx >= -2 && dx <= 6) || (dx == 2 && dy.abs() <= 4);

                if is_sea_cliff {
                    let (t, name) = if dx == -6 { (Terrain::Water, "海浪拍岸") } else if dx == -5 { (Terrain::Beach, "風化礁岩") } else { (Terrain::Mountain, "海角岩壁") };
                    let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}西側直面海潮沖刷的陡峭海岬崖岸，風浪喧囂。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_promontory {
                    // 燈塔保留在 vil_slots
                } else if is_streets {
                    let name = if dx == 0 && dy == 0 { "漁村石階道" } else if dx == 0 { "避風街" } else if dy == 0 { "觀海橫街" } else { "漁港東巷" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}傍海鋪設的防潮玄武岩石板路，海風吹拂，空氣中帶著鹽味。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (dx.abs() * 3 + dy.abs() * 7) % 3 == 0 {
                    (Terrain::Beach, "礁石淺灘")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 3 == 1 {
                    (Terrain::Road, "曬網石坪")
                } else {
                    (Terrain::Beach, "巨藻晾場")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 5. 烏石鼻：西南黑石漁灣與東側玄武採石場 ──
            if vil_name == "烏石鼻" {
                let is_cove_water = dx <= -4 && (-5..=1).contains(&dy);
                let is_pier_quay = dx == -2 && dy == 0;
                let is_quarry = dx >= 4 && dy.abs() <= 5;
                let is_streets = (dx == 0 && dy.abs() <= 6) || (dy == 0 && dx >= -2 && dx <= 3);

                if is_cove_water {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(vil_name, "黑石漁灣"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}西南避風的黑石平靜漁灣，天然深水，停靠近海採貝漁舟。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_pier_quay {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, "漁港碼頭道"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}連通烏石鼻漁船泊位的堅實石階起卸道。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_quarry {
                    let mut cell = GridCell::new(coord, Terrain::MinePass, format_settlement_cell_name(vil_name, "玄武採石場"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}東坡層次分明之深黑玄武岩採石階地，石質密實堅固。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_streets {
                    let name = if dx == 0 && dy == 0 { "漁村石階道" } else if dx == 0 { "烏石街" } else { "漁村橫道" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}厚重玄武石板鋪展的漁村大道，石匠與水手在此歇腳。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (dx.abs() * 3 + dy.abs() * 7) % 2 == 0 {
                    (Terrain::Road, "曬網石坪")
                } else {
                    (Terrain::Beach, "礁石淺灘")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 6. 牛埔：中央環形牧場石道與開闊草坪 ──
            if vil_name == "牛埔" {
                let is_center_hub = dx == 0 && dy == 0;
                let is_center_meadow = dx.abs() <= 1 && dy.abs() <= 1 && !is_center_hub;
                let is_ring_road = (dx.abs() == 2 && dy.abs() <= 2) || (dy.abs() == 2 && dx.abs() <= 2);
                let is_radial_road = (dx == 0 && dy.abs() >= 3) || (dy == 0 && dx.abs() >= 3);

                if is_center_hub {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, "牧坪中道"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}核心中央牧坪交會的平整石板中道，牧人騎馬巡行。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_center_meadow {
                    let mut cell = GridCell::new(coord, Terrain::Grassland, format_settlement_cell_name(vil_name, "中央牧坪"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}中心芳草如茵的柔軟草坪，牛群在微風中悠閒啃草。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_ring_road || is_radial_road {
                    let name = if is_ring_road { "環牧石板道" } else { "牧村出入口道" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}環繞牧場的硬實石板路，大車運載草料奶酪平穩順暢。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (dx.abs() * 3 + dy.abs() * 7) % 3 == 0 {
                    (Terrain::Road, "羊毛曬場")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 3 == 1 {
                    (Terrain::Grassland, "向陽牧坪")
                } else {
                    (Terrain::Grassland, "苜蓿草場")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 7. 紅土崁：階梯式紅土紅磚窯場與雙向選礦道 ──
            if vil_name == "紅土崁" {
                let is_streets = (dx == -2 && dy.abs() <= 6) || (dx == 2 && dy.abs() <= 6) || (dy == 0 && dx.abs() <= 6) || (dx == 0 && dy.abs() <= 6);
                if is_streets {
                    let name = if dx == 0 && dy == 0 { "礦道石坪" } else if dx == -2 { "西窯道" } else if dx == 2 { "東選礦道" } else { "橫嶺通階" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}堅實平整的工礦碎石路面，路旁整齊擺放著選礦木筐與推車。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (-5..=-3).contains(&dx) && (2..=5).contains(&dy) {
                    (Terrain::DryField, "紅土磚場")
                } else if (3..=5).contains(&dx) && (-5..=-2).contains(&dy) {
                    (Terrain::Road, "選礦石坪")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 2 == 0 {
                    (Terrain::Road, "礦車儲坪")
                } else {
                    (Terrain::EmptyFurrow, "耐火泥池")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 8. 三號坑口：礦谷岩壁、主坑口與礦車鐵軌道 ──
            if vil_name == "三號坑口" {
                let is_cliff = dx <= -5 || (dy >= 4 && dx <= 1);
                let is_streets = (dy == 0 && dx >= -4 && dx <= 6) || (dx == 0 && dy >= -6 && dy <= 3);

                if is_cliff {
                    let mut cell = GridCell::new(coord, Terrain::Mountain, format_settlement_cell_name(vil_name, "礦谷岩壁"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}西側與北側陡峭聳立的礦山岩壁，露出富含赤鐵與烏金之礦脈。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_streets {
                    let name = if dy == 0 { "礦車鐵軌道" } else if dx == 0 && dy == 0 { "礦道石坪" } else { "坑口中街" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}鋪設鑄鐵軌條與平整枕木的礦車專用運輸道，直通主坑口。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (dx.abs() * 3 + dy.abs() * 7) % 3 == 0 {
                    (Terrain::Road, "選礦石坪")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 3 == 1 {
                    (Terrain::Road, "礦車儲坪")
                } else {
                    (Terrain::DryField, "耐火磚場")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 9. 水尾寮：黑沼竹筏水道與高腳烏木棧道 ──
            let is_canal = dy == -1 && dx.abs() <= 6 && dx != 0;
            let is_bridge = dx == 0 && dy.abs() <= 6;
            let is_boardwalk = (dy == 0 && dx.abs() <= 6) || (dy == 3 && dx.abs() <= 4) || (dy == -4 && dx.abs() <= 4);

            if is_canal {
                let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(vil_name, "竹筏水道"));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}穿流於高腳木屋下方的平靜水道，水尾村民撐篙划竹筏往來。", vil_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if is_bridge {
                let mut cell = GridCell::new(coord, Terrain::Bridge, format_settlement_cell_name(vil_name, "高腳烏木棧橋"));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}以耐水烏木高樁打入泥澤架設的通風棧橋，離水乾爽安全。", vil_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if is_boardwalk {
                let name = if dy == 0 { "水生木棧道" } else if dy > 0 { "蘆花北棧" } else { "澤南橫棧" };
                let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}以厚實烏木打樁架高的通風木棧道，兩旁茂密蘆荻隨風搖曳。", vil_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if vil_slots_set.contains(&(dx, dy)) {
                continue;
            }
            let (t, name) = if (dx.abs() * 3 + dy.abs() * 7) % 3 == 0 {
                (Terrain::Swamp, "蘆葦淺澤")
            } else if (dx.abs() * 3 + dy.abs() * 7) % 3 == 1 {
                (Terrain::FarmField, "菱角淺塘")
            } else {
                (Terrain::HerbGarden, "水草藥畦")
            };
            let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
            cell.zone = vil_name.to_string();
            cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
            cell.explored = true;
            map.insert(coord, cell);
        }
    }

    // 依序填入 15 棟建築至指定專屬座標
    for (idx, &(kdx, kdy)) in vil_slots.iter().enumerate() {
        let coord = SquareCoord::new(cx + kdx, cy + kdy);
        let (terrain, bldg_name) = bldg_types[idx];
        let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(vil_name, bldg_name));
        cell.zone = vil_name.to_string();
        cell.description = generate_settlement_building_desc(vil_name, bldg_name, terrain);
        cell.explored = true;
        map.insert(coord, cell);
    }
}

