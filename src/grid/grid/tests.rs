#[cfg(test)]
mod tests {
    use super::*;

    fn plain(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Plain, format!("({x},{y})"))
    }

    fn water(x: i32, y: i32) -> GridCell {
        GridCell::new(
            SquareCoord::new(x, y),
            Terrain::WaterDeep,
            format!("深水({x},{y})"),
        )
    }

    fn road(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Road, format!("街道({x},{y})"))
    }

    fn shop(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Market, format!("商號({x},{y})"))
    }

    fn workshop(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Workshop, format!("工坊({x},{y})"))
    }

    // ── 基本 CRUD ───────────────────────────────────────────────

    #[test]
    fn insert_and_get() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(1, 0));
        assert_eq!(g.len(), 2);
        assert!(g.contains(SquareCoord::new(0, 0)));
        assert!(!g.contains(SquareCoord::new(5, 5)));
        assert!(g.get(SquareCoord::new(1, 0)).is_some());
        assert_eq!(
            g.get(SquareCoord::new(1, 0)).unwrap().terrain,
            Terrain::Plain
        );
    }

    #[test]
    fn insert_overwrites_and_returns_old() {
        let mut g = SquareGrid::new();
        // 第一次插入，無舊值
        let old = g.insert(plain(0, 0));
        assert!(old.is_none());

        // 第二次插入同座標，應覆蓋並回傳舊格子
        let mut overwrite = plain(0, 0);
        overwrite.terrain = Terrain::Forest;
        let old2 = g.insert(overwrite);
        assert!(old2.is_some());
        assert_eq!(old2.unwrap().terrain, Terrain::Plain);
        assert_eq!(
            g.get(SquareCoord::new(0, 0)).unwrap().terrain,
            Terrain::Forest
        );
    }

    #[test]
    fn remove_cell() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        let removed = g.remove(SquareCoord::new(0, 0));
        assert!(removed.is_some());
        assert!(g.is_empty());
        assert!(g.remove(SquareCoord::new(0, 0)).is_none());
    }

    #[test]
    fn get_mut_modifies_in_place() {
        let mut g = SquareGrid::new();
        g.insert(plain(2, 3));
        g.get_mut(SquareCoord::new(2, 3)).unwrap().explored = true;
        assert!(g.get(SquareCoord::new(2, 3)).unwrap().explored);
    }

    #[test]
    fn world_seed_roundtrip() {
        let mut g = SquareGrid::new();
        g.set_world_seed(0xDEAD_BEEF_1234_5678);
        assert_eq!(g.world_seed(), 0xDEAD_BEEF_1234_5678);
    }

    // ── 序列化 ──────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let mut g = SquareGrid::new();
        g.set_world_seed(0xABCD_EF01_2345_6789);
        g.insert(plain(0, 0));
        g.insert(water(1, 0));

        let json = serde_json::to_string(&g).unwrap();
        let back: SquareGrid = serde_json::from_str(&json).unwrap();

        assert_eq!(back.world_seed(), 0xABCD_EF01_2345_6789);
        assert_eq!(back.len(), 2);
        assert!(back.contains(SquareCoord::new(0, 0)));
        assert_eq!(
            back.get(SquareCoord::new(1, 0)).unwrap().terrain,
            Terrain::WaterDeep
        );
    }

    #[test]
    fn serde_zone_and_tags() {
        let mut cell = plain(5, 5);
        cell.zone = "野外".into();
        cell.tags = vec!["危險".into(), "資源豐富".into()];

        let mut g = SquareGrid::new();
        g.insert(cell);
        let json = serde_json::to_string(&g).unwrap();
        let back: SquareGrid = serde_json::from_str(&json).unwrap();
        let c = back.get(SquareCoord::new(5, 5)).unwrap();
        assert_eq!(c.zone, "野外");
        assert_eq!(c.tags, vec!["危險", "資源豐富"]);
    }

    // ── 尋路：直線 ──────────────────────────────────────────────

    #[test]
    fn find_path_straight_line() {
        let mut g = SquareGrid::new();
        // 水平直線：(0,0) → (4,0)
        for x in 0..=4_i32 {
            g.insert(plain(x, 0));
        }
        let path = g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(4, 0))
            .unwrap();
        assert_eq!(path.first(), Some(&SquareCoord::new(0, 0)));
        assert_eq!(path.last(), Some(&SquareCoord::new(4, 0)));
        // BFS 直線最短路：5 格（含起終點）
        assert_eq!(path.len(), 5);
    }

    #[test]
    fn find_path_diagonal() {
        // (0,0) → (3,3)：四向制下 Manhattan 距離 6，路徑 7 格（含起終點）。
        // 需要插入完整 4x4 可走區塊。
        let mut g = SquareGrid::new();
        for x in 0..=3_i32 {
            for y in 0..=3_i32 {
                g.insert(plain(x, y));
            }
        }
        let path = g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(3, 3))
            .unwrap();
        assert_eq!(path.len(), 7);
        assert_eq!(path.first(), Some(&SquareCoord::new(0, 0)));
        assert_eq!(path.last(), Some(&SquareCoord::new(3, 3)));
    }

    #[test]
    fn find_path_same_cell() {
        let mut g = SquareGrid::new();
        g.insert(plain(3, 3));
        let path = g
            .find_path(SquareCoord::new(3, 3), SquareCoord::new(3, 3))
            .unwrap();
        assert_eq!(path, vec![SquareCoord::new(3, 3)]);
    }

    // ── 尋路：繞過障礙 ──────────────────────────────────────────

    #[test]
    fn find_path_around_water_obstacle() {
        // 佈局（y 軸向北為正）：
        //   (0,0) — 水(1,0) — (2,0)
        //   (0,-1) — (1,-1) — (2,-1)
        //
        // 直路被水擋，BFS 應繞南方一行（y = -1）
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        g.insert(water(1, 0)); // 障礙
        g.insert(plain(2, 0));
        g.insert(plain(0, -1));
        g.insert(plain(1, -1));
        g.insert(plain(2, -1));

        let path = g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(2, 0))
            .unwrap();

        // 路徑不得踩水格
        assert!(!path.contains(&SquareCoord::new(1, 0)));
        // 起點與終點正確
        assert_eq!(path.first(), Some(&SquareCoord::new(0, 0)));
        assert_eq!(path.last(), Some(&SquareCoord::new(2, 0)));
    }

    // ── 尋路：不可達 ────────────────────────────────────────────

    #[test]
    fn find_path_unreachable() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(10, 10)); // 孤立格，中間無路

        assert!(
            g.find_path(SquareCoord::new(0, 0), SquareCoord::new(10, 10))
                .is_none()
        );
    }

    #[test]
    fn find_path_blocked_by_water_ring() {
        // 目標格 (3,3) 被水格完整包圍，從 (0,0) 不可達
        //
        // 水格：(2,2)(3,2)(4,2)
        //       (2,3)     (4,3)
        //       (2,4)(3,4)(4,4)
        // 目標：(3,3)
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0)); // 起點（離目標有段距離）

        // 圍牆（全水）
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                if dx != 0 || dy != 0 {
                    g.insert(water(3 + dx, 3 + dy));
                }
            }
        }
        g.insert(plain(3, 3)); // 目標，被水圍死

        assert!(
            g.find_path(SquareCoord::new(0, 0), SquareCoord::new(3, 3))
                .is_none()
        );
    }

    #[test]
    fn find_path_missing_endpoint_is_none() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        // 終點不在地圖中
        assert!(
            g.find_path(SquareCoord::new(0, 0), SquareCoord::new(1, 0))
                .is_none()
        );
    }

    #[test]
    fn road_and_building_traversal() {
        let mut g = SquareGrid::new();
        g.insert(road(0, 0));
        g.insert(shop(1, 0));
        g.insert(workshop(2, 0));
        g.insert(road(3, 0));

        // 街道 :: 踏入商貿 $
        assert!(g.can_move_between(SquareCoord::new(0, 0), SquareCoord::new(1, 0)));
        // 商貿 $ 踏回街道 ::
        assert!(g.can_move_between(SquareCoord::new(1, 0), SquareCoord::new(0, 0)));
        // 建築隔離法則：兩座皆有街道出口的建築，商貿 $ 直接穿牆到工坊 ⚒︎ 應被禁止
        assert!(!g.can_move_between(SquareCoord::new(1, 0), SquareCoord::new(2, 0)));
    }

    #[test]
    fn dynamic_mutation_chains() {
        let mut g = SquareGrid::new();
        let c = SquareCoord::new(0, 0);

        // 1. 開山挖礦 Mountain -> MinePass -> Mountain
        g.insert(GridCell::new(c, Terrain::Mountain, "山峰".to_string()));
        let (old, new_t, _) = g.mutate_cell_terrain(c, "mine").unwrap();
        assert_eq!(old, Terrain::Mountain);
        assert_eq!(new_t, Terrain::MinePass);
        let (_, new_t2, _) = g.mutate_cell_terrain(c, "collapse").unwrap();
        assert_eq!(new_t2, Terrain::Mountain);

        // 2. 伐林清地 Forest -> Plain
        g.insert(GridCell::new(c, Terrain::Forest, "樹林".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "deforest").unwrap();
        assert_eq!(new_t, Terrain::Plain);

        // 3. 開墾整地 Plain -> EmptyFurrow
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plow").unwrap();
        assert_eq!(new_t, Terrain::EmptyFurrow);

        // 4. 栽種變化 EmptyFurrow -> FarmField -> EmptyFurrow -> DryField
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plant_paddy").unwrap();
        assert_eq!(new_t, Terrain::FarmField);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "harvest").unwrap();
        assert_eq!(new_t, Terrain::EmptyFurrow);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plant_dry_crop").unwrap();
        assert_eq!(new_t, Terrain::DryField);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "harvest").unwrap();
        assert_eq!(new_t, Terrain::EmptyFurrow);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plant_herbs").unwrap();
        assert_eq!(new_t, Terrain::HerbGarden);

        // 5. 鋪路 Plain <-> Road
        g.insert(GridCell::new(c, Terrain::Plain, "草野".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "pave_road").unwrap();
        assert_eq!(new_t, Terrain::Road);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "break_road").unwrap();
        assert_eq!(new_t, Terrain::Plain);

        // 6. 造橋 Water <-> Bridge
        g.insert(GridCell::new(c, Terrain::Water, "溪流".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "build_bridge").unwrap();
        assert_eq!(new_t, Terrain::Bridge);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "demolish_bridge").unwrap();
        assert_eq!(new_t, Terrain::Water);

        // 7. 水文漲退 Plain <-> Swamp
        g.insert(GridCell::new(c, Terrain::Plain, "荒野".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "flood").unwrap();
        assert_eq!(new_t, Terrain::Swamp);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "drain").unwrap();
        assert_eq!(new_t, Terrain::Plain);

        // 8. 聚落營造 Plain -> House -> demolish -> Plain
        let (_, new_t, _) = g.mutate_cell_terrain(c, "build_house").unwrap();
        assert_eq!(new_t, Terrain::House);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "demolish").unwrap();
        assert_eq!(new_t, Terrain::Plain);
    }

    #[test]
    fn derive_room_tags_and_cell_has_room_tag() {
        let granary = GridCell::new(SquareCoord::new(0, 0), Terrain::Granary, "大圳頭公有糧倉");
        assert!(granary.has_room_tag("grain"));
        assert!(granary.has_room_tag("granary"));
        assert!(granary.has_room_tag("counter"));
        assert!(granary.has_room_tag("restricted_staff_only"));

        let smithy = GridCell::new(SquareCoord::new(1, 0), Terrain::Blacksmith, "西嶺百鍛鐵工坊");
        assert!(smithy.has_room_tag("blacksmith"));
        assert!(smithy.has_room_tag("workshop"));
        assert!(smithy.has_room_tag("grindstone"));
        assert!(smithy.has_room_tag("ore"));

        let canteen = GridCell::new(SquareCoord::new(2, 0), Terrain::Tavern, "文曜・三水大食堂");
        assert!(canteen.has_room_tag("canteen"));
        assert!(canteen.has_room_tag("hot_food"));
        assert!(canteen.has_room_tag("fresh_drink"));
        assert!(canteen.has_room_tag("counter"));

        let sluice = GridCell::new(SquareCoord::new(3, 0), Terrain::Water, "老堰頭・清泉水脈一號分洪閘");
        assert!(sluice.has_room_tag("water"));
        assert!(sluice.has_room_tag("sluice"));

        let eff = canteen.effective_tags();
        assert!(eff.contains(&"canteen".to_string()));
        assert!(eff.contains(&"hot_food".to_string()));

        let dock_cell = GridCell::new(SquareCoord::new(4, 0), Terrain::Dock, "風舶角深水貨港");
        assert!(dock_cell.has_room_tag("dock"));
        assert!(dock_cell.has_room_tag("courier"));
        assert!(dock_cell.has_room_tag("fishing_port"));
        assert!(dock_cell.has_room_tag("waterfront"));

        let residence_cell = GridCell::new(SquareCoord::new(5, 0), Terrain::House, "三水口・擺渡老爹宿所");
        assert!(residence_cell.has_room_tag("residence"));
        assert!(!residence_cell.has_room_tag("waterfront"));
        assert!(!residence_cell.has_room_tag("street"));

        let bridge_cell = GridCell::new(SquareCoord::new(6, 0), Terrain::Bridge, "中心石橋廣場");
        assert!(bridge_cell.has_room_tag("bridge"));
        assert!(bridge_cell.has_room_tag("plaza"));

        let street_cell = GridCell::new(SquareCoord::new(7, 0), Terrain::Road, "三水大街");
        assert!(street_cell.has_room_tag("street"));
        assert!(street_cell.has_room_tag("road"));
    }

    #[test]
    fn test_find_nearest_room_by_tags_and_new_tags() {
        let mut grid = SquareGrid::new();
        let bath_cell = GridCell::new(SquareCoord::new(2, 2), Terrain::Plain, "溫泉香湯浴場");
        assert!(bath_cell.has_room_tag("bathhouse"));

        let tea_cell = GridCell::new(SquareCoord::new(0, 1), Terrain::Tavern, "路口奉茶歇舍");
        assert!(tea_cell.has_room_tag("teahouse"));
        assert!(tea_cell.has_room_tag("inn"));

        let well_cell = GridCell::new(SquareCoord::new(-1, 0), Terrain::Plain, "村口青石水井");
        assert!(well_cell.has_room_tag("well"));

        let dock_cell = GridCell::new(SquareCoord::new(0, 3), Terrain::Dock, "半島聯運渡口");
        assert!(dock_cell.has_room_tag("courier"));
        assert!(dock_cell.has_room_tag("dock"));

        let bridge_cell = GridCell::new(SquareCoord::new(1, 0), Terrain::Bridge, "中心石橋廣場");
        assert!(bridge_cell.has_room_tag("bridge"));
        assert!(bridge_cell.has_room_tag("plaza"));

        grid.insert(bath_cell);
        grid.insert(tea_cell);
        grid.insert(well_cell);
        grid.insert(dock_cell);
        grid.insert(bridge_cell);

        let nearest_well_en = grid.find_nearest_room_by_tags((0, 0), 5, &["well"], 0);
        assert_eq!(nearest_well_en, Some(((-1, 0), "村口青石水井".to_string())));

        let nearest_square_en = grid.find_nearest_room_by_tags((0, 0), 5, &["plaza"], 0);
        assert_eq!(nearest_square_en, Some(((1, 0), "中心石橋廣場".to_string())));

        assert_eq!(grid.find_nearest_room_by_tags((0, 0), 0, &["bathhouse"], 0), None);
        assert_eq!(
            grid.find_nearest_room_by_tags((0, 0), 5, &["bathhouse"], 0),
            Some(((2, 2), "溫泉香湯浴場".to_string()))
        );
    }

    #[test]
    fn test_room_venue_object_resolution() {
        let clinic = resolve_room_venue_object("半島百草診所", "").expect("clinic venue");
        assert_eq!(clinic.venue_kind, "醫館藥局");
        assert_eq!(clinic.default_host_title, "當值醫師");
        assert!(clinic.has_counter_restricted_zone);
        assert_eq!(clinic.lendable_tool.unwrap().0, "wt_cat_1057");
        assert!(!clinic.on_duty_demeanor.is_empty());
        assert!(!clinic.on_duty_talk_lines.is_empty());
        assert!(!clinic.counter_trade_offers.is_empty());

        let wood = resolve_room_venue_object("巧木工坊", "").expect("woodwork venue");
        assert_eq!(wood.venue_kind, "木作工坊");
        assert_eq!(wood.default_host_title, "當值木匠");
        assert_eq!(wood.lendable_tool.unwrap().0, "wt_cat_0956");
        assert!(!wood.on_duty_demeanor.is_empty());
        assert!(!wood.on_duty_talk_lines.is_empty());

        let forge = resolve_room_venue_object("西嶺百鍛鐵工坊", "").expect("forge venue");
        assert_eq!(forge.venue_kind, "百鍛鐵坊");
        assert_eq!(forge.default_host_title, "當值鐵匠");
        assert_eq!(forge.lendable_tool.unwrap().0, "wt_cat_0957");
        assert!(!forge.on_duty_demeanor.is_empty());
        assert!(!forge.on_duty_talk_lines.is_empty());

        let canteen_cell = GridCell::new(SquareCoord::new(2, 0), Terrain::Tavern, "文曜・三水大食堂");
        let canteen = canteen_cell.venue_object().expect("canteen venue");
        assert_eq!(canteen.venue_kind, "大食堂");
        assert_eq!(canteen.default_host_title, "食堂主廚");
        assert_eq!(canteen.lendable_tool.unwrap().0, "wt_cat_0897");
        assert!(!canteen.on_duty_demeanor.is_empty());
        assert!(!canteen.on_duty_talk_lines.is_empty());

        let mill = resolve_room_venue_object("老堰頭水輪石磨", "").expect("mill venue");
        assert_eq!(mill.venue_kind, "水輪石磨工坊");
        assert_eq!(mill.default_host_title, "當值石磨匠師");
        assert!(mill.lendable_tool.is_none());
        assert!(!mill.on_duty_demeanor.is_empty());
        assert!(!mill.on_duty_talk_lines.is_empty());

        // 驗證 resolve_market_or_specialty_room_object 解析
        let fish = resolve_market_or_specialty_room_object("早市鮮魚拍賣場", "魚販").expect("fish venue");
        assert_eq!(fish.venue_kind, "鮮魚水產市集");
        assert_eq!(fish.counter_trade_offers.len(), 3);
        assert_eq!(fish.counter_trade_offers[0].0, "wt_cat_0224");
        assert_eq!(fish.on_duty_talk_lines.len(), 3);

        let forge_stall = resolve_market_or_specialty_room_object("重鐵水錘鍛造坊", "").expect("forge stall venue");
        assert_eq!(forge_stall.venue_kind, "大鍛鐵匠鍛造坊");
        assert_eq!(forge_stall.counter_trade_offers[0].0, "wt_cat_0945");
        assert!(forge_stall.on_duty_talk_lines[0].contains("工坊大匠千錘百鍊"));

        let kiln = resolve_market_or_specialty_room_object("高山木炭窯", "").expect("kiln stall venue");
        assert_eq!(kiln.venue_kind, "石材磚瓦窯坊");
        assert!(kiln.on_duty_talk_lines[2].contains("青櫟硬木"));

        let livestock = resolve_market_or_specialty_room_object("車馬獸醫館", "").expect("livestock venue");
        assert_eq!(livestock.venue_kind, "牲畜交易獸醫館");
        assert!(livestock.on_duty_talk_lines[2].contains("乾草料厚磚"));

        let lane = resolve_market_or_specialty_room_object("西門街", "").expect("street venue");
        assert_eq!(lane.venue_kind, "街巷橋樑公共空地");
        assert!(lane.counter_trade_offers.is_empty());
        assert_eq!(lane.on_duty_talk_lines.len(), 3);

        // 驗證 resolve_any_room_venue_object 與 GridCell::venue_object
        let any_clinic = resolve_any_room_venue_object("半島百草診所", "").expect("any clinic");
        assert_eq!(any_clinic.venue_kind, "醫館藥局");
        let any_fish = resolve_any_room_venue_object("早市鮮魚拍賣場", "魚販").expect("any fish");
        assert_eq!(any_fish.venue_kind, "鮮魚水產市集");

        let fish_cell = GridCell::new(SquareCoord::new(10, 10), Terrain::Market, "早市鮮魚拍賣場");
        assert_eq!(fish_cell.venue_object().unwrap().venue_kind, "鮮魚水產市集");
    }


    #[test]
    fn test_resolve_room_ambient_visitor_activity() {
        // 文曜核心場域驗證
        let bridge_act = resolve_room_ambient_visitor_activity("文曜石橋", 0);
        assert_eq!(bridge_act, Some("憑著雕花青石欄杆遠眺三水川往來貨船"));

        let clinic_act = resolve_room_ambient_visitor_activity("文曜草藥診所", 0);
        assert_eq!(clinic_act, Some("在藥櫃前依藥方稱量烘乾的光葉藥草與止血水苔"));

        let council_act = resolve_room_ambient_visitor_activity("文曜公議堂", 1);
        assert_eq!(council_act, Some("站在水土民務告示立木前審視各街坊修繕委託單"));

        // 半島外圍聚落特色場域驗證
        let mill_act = resolve_room_ambient_visitor_activity("大圳頭水輪石磨坊", 0);
        assert_eq!(mill_act, Some("背著一袋新收稻穀在石磨槽旁等候過篩出粉"));

        let lumber_act = resolve_room_ambient_visitor_activity("北山鋸木場", 1);
        assert_eq!(lumber_act, Some("拂去衣袖上的松木鋸屑，清點綑紮妥當的板材"));

        let stone_act = resolve_room_ambient_visitor_activity("山坳切石坊", 0);
        assert_eq!(stone_act, Some("在導水冷卻槽旁端詳切整平滑的青石板材紋理"));

        let pasture_act = resolve_room_ambient_visitor_activity("牛埔乳酪工坊", 0);
        assert_eq!(pasture_act, Some("提著木桶在乳酪壓榨架旁等候新鮮凝乳分裝"));

        let mine_act = resolve_room_ambient_visitor_activity("赤嶺選礦場", 0);
        assert_eq!(mine_act, Some("推著裝滿赤鐵原礦的木輪車沿軌道緩步前行"));

        let dock_act = resolve_room_ambient_visitor_activity("風舶角漁業碼頭", 1);
        assert_eq!(dock_act, Some("站在棧橋邊核對靠岸貨船卸下的海鹽與魚乾"));

        // 預設通用場域
        let default_act = resolve_room_ambient_visitor_activity("尋常林間小道", 0);
        assert_eq!(default_act, Some("提著竹編提籃在三水口商號間採辦日用雜貨"));
    }
}
