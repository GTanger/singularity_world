//! Store 的唯一位置入口；玩家與居民都沿 `world_routes` 行動。

use crate::grid::SquareGrid;
use crate::pg::world_place::LocationMigrationReport;
use crate::world::journey::EntityJourney;
use crate::world::place::{PlaceGraph, WorldPlace, WorldRoute, legacy_place_id};

use super::Store;

#[derive(Debug, Clone)]
pub struct PlaceMove {
    pub from_place_id: String,
    pub to_place_id: String,
    pub route: WorldRoute,
    pub action_id: String,
}

impl Store {
    pub fn reload_place_graph(&mut self) -> anyhow::Result<()> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
        self.place_graph = crate::pg::world_place::load_graph(pool)?;
        Ok(())
    }

    /// 退役相容空操作：不再重建 `legacy_grid`，僅確保其場所與路線維持 `retired`。
    pub fn sync_places_from_grid(
        &mut self,
        _grid: &SquareGrid,
    ) -> anyhow::Result<LocationMigrationReport> {
        let report = LocationMigrationReport {
            legacy_places: 0,
            legacy_routes: 0,
            migrated_entities: 0,
            unresolved_entities: 0,
            mirrored_entities: 0,
        };
        let Some(pool) = self.db_pool.clone() else {
            return Ok(report);
        };
        let mut conn = pool.get()?;
        conn.batch_execute(
            "UPDATE world_places SET status='retired',traversable=FALSE
             WHERE source_kind='legacy_grid' AND (status<>'retired' OR traversable);
             UPDATE world_routes SET status='retired'
             WHERE source_kind='legacy_grid' AND status<>'retired'",
        )?;
        drop(conn);
        self.reload_place_graph()?;
        Ok(report)
    }

    /// 僅當場所圖實際存在該格的舊式 id 時回傳；`legacy_grid` 退役後恆為 `None`。
    pub fn existing_legacy_place_id(&self, x: i32, y: i32) -> Option<String> {
        let id = legacy_place_id(crate::grid::SquareCoord::new(x, y));
        self.place_graph
            .place(&id)
            .filter(|p| p.traversable && p.status != "retired")
            .map(|_| id)
    }

    pub fn get_entity_place_id(&self, entity_id: &str) -> Option<String> {
        self.entities
            .get(entity_id)
            .and_then(|entity| entity.place_id.clone())
    }

    pub fn get_place(&self, place_id: &str) -> Option<&WorldPlace> {
        self.place_graph.place(place_id)
    }

    pub fn place_graph_snapshot(&self) -> PlaceGraph {
        self.place_graph.clone()
    }

    pub fn set_entity_place(&mut self, entity_id: &str, place_id: &str) -> anyhow::Result<()> {
        let place = self
            .place_graph
            .place(place_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("場所不存在：{place_id}"))?;
        anyhow::ensure!(place.traversable, "場所不可直接進入：{place_id}");
        self.apply_entity_place(entity_id, &place)
    }

    pub fn move_entity_direction(
        &mut self,
        entity_id: &str,
        direction: &str,
    ) -> anyhow::Result<Option<PlaceMove>> {
        let Some(from) = self.get_entity_place_id(entity_id) else {
            return Ok(None);
        };
        let Some(route) = self
            .place_graph
            .route_in_direction(&from, direction)
            .cloned()
        else {
            return Ok(None);
        };
        Ok(Some(
            self.apply_entity_move(entity_id, &from, &route, None)?,
        ))
    }

    pub fn move_entity_to_adjacent_place(
        &mut self,
        entity_id: &str,
        to_place_id: &str,
    ) -> anyhow::Result<Option<PlaceMove>> {
        let Some(from) = self.get_entity_place_id(entity_id) else {
            return Ok(None);
        };
        let Some(route) = self.place_graph.route_to(&from, to_place_id).cloned() else {
            return Ok(None);
        };
        Ok(Some(
            self.apply_entity_move(entity_id, &from, &route, None)?,
        ))
    }

    pub fn move_entity_to_adjacent_place_with_journey(
        &mut self,
        entity_id: &str,
        to_place_id: &str,
        journey: &EntityJourney,
    ) -> anyhow::Result<Option<PlaceMove>> {
        let Some(from) = self.get_entity_place_id(entity_id) else {
            return Ok(None);
        };
        let Some(route) = self.place_graph.route_to(&from, to_place_id).cloned() else {
            return Ok(None);
        };
        Ok(Some(self.apply_entity_move(
            entity_id,
            &from,
            &route,
            Some(journey),
        )?))
    }

    pub(super) fn set_entity_grid_compat(
        &mut self,
        entity_id: &str,
        x: i32,
        y: i32,
    ) -> anyhow::Result<()> {
        self.set_entity_grid_z(entity_id, x, y, 0)
    }

    /// 含樓層的格網定位；`legacy_grid` 已退役時 `place_id` 一律為 `None`。
    pub fn set_entity_grid_z(
        &mut self,
        entity_id: &str,
        x: i32,
        y: i32,
        z: i16,
    ) -> anyhow::Result<()> {
        let place_id = legacy_place_id(crate::grid::SquareCoord::new(x, y));
        if z == 0
            && self
                .place_graph
                .place(&place_id)
                .is_some_and(|place| place.traversable)
        {
            return self.set_entity_place(entity_id, &place_id);
        }
        anyhow::ensure!(
            self.entities.contains_key(entity_id),
            "實體不存在：{entity_id}"
        );
        let room_id = if z == 0 {
            format!("grid:{x}:{y}")
        } else {
            format!("grid:{x}:{y}:{z}")
        };
        if crate::pg::writer::is_active() {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityPlace {
                entity_id: entity_id.to_string(),
                place_id: None,
                grid_x: Some(x),
                grid_y: Some(y),
                grid_z: z,
                room_id: room_id.clone(),
            });
        } else {
            let pool = self
                .db_pool
                .clone()
                .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
            crate::pg::world_place::set_entity_place(
                &pool,
                entity_id,
                None,
                Some(x),
                Some(y),
                z,
                &room_id,
            )?;
        }
        let entity = self
            .entities
            .get_mut(entity_id)
            .ok_or_else(|| anyhow::anyhow!("實體不存在：{entity_id}"))?;
        entity.place_id = None;
        entity.grid_x = Some(x);
        entity.grid_y = Some(y);
        entity.grid_z = z;
        if entity.kind != "npc" && entity.current_activity.contains("於客舍內閉門靜養") {
            entity.current_activity = "漫步穿行於街坊之間".to_string();
        }
        self.entity_rooms.insert(entity_id.to_string(), room_id);
        Ok(())
    }

    fn apply_entity_place(&mut self, entity_id: &str, place: &WorldPlace) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.entities.contains_key(entity_id),
            "實體不存在：{entity_id}"
        );
        let coord = place.legacy_coord();
        let grid_x = coord.map(|coord| coord.x);
        let grid_y = coord.map(|coord| coord.y);
        let room_id = place.compatibility_room_id();
        if crate::pg::writer::is_active() {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityPlace {
                entity_id: entity_id.to_string(),
                place_id: Some(place.id.clone()),
                grid_x,
                grid_y,
                grid_z: 0,
                room_id: room_id.clone(),
            });
        } else {
            let pool = self
                .db_pool
                .clone()
                .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;

            crate::pg::world_place::set_entity_place(
                &pool,
                entity_id,
                Some(&place.id),
                grid_x,
                grid_y,
                0,
                &room_id,
            )?;
        }

        let entity = self
            .entities
            .get_mut(entity_id)
            .ok_or_else(|| anyhow::anyhow!("實體不存在：{entity_id}"))?;
        entity.place_id = Some(place.id.clone());
        entity.grid_x = grid_x;
        entity.grid_y = grid_y;
        entity.grid_z = 0;
        if entity.kind != "npc" && entity.current_activity.contains("於客舍內閉門靜養") {
            entity.current_activity = "漫步穿行於街坊之間".to_string();
        }
        self.entity_rooms.insert(entity_id.to_string(), room_id);
        Ok(())
    }

    fn apply_entity_move(
        &mut self,
        entity_id: &str,
        from_place_id: &str,
        route: &WorldRoute,
        journey: Option<&EntityJourney>,
    ) -> anyhow::Result<PlaceMove> {
        let place = self
            .place_graph
            .place(&route.to_place_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("場所不存在：{}", route.to_place_id))?;
        anyhow::ensure!(place.traversable, "場所不可直接進入：{}", place.id);
        let coord = place.legacy_coord();
        let grid_x = coord.map(|coord| coord.x);
        let grid_y = coord.map(|coord| coord.y);
        let room_id = place.compatibility_room_id();
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let action_id = if crate::pg::writer::is_active() {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityPlace {
                entity_id: entity_id.to_string(),
                place_id: Some(place.id.clone()),
                grid_x,
                grid_y,
                grid_z: 0,
                room_id: room_id.clone(),
            });
            if let Some(j) = journey {
                crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityJourney(j.clone()));
            }
            format!("move:{entity_id}:{now_unix}")
        } else {
            let pool = self
                .db_pool
                .clone()
                .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
            crate::pg::world_place::move_entity_place(
                &pool,
                entity_id,
                from_place_id,
                &place.id,
                grid_x,
                grid_y,
                &room_id,
                route,
                journey,
                now_unix,
            )?
        };
        let entity = self
            .entities
            .get_mut(entity_id)
            .ok_or_else(|| anyhow::anyhow!("實體不存在：{entity_id}"))?;
        entity.place_id = Some(place.id.clone());
        entity.grid_x = grid_x;
        entity.grid_y = grid_y;
        self.entity_rooms.insert(entity_id.to_string(), room_id);
        Ok(PlaceMove {
            from_place_id: from_place_id.to_string(),
            to_place_id: place.id,
            route: route.clone(),
            action_id,
        })
    }
}
