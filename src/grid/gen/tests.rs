// ─── 單元測試 ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_grid_dimensions_and_counts() {
        assert_eq!(TOTAL_CELLS, 800_000);
        assert_eq!(GRID_WIDTH, 800);
        assert_eq!(GRID_HEIGHT, 1000);

        let grid = generate_white_tower_grid();

        // 1. 驗證虛擬總格數剛好為 800,000 格
        assert_eq!(grid.total_virtual_cells(), 800_000);

        // 2. 驗證四個角落座標皆能正常查詢且在地圖範圍內
        let corner_sw = SquareCoord::new(-400, -500);
        let corner_ne = SquareCoord::new(399, 499);
        assert!(in_peninsula_bounds(corner_sw));
        assert!(in_peninsula_bounds(corner_ne));
        assert!(grid.get(corner_sw).is_some());
        assert!(grid.get(corner_ne).is_some());
        assert!(grid.contains(corner_sw));
        assert!(grid.contains(corner_ne));

        // 3. 驗證關鍵正典聚落座標
        let wen_yao = grid.get(SquareCoord::new(0, 0)).expect("文曜中心 (0, 0) 必須存在");
        assert_eq!(wen_yao.zone, "文曜");

        let lao_yan_tou = grid.get(SquareCoord::new(0, 125)).expect("老堰頭中心 (0, 125) 必須存在");
        assert_eq!(lao_yan_tou.zone, "老堰頭");

        let feng_bo_jiao = grid.get(SquareCoord::new(-260, 160)).expect("風舶角中心 (-260, 160) 必須存在");
        assert_eq!(feng_bo_jiao.zone, "風舶角");

        // 4. 驗證 15 座聚落建築總數恰為 595 棟 (245 + 55 + 4x40 + 9x15 = 595)
        let settlements = generate_all_settlements();
        let total_buildings = settlements
            .values()
            .filter(|c| c.terrain.category() == "landmark")
            .count();
        assert_eq!(
            total_buildings, 595,
            "15 聚落建築總數必須精準等於 595 棟"
        );

        // 5. 驗證 19 處自然獵場在全圖顯式錨點中皆有覆蓋
        let mut zones_found: HashSet<String> = HashSet::new();
        for cell in grid.cells() {
            zones_found.insert(cell.zone.clone());
        }

        let required_wild_zones = [
            "長風草甸", "古河巨木林", "赤石坡", "落日岬", "驚濤岩岸",
            "巨藻淺灣", "黑石灣", "翠冠巨木林", "清泉溪谷", "北坡雲杉林",
            "西麓林道", "淺澤泥口", "白茫洲", "貼山深潭", "青籠關",
            "西嶺礦谷", "東海峭壁", "雲腰雪林", "白塔雪巔", "北冕海", "東海",
        ];
        for zone in required_wild_zones {
            assert!(
                zones_found.contains(zone),
                "遺漏自然獵場區塊：{}",
                zone
            );
        }

        // 6. 驗證 15 聚落皆存在
        let required_settlements = [
            "文曜", "風舶角", "風舶嶼", "大圳頭", "半嶺鎮", "南哨",
            "老堰頭", "北嶺腳", "霜林口", "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口", "水尾寮",
        ];
        for s in required_settlements {
            assert!(
                zones_found.contains(s),
                "遺漏聚落：{}",
                s
            );
        }
    }

    #[test]
    fn test_wen_yao_canonical_layout_and_no_duplicate_prefix() {
        let settlements = generate_all_settlements();

        // 1. 驗證 (0, 0) 三水匯流大石橋
        let center = settlements
            .get(&SquareCoord::new(0, 0))
            .expect("文曜中心 (0, 0) 必須存在");
        assert_eq!(center.name, "文曜・三水匯流大石橋");
        assert!(center.description.contains("三水川"));

        // 2. 檢驗文曜全域 (-17..=17, -17..=17) 共 1,225 格
        let forbidden = ["debug", "資料庫", "database", "grid", "scene_stub", "還有", "聖塔"];
        let mut wen_yao_landmarks = Vec::new();
        let mut wen_yao_all_names = HashSet::new();

        for dx in -17..=17 {
            for dy in -17..=17 {
                let coord = SquareCoord::new(dx, dy);
                let cell = settlements.get(&coord).unwrap_or_else(|| {
                    panic!("文曜座標 ({}, {}) 必須存在", dx, dy);
                });
                assert_eq!(cell.zone, "文曜");

                // 不得出現重複前綴
                assert!(
                    !cell.name.contains("文曜・文曜") && !cell.name.contains("文曜文曜"),
                    "發現重複聚落前綴：{}",
                    cell.name
                );
                assert!(
                    cell.name.starts_with("文曜・"),
                    "文曜格子名稱必須以 '文曜・' 開頭：{}",
                    cell.name
                );

                // 不得包含禁用詞
                for word in forbidden {
                    assert!(
                        !cell.name.to_lowercase().contains(word),
                        "名稱含有禁詞 '{}': {}",
                        word,
                        cell.name
                    );
                    assert!(
                        !cell.description.to_lowercase().contains(word),
                        "描述含有禁詞 '{}': {}",
                        word,
                        cell.description
                    );
                }

                wen_yao_all_names.insert(cell.name.clone());

                if cell.terrain.category() == "landmark" {
                    wen_yao_landmarks.push((coord, cell.name.clone()));
                }
            }
        }

        // 3. 驗證文曜 landmark 數量恰為 245 且名稱無重複（95 棟功能／商業地標 + 150 棟民居住宅）
        assert_eq!(
            wen_yao_landmarks.len(),
            245,
            "文曜地標建築數量必須恰好為 245 棟"
        );
        let wen_yao_commercial = wen_yao_landmarks
            .iter()
            .filter(|(coord, _)| {
                let cell = settlements.get(coord).unwrap();
                cell.terrain.category() == "landmark" && cell.terrain != Terrain::House
            })
            .count();
        assert_eq!(
            wen_yao_commercial,
            95,
            "文曜商業與功能地標數量必須恰好為 95 棟"
        );
        let wen_yao_residential = wen_yao_landmarks
            .iter()
            .filter(|(coord, _)| {
                let cell = settlements.get(coord).unwrap();
                cell.terrain == Terrain::House
            })
            .count();
        assert_eq!(
            wen_yao_residential,
            150,
            "文曜住宅民居數量必須恰好為 150 棟"
        );
        let mut unique_landmarks = HashSet::new();
        for (_, name) in &wen_yao_landmarks {
            assert!(
                unique_landmarks.insert(name.clone()),
                "文曜地標建築名稱重複：{}",
                name
            );
        }

        // 4. 驗證四大街區核心標誌建築皆存在
        let expected_canonical_buildings = [
            // 西北三水口
            "文曜・三水總商行總部",
            "文曜・傳訊鐘樓與信使總站",
            "文曜・三水大食堂",
            "文曜・泉香溫水浴場",
            "文曜・三水口乾貨海味行",
            "文曜・三水口・陸家宅邸",
            // 東北講堂街
            "文曜・學院主校區",
            "文曜・學院觀星台與評議廳",
            "文曜・藏書閣與印行館",
            "文曜・百工商算學舍",
            "文曜・溫氏脈絡診所",
            "文曜・講堂街・陸氏清雅書齋",
            // 東南分家巷
            "文曜・岑氏第一鍛造工坊",
            "文曜・東橋旅社與山風茶屋",
            "文曜・巧木居所傢俱坊",
            "文曜・雲錦織造與服飾坊",
            "文曜・分家巷・岑氏匠師宗宅",
            // 西南西門
            "文曜・公議堂",
            "文曜・公議堂房地契書局",
            "文曜・西門車馬聯運總站",
            "文曜・韓氏聯運貨運行",
            "文曜・西門・韓氏總站宅院",
            // 西北三水外坊
            "文曜・三水外坊・北灣貨船大埠",
            "文曜・三水外坊・早市鮮魚拍賣場",
            "文曜・三水外坊・通濟河運總倉",
            "文曜・三水外坊・老舵手石砌院",
            // 東北講堂外坊
            "文曜・講堂外坊・格物輿圖外院",
            "文曜・講堂外坊・活字排版印書局",
            "文曜・講堂外坊・杏林百草醫館",
            "文曜・講堂外坊・刻書雕版匠居",
            // 東南百工外坊
            "百工外坊・重鐵水錘大鍛坊",
            "百工外坊・魯班大木車船坊",
            "百工外坊・五金農具總商行",
            "百工外坊・司爐老鐵匠院",
            // 西南西門外坊
            "西門外坊・萬里駝隊大驛站",
            "西門外坊・西門皮毛大市",
            "西門外坊・常平糧草總大庫",
            "西門外坊・老護衛四合院",
        ];
        for b in expected_canonical_buildings {
            let full_name = if b.starts_with("文曜・") {
                b.to_string()
            } else {
                format!("文曜・{}", b)
            };
            assert!(
                unique_landmarks.contains(&full_name),
                "文曜缺少正典標誌建築：{}",
                full_name
            );
        }

        // 5. 驗證分段寫實街道命名存在
        let expected_streets = [
            "文曜・三水口北街",
            "文曜・鐘樓前街",
            "文曜・講堂街南口",
            "文曜・講堂街中段",
            "文曜・講堂街北段",
            "文曜・三水東街",
            "文曜・東橋石板拱橋",
            "文曜・東門石板大道",
            "文曜・公議堂東街",
            "文曜・分家巷百工街口",
            "文曜・南門石板大街",
            "文曜・三水口西街",
            "文曜・西門聯運大道",
            "文曜・西門車馬前街",
            "文曜・講堂街學院巷",
            "文曜・分家巷百工石道",
            "文曜・三水口商號街",
            "文曜・西門貨棧街",
            // 內外門坊與外環道
            "文曜・北內門坊",
            "文曜・南內門坊",
            "文曜・東內門坊",
            "文曜・西內門坊",
            "文曜・北城門",
            "文曜・南城門",
            "文曜・東城門",
            "文曜・西城門",
            "文曜・三水外環北道",
            "文曜・講堂外環北道",
            "文曜・百工外環南道",
            "文曜・西門外環南道",
            "文曜・北門外街南段",
            "文曜・南門外街北段",
            "文曜・東門外街西段",
            "文曜・西門外街東段",
            "文曜・三水內環西北隅",
            "文曜・講堂內環東北隅",
            "文曜・分家內環東南隅",
            "文曜・西門內環西南隅",
        ];
        for s in expected_streets {
            assert!(
                wen_yao_all_names.contains(s),
                "文曜缺少分段街道：{}",
                s
            );
        }
    }

    #[test]
    fn test_canonical_hunting_grounds_and_wilderness_road_naming() {
        let grid = generate_white_tower_grid();

        // 1. 驗證 19 處獵場正典中心皆落在對應獵場區（尤其大圳頭 4 獵場與西北西麓林道）
        let expected_centers = [
            ("長風草甸", (-180, -195)),
            ("古河巨木林", (-80, -60)),
            ("赤石坡", (-85, -180)),
            ("落日岬", (-340, -140)),
            ("驚濤岩岸", (-360, 180)),
            ("巨藻淺灣", (-300, 260)),
            ("黑石灣", (-260, -20)),
            ("西麓林道", (-130, 80)),
            ("清泉溪谷", (60, 160)),
            ("翠冠巨木林", (0, 240)),
            ("北坡雲杉林", (80, 300)),
            ("淺澤泥口", (0, -340)),
            ("白茫洲", (-120, -320)),
            ("貼山深潭", (120, -320)),
            ("青籠關", (0, -450)),
            ("西嶺礦谷", (180, 40)),
            ("東海峭壁", (320, 200)),
            ("雲腰雪林", (180, 320)),
            ("白塔雪巔", (0, 420)),
        ];
        for (expected_zone, (cx, cy)) in expected_centers {
            let (actual_zone, _, _) = classify_wilderness_cell(cx, cy);
            assert_eq!(
                actual_zone, expected_zone,
                "獵場 {} 中心 ({}, {}) 卻被分類為 {}",
                expected_zone, cx, cy, actual_zone
            );
        }

        // 2. 驗證文曜西城門外 (-63, 0) 為西麓林道・道路，而非長風大道或百工大道
        let west_road = grid
            .get(SquareCoord::new(-63, 0))
            .expect("文曜西城門外 (-63, 0) 必須存在");
        assert_eq!(west_road.zone, "西麓林道");
        assert_eq!(west_road.name, "西麓林道・道路");

        // 3. 驗證大圳頭長風草甸內道路 (-180, -195) 為長風草甸・道路
        let meadow_road = grid
            .get(SquareCoord::new(-180, -195))
            .expect("大圳頭長風草甸道路 (-180, -195) 必須存在");
        assert_eq!(meadow_road.zone, "長風草甸");
        assert_eq!(meadow_road.name, "長風草甸・道路");

        // 4. 驗證全圖所有顯式格子皆不含非正典「長風大道／百工大道／百工原／西南原大道／幹道」
        let forbidden = ["長風大道", "百工大道", "百工原", "西南原大道", "西北幹道", "西南幹道", "半島主幹道"];
        for cell in grid.cells() {
            for bad in forbidden {
                assert!(
                    !cell.name.contains(bad) && !cell.zone.contains(bad),
                    "發現非正典道路或區域命名 ({}, {}): zone={}, name={}",
                    cell.coord.x,
                    cell.coord.y,
                    cell.zone,
                    cell.name
                );
            }
        }
    }

    #[test]
    fn test_wilderness_road_and_hunting_ground_objects() {
        let grid = generate_white_tower_grid();

        // 1. 文曜四城門外道路格必有指路石碑與野地營火
        let gate_coords = [(-18, 0), (18, 0), (0, 18), (0, -18)];
        for &(gx, gy) in &gate_coords {
            let coord = SquareCoord::new(gx, gy);
            let cell = grid.get(coord).expect("城門外道路格必須存在");
            assert!(
                cell.objects.iter().any(|o| o.name.contains("指路石碑")),
                "門外道路 ({}, {}) 必須有指路石碑", gx, gy
            );
            assert!(
                cell.objects.iter().any(|o| o.name == "野地營火"),
                "門外道路 ({}, {}) 必須有野地營火", gx, gy
            );
        }

        // 2. 19 處正典獵場非道路格至少有 2 件物件（1 件主要資源 + 1 件次要生態節點）
        let hunting_centers = [
            (-181, -195), // 長風草甸
            (-80, -60),   // 古河巨木林
            (-86, -180),  // 赤石坡
            (-340, -140), // 落日岬
            (-360, 180),  // 驚濤岩岸
            (-300, 260),  // 巨藻淺灣
            (-260, -20),  // 黑石灣
            (-130, 80),   // 西麓林道
            (60, 160),    // 清泉溪谷
            (1, 240),     // 翠冠巨木林
            (80, 301),    // 北坡雲杉林
            (1, -340),    // 淺澤泥口
            (-120, -320), // 白茫洲
            (120, -320),  // 貼山深潭
            (1, -450),    // 青籠關
            (180, 40),    // 西嶺礦谷
            (320, 200),   // 東海峭壁
            (180, 320),   // 雲腰雪林
            (1, 420),     // 白塔雪巔
        ];
        for &(hx, hy) in &hunting_centers {
            let coord = SquareCoord::new(hx, hy);
            let cell = grid.get(coord).expect("獵場中心格必須存在");
            assert!(
                cell.objects.len() >= 2,
                "獵場中心 ({}, {}) 物件數必須 >= 2，實際只有 {}",
                hx, hy, cell.objects.len()
            );
            let sec_names = ["獸徑足印", "野地營火", "山澗清泉眼", "避風岩龕", "巡山木樁"];
            assert!(
                cell.objects.iter().any(|o| sec_names.contains(&o.name.as_str())),
                "獵場中心 ({}, {}) 必須包含次要生態節點之一", hx, hy
            );
        }

        // 3. 道路格皆有物件
        let road_cell = grid.get(SquareCoord::new(-63, 0)).expect("道路格必須存在");
        assert!(!road_cell.objects.is_empty(), "道路格必須有路旁物件");
    }

    #[test]
    fn test_continuous_river_and_canal_system() {
        let grid = generate_white_tower_grid();

        // 1. 北源清泉水脈貫通 (y in 18..=385)
        let north_sample = grid.get(SquareCoord::new(-4, 100)).expect("北源清泉水脈 (-4, 100) 必須存在");
        assert_eq!(north_sample.terrain, Terrain::Water);
        // 2. 東源清泉溪貫通 (18..=155)
        let east_sample = grid.get(SquareCoord::new(18, 2)).expect("東源清泉溪 (18, 2) 必須存在");
        assert_eq!(east_sample.terrain, Terrain::Water);

        // 3. 北嶺腳 (110, 115) 與周邊 (100, 115) 絕非舊版 24,000 格大水塊溺水
        let bei_ling_jiao = grid.get(SquareCoord::new(110, 115)).expect("北嶺腳中心 (110, 115) 必須存在");
        assert_ne!(bei_ling_jiao.terrain, Terrain::Water);
        let dry_valley = classify_wilderness_cell(100, 115);
        assert_ne!(dry_valley.2, Terrain::Water);

        // 4. 清泉溪谷正典中心 (60, 160) 保持水質溪谷
        let qing_quan = classify_wilderness_cell(60, 160);
        assert_eq!(qing_quan.0, "清泉溪谷");
        assert_eq!(qing_quan.1, "溪谷");
        assert_eq!(qing_quan.2, Terrain::Water);

        // 5. 西南大圳 (x in -235..=-2) 貫穿大圳頭 (-180, -138)
        let canal_sample = grid.get(SquareCoord::new(-100, -109)).expect("西南大圳 (-100, -109) 必須存在");
        assert_eq!(canal_sample.terrain, Terrain::Water);
        assert_eq!(canal_sample.name, "古河巨木林・大圳");
    }

    #[test]
    fn test_settlement_building_counts_and_walkability() {
        let settlements = generate_all_settlements();

        // 1. 建築總數恰為 595 棟
        let total_buildings = settlements
            .values()
            .filter(|c| c.terrain.category() == "landmark")
            .count();
        assert_eq!(total_buildings, 595, "所有 15 座聚落的地標建築總數必須為 595 棟");

        // 2. 每棟建築必須四向相鄰至少一格可通行的道路或橋樑（無孤立受困建築）
        let dirs = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for (coord, cell) in &settlements {
            if cell.terrain.category() == "landmark" {
                let has_adjacent_walkable = dirs.iter().any(|&(dx, dy)| {
                    let neighbor_coord = SquareCoord::new(coord.x + dx, coord.y + dy);
                    if let Some(neighbor) = settlements.get(&neighbor_coord) {
                        neighbor.terrain == Terrain::Road
                            || neighbor.terrain == Terrain::Bridge
                            || neighbor.terrain.walkable()
                    } else {
                        false
                    }
                });
                assert!(
                    has_adjacent_walkable,
                    "聚落建築 {} ({}, {}) 四向無相鄰之可通行道路或木橋！",
                    cell.name, coord.x, coord.y
                );
            }
        }

        // 3. 聚落內部不得存在孤立 1x1 水體受困格
        for (coord, cell) in &settlements {
            if cell.terrain == Terrain::Water && !["文曜", "風舶角", "風舶嶼", "風櫃尾", "烏石鼻"].contains(&cell.zone.as_str()) {
                // 對於非海港市鎮，內部水體（如老堰頭溪水、大圳頭大渠、霜林口溪流、烏石鼻漁灣、水尾寮水道）
                // 必須緊鄰可通行橋樑或水岸步道，不得為單獨封閉死格
                let adjacent_bridge_or_road = dirs.iter().any(|&(dx, dy)| {
                    let n_coord = SquareCoord::new(coord.x + dx, coord.y + dy);
                    if let Some(neighbor) = settlements.get(&n_coord) {
                        neighbor.terrain == Terrain::Bridge || neighbor.terrain == Terrain::Road || neighbor.terrain == Terrain::Dock
                    } else {
                        false
                    }
                });
                assert!(
                    adjacent_bridge_or_road,
                    "聚落 {} 內的水面格 ({}, {}) 無相鄰道路或橋樑！",
                    cell.name, coord.x, coord.y
                );
            }
        }
    }
    #[test]
    fn test_fengbo_strait_and_barrier_island_geometry() {
        let grid = generate_white_tower_grid();

        // 1. 水道中心點 (-300, 160) 與南北出海口 (-300, 195), (-300, 110) 為天然港灣道・水域
        let strait_sample = classify_wilderness_cell(-300, 160);
        assert_eq!(strait_sample.0, "天然港灣道");
        assert_eq!(strait_sample.2, Terrain::Water);

        let north_mouth = classify_wilderness_cell(-300, 195);
        assert_eq!(north_mouth.0, "天然港灣道");
        assert_eq!(north_mouth.2, Terrain::Water);

        let south_mouth = classify_wilderness_cell(-300, 110);
        assert_eq!(south_mouth.0, "天然港灣道");
        assert_eq!(south_mouth.2, Terrain::Water);

        // 2. 驗證顯式生成的 (-300, 195) 在 grid 中存在且為天然港灣道
        let grid_north_mouth = grid.get(SquareCoord::new(-300, 195)).expect("(-300, 195) 必須顯式存在");
        assert_eq!(grid_north_mouth.zone, "天然港灣道");
        assert_eq!(grid_north_mouth.terrain, Terrain::Water);

        // 3. 驗證驚濤岩岸正典中心 (-360, 180) 位於風舶外島上
        let jing_tao = classify_wilderness_cell(-360, 180);
        assert_eq!(jing_tao.0, "驚濤岩岸");

        // 4. 驗證外海 (-380, 160) 為北冕海・深水
        let deep_sea = classify_wilderness_cell(-380, 160);
        assert_eq!(deep_sea.0, "北冕海");
        assert_eq!(deep_sea.2, Terrain::WaterDeep);

        // 5. 驗證青籠關以南地峽直通南方未知大陸，絕非深海外海
        let south_pass = classify_wilderness_cell(0, -490);
        assert_eq!(south_pass.0, "青籠關");
        assert_ne!(south_pass.2, Terrain::WaterDeep);

        let south_west = classify_wilderness_cell(-150, -480);
        assert_eq!(south_west.0, "青籠關");
        assert_ne!(south_west.2, Terrain::WaterDeep);

        let south_east = classify_wilderness_cell(150, -480);
        assert_eq!(south_east.0, "青籠關");
        assert_ne!(south_east.2, Terrain::WaterDeep);

        // 6. 驗證貼山深潭收束於青脊山西麓盆地，且東南青脊山主脊（西嶺礦谷）向南延伸銜接青籠關
        let tie_shan_basin = classify_wilderness_cell(120, -320);
        assert_eq!(tie_shan_basin.0, "貼山深潭");
        let southeast_ridge = classify_wilderness_cell(230, -320);
        assert_eq!(southeast_ridge.0, "西嶺礦谷");

        // 7. 驗證全島荒野分類器回傳之所有 (zone, sub_geo) 皆已登錄於 WILDERNESS_PROTOTYPES，絕無退回通用名稱
        for y in (-500..=499).step_by(3) {
            for x in (-400..=399).step_by(3) {
                let (z, g, _) = classify_wilderness_cell(x, y);
                assert!(
                    WILDERNESS_PROTOTYPES.contains_key(&(z, g)),
                    "座標 ({}, {}) 分類為 ({}, {}) 未登錄於 WILDERNESS_PROTOTYPES！",
                    x,
                    y,
                    z,
                    g
                );
            }
        }
    }
}
