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

    pub fn sync_places_from_grid(
        &mut self,
        grid: &SquareGrid,
    ) -> anyhow::Result<LocationMigrationReport> {
        let pool = self
            .db_pool
            .clone()
            .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
        let report = crate::pg::world_place::sync_legacy_grid(&pool, grid)?;
        self.reload_place_graph()?;
        anyhow::ensure!(self.load_entities_from_pg(), "位置遷移後無法重載 entities");
        Ok(report)
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
        let place_id = legacy_place_id(crate::grid::SquareCoord::new(x, y));
        if self.place_graph.place(&place_id).is_some() {
            return self.set_entity_place(entity_id, &place_id);
        }
        let has_legacy_places = self
            .place_graph
            .places()
            .any(|place| place.source_kind == "legacy_grid");
        anyhow::ensure!(!has_legacy_places, "座標沒有對應正式場所：{x},{y}");
        anyhow::ensure!(
            self.entities.contains_key(entity_id),
            "實體不存在：{entity_id}"
        );
        let room_id = crate::grid::grid_room_id_from_coord(x, y);
        if crate::pg::writer::is_active() {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityPlace {
                entity_id: entity_id.to_string(),
                place_id: place_id.clone(),
                grid_x: Some(x),
                grid_y: Some(y),
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
                &place_id,
                Some(x),
                Some(y),
                &room_id,
            )?;
        }
        let entity = self
            .entities
            .get_mut(entity_id)
            .expect("entity existence checked before committed location write");
        entity.place_id = Some(place_id);
        entity.grid_x = Some(x);
        entity.grid_y = Some(y);
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
                place_id: place.id.clone(),
                grid_x,
                grid_y,
                room_id: room_id.clone(),
            });
        } else {
            let pool = self
                .db_pool
                .clone()
                .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;

            crate::pg::world_place::set_entity_place(
                &pool, entity_id, &place.id, grid_x, grid_y, &room_id,
            )?;
        }

        let entity = self
            .entities
            .get_mut(entity_id)
            .expect("entity existence checked before committed location write");
        entity.place_id = Some(place.id.clone());
        entity.grid_x = grid_x;
        entity.grid_y = grid_y;
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
                place_id: place.id.clone(),
                grid_x,
                grid_y,
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
