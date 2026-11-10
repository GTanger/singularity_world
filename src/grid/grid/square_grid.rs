impl GridCell {
    pub fn new(coord: SquareCoord, terrain: Terrain, name: impl Into<String>) -> Self {
        let name_str = name.into();
        let tags = derive_room_tags(terrain, &name_str, "");
        Self {
            coord,
            terrain,
            name: name_str,
            zone: String::new(),
            tags,
            description: String::new(),
            objects: Vec::new(),
            explored: false,
            freshness: Freshness::Fresh,
            explicit_exits: Vec::new(),
        }
    }

    /// 取得格子目前所有生效的語意標籤（包含持久化標籤與動態推導標籤）
    pub fn effective_tags(&self) -> Vec<String> {
        let mut all = self.tags.clone();
        for t in derive_room_tags(self.terrain, &self.name, &self.zone) {
            if !all.contains(&t) {
                all.push(t);
            }
        }
        all
    }

    /// 檢查格子是否具備指定的語意標籤
    pub fn has_room_tag(&self, wanted: &str) -> bool {
        if wanted.is_empty() {
            return false;
        }
        let canon_wanted = canonical_room_tag(wanted);
        let matches_tag = |t: &str| {
            t == wanted || (!canon_wanted.is_empty() && canonical_room_tag(t) == canon_wanted)
        };
        if self.tags.iter().any(|t| matches_tag(t)) {
            return true;
        }
        let derived = derive_room_tags(self.terrain, &self.name, &self.zone);
        if derived.iter().any(|t| matches_tag(t)) {
            return true;
        }
        false
    }

    /// 依房間名稱與分區推導結構化房間設施物件
    pub fn venue_object(&self) -> Option<RoomVenueObject> {
        if self.terrain == Terrain::House || is_private_residence_room_name(&self.name) {
            return None;
        }
        resolve_any_room_venue_object(&self.name, &self.zone)
    }
}

// ─── 自訂 Serialize / Deserialize（cells 轉 Vec 以相容 JSON）───

/// 方格地圖：world_seed + 已揭露格子集合。
///
/// 格子以 `SquareCoord` 為鍵存入 `HashMap`；序列化時轉成 `Vec`
/// 與 `HexGrid` 慣例一致。未插入 = 黑格（未揭露）。
///
/// 相鄰格之間預設八方皆通；只有不可行走地形（Water、Mountain 等）阻斷移動。
#[derive(Debug, Clone, Default)]

/// 序列化用的中間結構（cells 轉 Vec，符合 JSON 慣例）
#[derive(Serialize, Deserialize)]
struct SquareGridSer {
    #[serde(default)]
    world_seed: u64,
    cells: Vec<GridCell>,
}

impl Serialize for SquareGrid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let ser = SquareGridSer {
            world_seed: self.world_seed,
            cells: self.cells.values().cloned().collect(),
        };
        ser.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SquareGrid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ser = SquareGridSer::deserialize(deserializer)?;
        let cells = ser.cells.into_iter().map(|c| (c.coord, c)).collect();
        Ok(SquareGrid {
            world_seed: ser.world_seed,
            cells: Arc::new(cells),
            dirty: false,
        })
    }
}

// ─── Cell CRUD ───────────────────────────────────────────────────

impl SquareGrid {
    /// 建立空地圖（world_seed 預設 0，可用 `set_world_seed` 覆蓋）
    pub fn new() -> Self {
        Self::default()
    }

    pub fn world_seed(&self) -> u64 {
        self.world_seed
    }

    pub fn set_world_seed(&mut self, seed: u64) {
        self.world_seed = seed;
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// 插入格子；若座標已存在則覆蓋並回傳舊值。
    pub fn insert(&mut self, cell: GridCell) -> Option<GridCell> {
        let coord = cell.coord;
        self.dirty = true;
        Arc::make_mut(&mut self.cells).insert(coord, cell)
    }

    /// 移除格子；不存在時回傳 `None`。
    pub fn remove(&mut self, coord: SquareCoord) -> Option<GridCell> {
        let r = Arc::make_mut(&mut self.cells).remove(&coord);
        if r.is_some() {
            self.dirty = true;
        }
        r
    }
    /// 是否為顯式儲存的格子（而非依據種子即時生成的荒野格）
    pub fn has_explicit_cell(&self, coord: SquareCoord) -> bool {
        self.cells.contains_key(&coord)
    }

    /// 是否啟用白塔半島 800×1,000 無縫荒野基底層
    #[inline]
    pub fn has_peninsula_wilderness(&self) -> bool {
        self.world_seed == 2026_07_03 && self.cells.len() > 2000
    }

    /// 取得格子（未揭露回傳 `None`）
    pub fn get(&self, coord: SquareCoord) -> Option<&GridCell> {
        if let Some(c) = self.cells.get(&coord) {
            return Some(c);
        }
        if self.has_peninsula_wilderness() && super::r#gen::in_peninsula_bounds(coord) {
            return super::r#gen::procedural_wilderness_cell_ref(coord);
        }
        None
    }

    /// 取得格子（可寫）——呼叫者修改內容後應呼叫 `mark_dirty()`。
    pub fn get_mut(&mut self, coord: SquareCoord) -> Option<&mut GridCell> {
        if !self.cells.contains_key(&coord)
            && self.has_peninsula_wilderness()
            && super::r#gen::in_peninsula_bounds(coord)
        {
            let cell = super::r#gen::materialize_wilderness_cell(coord);
            Arc::make_mut(&mut self.cells).insert(coord, cell);
            self.dirty = true;
        }
        Arc::make_mut(&mut self.cells).get_mut(&coord)
    }

    /// 查詢格子是否已揭露
    pub fn contains(&self, coord: SquareCoord) -> bool {
        self.cells.contains_key(&coord)
            || (self.has_peninsula_wilderness() && super::r#gen::in_peninsula_bounds(coord))
    }

    /// 已揭露格子總數
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// 虛擬全圖格數（半島正典為 800,000 格）
    pub fn total_virtual_cells(&self) -> usize {
        if self.has_peninsula_wilderness() {
            super::r#gen::TOTAL_CELLS
        } else {
            self.cells.len()
        }
    }


    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// 遍歷所有已揭露格子
    pub fn cells(&self) -> impl Iterator<Item = &GridCell> {
        self.cells.values()
    }

    /// 遍歷所有已揭露格子（可寫；修改後應標記 dirty）
    pub fn cells_mut(&mut self) -> impl Iterator<Item = &mut GridCell> {
        Arc::make_mut(&mut self.cells).values_mut()
    }

    /// 遍歷所有已揭露座標
    pub fn coords(&self) -> impl Iterator<Item = &SquareCoord> {
        self.cells.keys()
    }

}


// ─── 鄰接 & 移動 ─────────────────────────────────────────────────

impl SquareGrid {
    /// 回傳已存在的相鄰格（八方向，不考慮地形通行性）
    pub fn neighbors(&self, coord: SquareCoord) -> Vec<&GridCell> {
        coord
            .neighbors()
            .into_iter()
            .filter_map(|n| self.get(n))
            .collect()
    }

    /// 能否從 `from` 步行到相鄰的 `to`？（別名方法）
    pub fn can_move_between(&self, from: SquareCoord, to: SquareCoord) -> bool {
        self.can_walk(from, to)
    }

    pub fn has_direct_non_building_walkable_exit(&self, coord: SquareCoord) -> bool {
        for &(dx, dy) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
            let nb = SquareCoord::new(coord.x + dx, coord.y + dy);
            if let Some(c) = self.get(nb) {
                if c.terrain.walkable()
                    && c.terrain.category() != "landmark"
                    && c.terrain != Terrain::Water
                {
                    return true;
                }
            }
        }
        false
    }

    /// 能否從 `from` 步行到相鄰的 `to`？
    ///
    /// 若 `from` 有顯式出口（explicit_exits），以出口表為準；
    /// 否則回退到座標鄰接邏輯（Manhattan 距離 1、地形可走、街道與公開建築格暢通、建築隔離法則）。
    pub fn can_walk(&self, from: SquareCoord, to: SquareCoord) -> bool {
        let Some(from_cell) = self.get(from) else {
            return false;
        };
        let Some(to_cell) = self.get(to) else {
            return false;
        };

        // 顯式出口覆蓋座標邏輯
        if !from_cell.explicit_exits.is_empty() {
            return from_cell.explicit_exits.iter().any(|e| e.target == to);
        }

        // Fallback: 座標鄰接邏輯
        if from.manhattan_distance(to) != 1 {
            return false;
        }
        if !to_cell.terrain.walkable() {
            return false;
        }
        if self.world_seed == crate::grid::SHALU_ZHULIN_WORLD_SEED {
            return true;
        }

        // 街道 :: 與相鄰公開建築格（$ ⚒︎ + ⌂ ⚑ 𖠿 ∆）互相開放通行
        let is_bldg = |t: Terrain| -> bool {
            t.category() == "landmark"
        };
        let is_road = |t: Terrain| -> bool {
            t == Terrain::Road
        };

        if is_road(from_cell.terrain) && is_bldg(to_cell.terrain) {
            return true;
        }
        if is_bldg(from_cell.terrain) && is_road(to_cell.terrain) {
            return true;
        }

        // 城鎮內河水道隔離：禁止由建築或城鎮街道直接踏入深渠水體（須經石橋 ≍ 通行）
        if (from_cell.terrain == Terrain::Water || to_cell.terrain == Terrain::Water)
            && (!from_cell.zone.is_empty() || !to_cell.zone.is_empty() || is_bldg(from_cell.terrain) || is_bldg(to_cell.terrain))
        {
            return false;
        }

        // 建築隔離法則：禁止建築物 -> 建築物 的直接穿牆（同名複合建築格除外）
        if is_bldg(from_cell.terrain) && is_bldg(to_cell.terrain) {
            if from_cell.name != to_cell.name
                && self.has_direct_non_building_walkable_exit(from)
                && self.has_direct_non_building_walkable_exit(to)
            {
                return false;
            }
        }
        true
    }

    /// 動態改變網格地形（10 條動態地形演變鏈）。
    ///
    /// 成功時回傳 `Ok((old_terrain, new_terrain, 描述字串))`，並將網格標記為 dirty。
    pub fn mutate_cell_terrain(
        &mut self,
        coord: SquareCoord,
        action: &str,
    ) -> Result<(Terrain, Terrain, String), String> {
        let cell = self.get_mut(coord).ok_or_else(|| {
            format!("座標 ({}, {}) 處無有效網格", coord.x, coord.y)
        })?;
        let old = cell.terrain;
        let (new_terrain, desc) = match (old, action) {
            // 1. 開山挖礦 / 坍方還山
            (Terrain::Mountain, "mine" | "quarry" | "excavate") => (
                Terrain::MinePass,
                "劈石開道，山體岩壁被鑿出可供通行的礦道山隘。".to_string(),
            ),
            (Terrain::MinePass, "collapse" | "restore_mountain") => (
                Terrain::Mountain,
                "岩層坍塌封死礦道，重歸高聳嶙峋的實心山脊。".to_string(),
            ),

            // 2. 伐林清地
            (
                Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle,
                "deforest" | "clear_trees" | "logging",
            ) => (
                Terrain::Plain,
                "伐倒巨木灌叢、清理樹樁，林地化為一片開闊平野。".to_string(),
            ),

            // 3. 開墾整地
            (Terrain::Plain | Terrain::Grassland, "plow" | "till" | "cultivate") => (
                Terrain::EmptyFurrow,
                "犁開熟土、起壟除草，平野闢為待播種之空田壟。".to_string(),
            ),

            // 4. 依栽種變化、收成與休耕復育
            (Terrain::EmptyFurrow, "plant_paddy" | "irrigate" | "plant_rice") => (
                Terrain::FarmField,
                "引清流灌渠入壟，阡陌縱橫，注滿水光映照之水田。".to_string(),
            ),
            (Terrain::EmptyFurrow, "plant_dry_crop" | "plant_wheat" | "dry_field") => (
                Terrain::DryField,
                "耙土覆麥、起溝避澇，田壟化作耐旱麥浪之旱田。".to_string(),
            ),
            (Terrain::EmptyFurrow, "plant_herbs" | "herb_garden") => (
                Terrain::HerbGarden,
                "移栽靈草仙葩、精細分畦，泥壟闢成藥香瀰漫之藥圃。".to_string(),
            ),
            (Terrain::EmptyFurrow, "afforest" | "plant_trees") => (
                Terrain::Forest,
                "栽植樹苗防風固土，田壟漸次鬱閉為青蔥林木。".to_string(),
            ),
            (
                Terrain::FarmField | Terrain::DryField | Terrain::HerbGarden,
                "harvest" | "clear_field",
            ) => (
                Terrain::EmptyFurrow,
                "收割豐實作物，田畦重歸平整空壟，靜候下季播種。".to_string(),
            ),
            (Terrain::EmptyFurrow, "fallow" | "abandon_field") => (
                Terrain::Plain,
                "停歇耕作任野草蔓延，田壟風化還原為原野平地。".to_string(),
            ),

            // 5. 踩踏鋪路 / 道路廢弛
            (Terrain::Plain | Terrain::Grassland, "pave_road" | "tread_path" | "build_road") => (
                Terrain::Road,
                "夯實路基鋪設平整石板，踩出一條四通八達之道途。".to_string(),
            ),
            (Terrain::Road, "break_road" | "abandon_road" | "remove_road") => (
                Terrain::Plain,
                "掘起石板任草木復生，道路風化還原為尋常土地。".to_string(),
            ),

            // 6. 百工造橋 / 橋樑毀損
            (Terrain::Water, "build_bridge") => (
                Terrain::Bridge,
                "立墩架樑、鋪設石木橋板，清波之上升起連通兩岸之飛橋。".to_string(),
            ),
            (Terrain::Bridge, "demolish_bridge" | "collapse_bridge") => (
                Terrain::Water,
                "橋樑拆除倒塌，河川水流暢通，復還為一泓碧水。".to_string(),
            ),

            // 7. 水文漲退
            (Terrain::Plain | Terrain::Grassland, "flood" | "inundate") => (
                Terrain::Swamp,
                "潮汛上漲、地勢凹陷聚水，土地浸軟化作蘆荻泥澤。".to_string(),
            ),
            (Terrain::Swamp, "drain" | "reclaim_swamp") => (
                Terrain::Plain,
                "疏浚導流、填石排澇，泥濘澤地重現為堅實乾爽之平原。".to_string(),
            ),

            // 8. 聚落營造與拆除
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_house" | "build_home",
            ) => (
                Terrain::House,
                "起基立柱、覆瓦築垣，落成一棟溫暖樸實之民宅。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_market" | "build_shop",
            ) => (
                Terrain::Market,
                "搭建貨架攤棚、懸掛商號錦旗，闢出一處繁華商貿之市集。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_workshop" | "build_blacksmith",
            ) => (
                Terrain::Workshop,
                "置爐安砧、排設風箱機關，落成一座百工鍛造之工坊。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_clinic" | "build_apothecary",
            ) => (
                Terrain::Clinic,
                "配置診療床、草木調理台與自然脈絡感應儀，落成一間脈絡診所。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_inn" | "build_tavern",
            ) => (
                Terrain::Inn,
                "結綵挑燈、陳設几榻佳釀，營造一處迎送行旅之現代行館。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_fence",
            ) => (
                Terrain::Fence,
                "深埋排樁、橫綁堅木防禦，立起阻絕侵擾之木柵圍欄。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_wall",
            ) => (
                Terrain::Wall,
                "條石錯縫、灌漿版築，壘砌起厚重巍峨之實心城牆。".to_string(),
            ),
            (
                Terrain::House
                | Terrain::Market
                | Terrain::Workshop
                | Terrain::Clinic
                | Terrain::Inn
                | Terrain::Fence
                | Terrain::Wall
                | Terrain::Blacksmith
                | Terrain::GeneralStore
                | Terrain::Farmhouse
                | Terrain::Warehouse
                | Terrain::Granary,
                "demolish" | "tear_down",
            ) => (
                Terrain::Plain,
                "棟樑傾覆、磚石清運，建物平毀為一片乾淨平地。".to_string(),
            ),

            // 9. 荒野復育 / 植被演替
            (Terrain::Plain | Terrain::Grassland, "grow_forest" | "wild_growth") => (
                Terrain::Forest,
                "草木滋長、嘉木成蔭，日光穿透繁枝形成幽謐林境。".to_string(),
            ),

            // 10. 沙石沉積 / 海蝕沖刷
            (Terrain::Water, "sediment" | "deposit_sand") => (
                Terrain::Beach,
                "潮沙堆積、水勢退讓，湧浪邊緣沉澱出細密沙礫之海灘。".to_string(),
            ),
            (Terrain::Beach, "erode" | "sea_wash") => (
                Terrain::Water,
                "波濤沖刷侵蝕岸線，沙灘隨浪散逸復沒入清澈淺水。".to_string(),
            ),
            (Terrain::Beach, "solidify_land") => (
                Terrain::Plain,
                "耐鹽草根固著泥沙，海灘化作堅實平整之沿岸草原。".to_string(),
            ),

            _ => {
                return Err(format!(
                    "地形 {:?} 不支援演變動作 '{}'",
                    old, action
                ));
            }
        };

        cell.terrain = new_terrain;
        self.dirty = true;
        Ok((old, new_terrain, desc))
    }

    /// BFS 最短路徑（跳數）；回傳包含起點與終點的座標序列。
    ///
    /// - `from == to`：回傳 `Some(vec![from])`
    /// - 起點或終點不在地圖中：回傳 `None`
    /// - 不可達：回傳 `None`
    pub fn find_path(&self, from: SquareCoord, to: SquareCoord) -> Option<Vec<SquareCoord>> {
        if from == to {
            return Some(vec![from]);
        }
        if !self.contains(from) || !self.contains(to) {
            return None;
        }

        let mut visited: HashSet<SquareCoord> = HashSet::new();
        let mut parent: HashMap<SquareCoord, SquareCoord> = HashMap::new();
        let mut queue: VecDeque<SquareCoord> = VecDeque::new();

        visited.insert(from);
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            for next in current.neighbors() {
                if !self.can_walk(current, next) || visited.contains(&next) {
                    continue;
                }
                visited.insert(next);
                parent.insert(next, current);

                if next == to {
                    // 回溯路徑：from … to
                    let mut path = vec![to];
                    let mut c = to;
                    while let Some(&p) = parent.get(&c) {
                        path.push(p);
                        c = p;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(next);
            }
        }
        None
    }

    /// 加權尋路（Dijkstra）；優先沿已開闢之「疏竹小徑」、「標記竹徑」、「竹林營火」與道路推進，回傳包含起點與終點的座標序列。
    pub fn find_travel_path(&self, from: SquareCoord, to: SquareCoord) -> Option<Vec<SquareCoord>> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        if from == to {
            return Some(vec![from]);
        }
        if !self.contains(from) || !self.contains(to) {
            return None;
        }

        let step_weight = |cell: &GridCell| -> u32 {
            if matches!(cell.terrain, Terrain::Road | Terrain::Bridge)
                || crate::grid::zhulin::is_bamboo_trail_cleared(cell)
                || crate::grid::zhulin::has_bamboo_trail_mark(cell)
                || cell.has_room_tag("zl_campfire:active")
                || cell
                    .objects
                    .iter()
                    .any(|o| o.id == "zl_bamboo_campfire" && o.quantity > 0)
            {
                10
            } else if (crate::grid::zhulin::is_bamboo_slot_cell(cell)
                && !crate::grid::zhulin::is_bamboo_trail_cleared(cell))
                || cell.terrain.move_cost() >= 1.5
            {
                24
            } else {
                14
            }
        };

        let mut dist: HashMap<SquareCoord, u32> = HashMap::new();
        let mut parent: HashMap<SquareCoord, SquareCoord> = HashMap::new();
        let mut heap: BinaryHeap<Reverse<(u32, i32, i32)>> = BinaryHeap::new();

        dist.insert(from, 0);
        heap.push(Reverse((0, from.x, from.y)));

        while let Some(Reverse((cost, cx, cy))) = heap.pop() {
            let current = SquareCoord::new(cx, cy);
            if current == to {
                let mut path = vec![to];
                let mut c = to;
                while let Some(&p) = parent.get(&c) {
                    path.push(p);
                    c = p;
                }
                path.reverse();
                return Some(path);
            }
            if dist.get(&current).is_some_and(|&best| cost > best) {
                continue;
            }
            let candidates: Vec<SquareCoord> = if let Some(fc) = self.get(current)
                && !fc.explicit_exits.is_empty()
            {
                fc.explicit_exits.iter().map(|e| e.target).collect()
            } else {
                current.neighbors().to_vec()
            };
            for next in candidates {
                if !self.can_walk(current, next) {
                    continue;
                }
                let Some(next_cell) = self.get(next) else {
                    continue;
                };
                let next_cost = cost.saturating_add(step_weight(next_cell));
                let is_better = dist.get(&next).is_none_or(|&prev| next_cost < prev);
                if is_better {
                    dist.insert(next, next_cost);
                    parent.insert(next, current);
                    heap.push(Reverse((next_cost, next.x, next.y)));
                }
            }
        }
        None
    }
}

// ─────────────────────────────────────────────────────────────────
