//! 白塔半島正式執行世界的 PostgreSQL 採用、位置遷移與回復。
//!
//! Azgaar 作者包永遠保留在檔案層；此模組只把已驗證的 runtime seed 寫成
//! 執行主資料。遷移前先保存每一個人物的位置與旅程，舊格只退役、不刪除。

use std::collections::HashMap;

use r2d2_postgres::postgres::{GenericClient, Transaction};
use serde::{Deserialize, Serialize};

use crate::store::sql::DbPool;
use crate::world::physical_geography::NormalizedPoint;
use crate::world::runtime_world::{
    RuntimeAreaProfile, WHITE_TOWER_RUNTIME_ID, WhiteTowerRuntimeWorld,
    builtin_white_tower_runtime_world,
};

const ACTIVE_STATUS: &str = "active";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeMigrationReport {
    pub runtime_id: String,
    pub runtime_places: usize,
    pub runtime_routes: usize,
    pub source_layers: usize,
    pub resource_sources: usize,
    pub economy_goods: i64,
    pub economy_markets: i64,
    pub economy_quotes: i64,
    pub economy_trade_evidence: i64,
    pub economy_work_sites: i64,
    pub economy_upstream_supply_profiles: i64,
    pub economy_producer_sites: i64,
    pub economy_operational_inventory: i64,
    pub economy_flow_lanes: i64,
    pub legacy_places_retired: u64,
    pub legacy_routes_retired: u64,
    pub location_backups: i64,
    pub entities_migrated: u64,
    pub journeys_migrated: u64,
    pub unresolved_entities: i64,
    pub invalid_journeys: usize,
    pub codex_from_place_id: Option<String>,
    pub codex_to_place_id: Option<String>,
    pub already_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeRollbackReport {
    pub runtime_id: String,
    pub entities_restored: u64,
    pub journeys_restored: u64,
    pub legacy_places_reactivated: u64,
    pub legacy_routes_reactivated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeDbStatus {
    pub runtime_id: String,
    pub status: String,
    pub runtime_places: i64,
    pub runtime_routes: i64,
    pub source_layers: i64,
    pub resource_sources: i64,
    pub economy_goods: i64,
    pub economy_markets: i64,
    pub economy_quotes: i64,
    pub economy_trade_evidence: i64,
    pub economy_blocked_human_goods: i64,
    pub economy_work_sites: i64,
    pub economy_upstream_supply_profiles: i64,
    pub economy_producer_sites: i64,
    pub economy_operational_inventory: i64,
    pub economy_flow_lanes: i64,
    pub entities_total: i64,
    pub entities_in_runtime: i64,
    pub entity_rooms_aligned: i64,
    pub location_backups: i64,
    pub journeys_total: i64,
    pub invalid_journeys: usize,
    pub active_legacy_places: i64,
    pub active_legacy_routes: i64,
    pub codex_place_id: Option<String>,
}

#[derive(Debug, Clone)]
struct LegacyPlace {
    id: String,
    x: i32,
    y: i32,
}

/// 建立正式場所／路線／來源目錄並把尚未進入白塔世界的人物原子遷入。
///
/// 可重複呼叫：已在 runtime 的人物不會被重置，舊備份也不會被覆寫。
pub fn ensure_and_migrate(pool: &DbPool, now_unix: i64) -> anyhow::Result<RuntimeMigrationReport> {
    let world = builtin_white_tower_runtime_world()?;
    super::world_journey::ensure_schema(pool)?;
    let mut conn = pool.get()?;
    super::world_place::ensure_schema(&mut conn)?;
    ensure_schema(&mut *conn)?;

    let already_active = conn
        .query_opt(
            "SELECT status FROM world_runtime_versions WHERE id=$1",
            &[&WHITE_TOWER_RUNTIME_ID],
        )?
        .is_some_and(|row| row.get::<_, String>(0) == ACTIVE_STATUS);
    let codex_identity_before = codex_identity(&mut *conn)?;
    let codex_from_place_id = entity_place(&mut *conn, "codex")?;

    let mut tx = conn.transaction()?;
    upsert_runtime_seed(&mut tx, &world, now_unix)?;
    let economy = super::world_economy::upsert_seed(&mut tx, now_unix)?;
    let legacy_places = load_legacy_places(&mut tx)?;
    let migration_map =
        build_legacy_mapping(&legacy_places, &world.area_profiles, &world.spawn_place_id);
    upsert_migration_map(&mut tx, &migration_map, now_unix)?;
    capture_location_backups(&mut tx, now_unix)?;

    let (entities_migrated, entity_targets) = migrate_entities(&mut tx, &world, &migration_map)?;
    let journeys_migrated =
        migrate_journeys(&mut tx, &world, &migration_map, &entity_targets, now_unix)?;
    let legacy_routes_retired = tx.execute(
        "UPDATE world_routes SET status='retired' WHERE source_kind='legacy_grid' AND status<>'retired'",
        &[],
    )?;
    let legacy_places_retired = tx.execute(
        "UPDATE world_places SET status='retired',traversable=FALSE
         WHERE source_kind='legacy_grid' AND (status<>'retired' OR traversable)",
        &[],
    )?;

    let unresolved_entities: i64 = tx
        .query_one(
            "SELECT COUNT(*)::bigint FROM entities e
             LEFT JOIN world_places p ON p.id=e.place_id
             WHERE NOT (
                 (e.place_id IS NOT NULL AND p.source_kind='azgaar_runtime' AND p.status='active' AND p.traversable AND e.grid_x IS NULL AND e.grid_y IS NULL)
                 OR (e.place_id IS NULL AND e.grid_x IS NOT NULL AND e.grid_y IS NOT NULL)
             )",
            &[],
        )?
        .get(0);
    anyhow::ensure!(
        unresolved_entities == 0,
        "仍有 {unresolved_entities} 個人物未完整遷入白塔半島"
    );
    let journey_validation = validate_journeys(&mut tx, &world)?;
    anyhow::ensure!(
        journey_validation.invalid == 0,
        "仍有 {} 條旅程引用無效場所或路線：{}",
        journey_validation.invalid,
        journey_validation.examples.join("；")
    );

    let location_backups: i64 = tx
        .query_one(
            "SELECT COUNT(*)::bigint FROM world_location_migration_backups WHERE runtime_id=$1",
            &[&WHITE_TOWER_RUNTIME_ID],
        )?
        .get(0);
    let codex_to_place_id = entity_place(&mut tx, "codex")?;
    let report = RuntimeMigrationReport {
        runtime_id: WHITE_TOWER_RUNTIME_ID.to_string(),
        runtime_places: world.places.len(),
        runtime_routes: world.routes.len(),
        source_layers: world.source_layers.len(),
        resource_sources: world.resources.len(),
        economy_goods: economy.goods,
        economy_markets: economy.markets,
        economy_quotes: economy.quotes,
        economy_trade_evidence: economy.trade_evidence,
        economy_work_sites: economy.work_sites,
        economy_upstream_supply_profiles: economy.upstream_supply_profiles,
        economy_producer_sites: economy.producer_sites,
        economy_operational_inventory: economy.operational_inventory,
        economy_flow_lanes: economy.operational_flow_lanes,
        legacy_places_retired,
        legacy_routes_retired,
        location_backups,
        entities_migrated,
        journeys_migrated,
        unresolved_entities,
        invalid_journeys: journey_validation.invalid,
        codex_from_place_id,
        codex_to_place_id,
        already_active,
    };
    let receipt = serde_json::to_string(&report)?;
    tx.execute(
        "INSERT INTO world_runtime_versions
         (id,status,source_package_id,package_manifest_sha256,full_json_sha256,
          physical_geography_sha256,applied_at,rolled_back_at,receipt)
         VALUES ($1,'active',$2,$3,$4,$5,$6,NULL,$7)
         ON CONFLICT(id) DO UPDATE SET status='active',source_package_id=EXCLUDED.source_package_id,
         package_manifest_sha256=EXCLUDED.package_manifest_sha256,
         full_json_sha256=EXCLUDED.full_json_sha256,
         physical_geography_sha256=EXCLUDED.physical_geography_sha256,
         applied_at=EXCLUDED.applied_at,rolled_back_at=NULL,receipt=EXCLUDED.receipt",
        &[
            &WHITE_TOWER_RUNTIME_ID,
            &world.source.package_id,
            &world.source.package_manifest_sha256,
            &world.source.full_json_sha256,
            &world.source.physical_geography_sha256,
            &now_unix,
            &receipt,
        ],
    )?;
    let codex_identity_after = codex_identity(&mut tx)?;
    anyhow::ensure!(
        codex_identity_before == codex_identity_after,
        "codex 的非位置身分資料在遷移中被改動"
    );
    tx.commit()?;
    Ok(report)
}

/// 將遷移前的位置與旅程快照還原；runtime 種子與作者來源目錄仍保留。
pub fn rollback(pool: &DbPool, now_unix: i64) -> anyhow::Result<RuntimeRollbackReport> {
    let mut conn = pool.get()?;
    ensure_schema(&mut *conn)?;
    let codex_identity_before = codex_identity(&mut *conn)?;
    let mut tx = conn.transaction()?;
    let entities_restored = tx.execute(
        "UPDATE entities e SET place_id=b.old_place_id,grid_x=b.old_grid_x,grid_y=b.old_grid_y
         FROM world_location_migration_backups b
         WHERE b.runtime_id=$1 AND b.entity_id=e.id",
        &[&WHITE_TOWER_RUNTIME_ID],
    )?;
    tx.execute(
        "INSERT INTO entity_rooms(entity_id,room_id)
         SELECT entity_id,COALESCE(old_room_id,'') FROM world_location_migration_backups
         WHERE runtime_id=$1
         ON CONFLICT(entity_id) DO UPDATE SET room_id=EXCLUDED.room_id",
        &[&WHITE_TOWER_RUNTIME_ID],
    )?;
    let journeys_restored = tx.execute(
        "INSERT INTO entity_journeys
         (entity_id,reason,destination_place_id,planned_path,consequence,updated_at)
         SELECT entity_id,COALESCE(old_journey_reason,'idle'),old_destination_place_id,
                COALESCE(old_planned_path,'[]'),COALESCE(old_consequence,''),
                COALESCE(old_journey_updated_at,0)
         FROM world_location_migration_backups WHERE runtime_id=$1 AND old_journey_reason IS NOT NULL
         ON CONFLICT(entity_id) DO UPDATE SET reason=EXCLUDED.reason,
         destination_place_id=EXCLUDED.destination_place_id,planned_path=EXCLUDED.planned_path,
         consequence=EXCLUDED.consequence,updated_at=EXCLUDED.updated_at",
        &[&WHITE_TOWER_RUNTIME_ID],
    )?;
    tx.execute(
        "DELETE FROM entity_journeys j USING world_location_migration_backups b
         WHERE b.runtime_id=$1 AND b.entity_id=j.entity_id AND b.old_journey_reason IS NULL",
        &[&WHITE_TOWER_RUNTIME_ID],
    )?;
    let legacy_places_reactivated = tx.execute(
        "UPDATE world_places SET status='material',traversable=TRUE WHERE source_kind='legacy_grid'",
        &[],
    )?;
    let legacy_routes_reactivated = tx.execute(
        "UPDATE world_routes SET status='active' WHERE source_kind='legacy_grid'",
        &[],
    )?;
    tx.execute(
        "UPDATE world_places SET status='retained_runtime',traversable=FALSE
         WHERE source_kind='azgaar_runtime'",
        &[],
    )?;
    tx.execute(
        "UPDATE world_routes SET status='retained_runtime' WHERE source_kind='azgaar_runtime'",
        &[],
    )?;
    tx.execute(
        "UPDATE world_runtime_versions SET status='rolled_back',rolled_back_at=$2 WHERE id=$1",
        &[&WHITE_TOWER_RUNTIME_ID, &now_unix],
    )?;
    let codex_identity_after = codex_identity(&mut tx)?;
    anyhow::ensure!(
        codex_identity_before == codex_identity_after,
        "codex 的非位置身分資料在回復中被改動"
    );
    tx.commit()?;
    Ok(RuntimeRollbackReport {
        runtime_id: WHITE_TOWER_RUNTIME_ID.to_string(),
        entities_restored,
        journeys_restored,
        legacy_places_reactivated,
        legacy_routes_reactivated,
    })
}

/// 直接從 PostgreSQL 驗證目前是否真由完整白塔半島 runtime 掌權。
pub fn check_active(pool: &DbPool) -> anyhow::Result<RuntimeDbStatus> {
    let world = builtin_white_tower_runtime_world()?;
    let economy = crate::world::author_economy::builtin_white_tower_author_economy()?;
    let upstream_expected = super::world_supply::seed_counts(&economy)?;
    let mut conn = pool.get()?;
    super::world_place::ensure_schema(&mut conn)?;
    ensure_schema(&mut *conn)?;
    let status = conn
        .query_opt(
            "SELECT status FROM world_runtime_versions WHERE id=$1",
            &[&WHITE_TOWER_RUNTIME_ID],
        )?
        .map(|row| row.get(0))
        .unwrap_or_default();
    let runtime_places = count(
        &mut *conn,
        "SELECT COUNT(*)::bigint FROM world_places WHERE source_kind='azgaar_runtime' AND status='active' AND traversable",
    )?;
    let runtime_routes = count(
        &mut *conn,
        "SELECT COUNT(*)::bigint FROM world_routes WHERE source_kind='azgaar_runtime' AND status='active'",
    )?;
    let source_layers = count(
        &mut *conn,
        "SELECT COUNT(*)::bigint FROM world_source_layers WHERE runtime_id='white_tower_runtime_v1'",
    )?;
    let resource_sources = count(
        &mut *conn,
        "SELECT COUNT(*)::bigint FROM world_resource_sources WHERE runtime_id='white_tower_runtime_v1'",
    )?;
    let economy_status = super::world_economy::status(&mut *conn)?;
    let entities_total = count(&mut *conn, "SELECT COUNT(*)::bigint FROM entities")?;
    let entities_in_runtime = count(
        &mut *conn,
        "SELECT COUNT(*)::bigint FROM entities e LEFT JOIN world_places p ON p.id=e.place_id
         WHERE (p.source_kind='azgaar_runtime' AND p.status='active' AND p.traversable AND e.grid_x IS NULL AND e.grid_y IS NULL)
            OR (e.place_id IS NULL AND e.grid_x IS NOT NULL AND e.grid_y IS NOT NULL)",
    )?;
    let entity_rooms_aligned = count(
        &mut *conn,
        "SELECT COUNT(*)::bigint FROM entities e JOIN entity_rooms er ON er.entity_id=e.id
         WHERE er.room_id=e.place_id OR (e.place_id IS NULL AND er.room_id LIKE 'grid:%')",
    )?;
    let location_backups = count(
        &mut *conn,
        "SELECT COUNT(*)::bigint FROM world_location_migration_backups
         WHERE runtime_id='white_tower_runtime_v1'",
    )?;
    let _ = conn.execute(
        "DELETE FROM entity_journeys WHERE entity_id IN (SELECT id FROM entities WHERE place_id IS NULL) OR entity_id NOT IN (SELECT id FROM entities)",
        &[],
    );
    let journeys_total = count(&mut *conn, "SELECT COUNT(*)::bigint FROM entity_journeys")?;
    let journey_validation = validate_journeys(&mut *conn, &world)?;
    let invalid_journey_examples = journey_validation.examples.join("；");
    let active_legacy_places = count(
        &mut *conn,
        "SELECT COUNT(*)::bigint FROM world_places WHERE source_kind='legacy_grid'
         AND status<>'retired' AND traversable",
    )?;
    let active_legacy_routes = count(
        &mut *conn,
        "SELECT COUNT(*)::bigint FROM world_routes WHERE source_kind='legacy_grid' AND status='active'",
    )?;
    let codex_place_id = entity_place(&mut *conn, "codex")?;
    let result = RuntimeDbStatus {
        runtime_id: WHITE_TOWER_RUNTIME_ID.to_string(),
        status,
        runtime_places,
        runtime_routes,
        source_layers,
        resource_sources,
        economy_goods: economy_status.goods,
        economy_markets: economy_status.markets,
        economy_quotes: economy_status.quotes,
        economy_trade_evidence: economy_status.trade_evidence,
        economy_blocked_human_goods: economy_status.blocked_human_goods,
        economy_work_sites: economy_status.work_sites,
        economy_upstream_supply_profiles: economy_status.upstream_supply_profiles,
        economy_producer_sites: economy_status.producer_sites,
        economy_operational_inventory: economy_status.operational_inventory,
        economy_flow_lanes: economy_status.operational_flow_lanes,
        entities_total,
        entities_in_runtime,
        entity_rooms_aligned,
        location_backups,
        journeys_total,
        invalid_journeys: journey_validation.invalid,
        active_legacy_places,
        active_legacy_routes,
        codex_place_id,
    };
    anyhow::ensure!(result.status == ACTIVE_STATUS, "白塔 runtime 尚未啟用");
    anyhow::ensure!(
        result.runtime_places == world.places.len() as i64,
        "runtime 場所不完整"
    );
    anyhow::ensure!(
        result.runtime_routes == world.routes.len() as i64,
        "runtime 路線不完整"
    );
    anyhow::ensure!(
        result.source_layers == world.source_layers.len() as i64,
        "來源分類不完整"
    );
    anyhow::ensure!(
        result.resource_sources == world.resources.len() as i64,
        "資源來源不完整"
    );
    anyhow::ensure!(
        result.economy_goods == economy.goods.len() as i64,
        "商品證據不完整"
    );
    anyhow::ensure!(
        result.economy_markets == economy.markets.len() as i64,
        "市場證據不完整"
    );
    anyhow::ensure!(
        result.economy_quotes == economy.coverage.market_quotes_accounted as i64,
        "市場報價證據不完整"
    );
    anyhow::ensure!(
        result.economy_trade_evidence == economy.trade_evidence.len() as i64,
        "來源交易證據不完整"
    );
    anyhow::ensure!(
        result.economy_blocked_human_goods == 1,
        "人口販運來源未被明確阻擋"
    );
    anyhow::ensure!(
        result.economy_work_sites
            == economy.markets.len() as i64 + upstream_expected.producer_sites,
        "市場與地方供應工作場所未完整接通"
    );
    anyhow::ensure!(
        result.economy_upstream_supply_profiles == upstream_expected.profiles,
        "上游供應證據未完整接通"
    );
    anyhow::ensure!(
        result.economy_producer_sites == upstream_expected.producer_sites,
        "地方供應場所未完整接通"
    );
    anyhow::ensure!(
        result.economy_operational_inventory
            == (economy.markets.len() * economy.coverage.goods_runtime_allowed) as i64,
        "實際市場庫存未覆蓋全部可用商品"
    );
    anyhow::ensure!(result.economy_flow_lanes == 478, "市場物流流向未完整接通");
    anyhow::ensure!(
        result.entities_in_runtime == result.entities_total,
        "仍有人物不在白塔 runtime"
    );
    anyhow::ensure!(
        result.entity_rooms_aligned == result.entities_total,
        "人物房間鏡像未對齊 place_id"
    );
    anyhow::ensure!(
        result.location_backups == result.entities_total,
        "人物遷移備份不完整"
    );
    anyhow::ensure!(
        result.invalid_journeys == 0,
        "仍有無效旅程：{invalid_journey_examples}"
    );
    anyhow::ensure!(result.active_legacy_places == 0, "舊格場所仍在掌權");
    anyhow::ensure!(result.active_legacy_routes == 0, "舊格路線仍在掌權");
    anyhow::ensure!(
        result
            .codex_place_id
            .as_deref()
            .is_some_and(|id| world.places.iter().any(|place| place.id == id)),
        "codex 不在白塔半島"
    );
    Ok(result)
}

/// runtime 啟用後新建立的人物，也先留下「進入白塔前」的空位置快照。
pub fn capture_entity_backup_if_missing(
    pool: &DbPool,
    entity_id: &str,
    now_unix: i64,
) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    ensure_schema(&mut *conn)?;
    conn.execute(
        "INSERT INTO world_location_migration_backups
         (runtime_id,entity_id,old_place_id,old_grid_x,old_grid_y,old_room_id,
          old_journey_reason,old_destination_place_id,old_planned_path,old_consequence,
          old_journey_updated_at,identity_snapshot,captured_at)
         SELECT $1,e.id,e.place_id,e.grid_x,e.grid_y,er.room_id,j.reason,j.destination_place_id,
                j.planned_path,j.consequence,j.updated_at,
                (to_jsonb(e)-'place_id'-'grid_x'-'grid_y')::text,$3
         FROM entities e
         LEFT JOIN entity_rooms er ON er.entity_id=e.id
         LEFT JOIN entity_journeys j ON j.entity_id=e.id
         WHERE e.id=$2
         ON CONFLICT(runtime_id,entity_id) DO NOTHING",
        &[&WHITE_TOWER_RUNTIME_ID, &entity_id, &now_unix],
    )?;
    Ok(())
}

pub fn ensure_schema(client: &mut impl GenericClient) -> anyhow::Result<()> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS world_runtime_versions (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            source_package_id TEXT NOT NULL,
            package_manifest_sha256 TEXT NOT NULL,
            full_json_sha256 TEXT NOT NULL,
            physical_geography_sha256 TEXT NOT NULL,
            applied_at BIGINT NOT NULL,
            rolled_back_at BIGINT,
            receipt TEXT NOT NULL DEFAULT '{}'
        );
        CREATE TABLE IF NOT EXISTS world_source_layers (
            runtime_id TEXT NOT NULL,
            layer TEXT NOT NULL,
            source_count BIGINT NOT NULL,
            disposition TEXT NOT NULL,
            runtime_use TEXT NOT NULL,
            PRIMARY KEY(runtime_id,layer)
        );
        CREATE TABLE IF NOT EXISTS world_place_profiles (
            runtime_id TEXT NOT NULL,
            place_id TEXT NOT NULL,
            centroid_x DOUBLE PRECISION NOT NULL,
            centroid_y DOUBLE PRECISION NOT NULL,
            terrain_class TEXT NOT NULL,
            elevation_band TEXT NOT NULL,
            profile TEXT NOT NULL,
            PRIMARY KEY(runtime_id,place_id)
        );
        CREATE TABLE IF NOT EXISTS world_place_migration_map (
            runtime_id TEXT NOT NULL,
            source_place_id TEXT NOT NULL,
            target_place_id TEXT NOT NULL,
            mapped_at BIGINT NOT NULL,
            PRIMARY KEY(runtime_id,source_place_id)
        );
        CREATE TABLE IF NOT EXISTS world_location_migration_backups (
            runtime_id TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            old_place_id TEXT,
            old_grid_x INTEGER,
            old_grid_y INTEGER,
            old_room_id TEXT,
            old_journey_reason TEXT,
            old_destination_place_id TEXT,
            old_planned_path TEXT,
            old_consequence TEXT,
            old_journey_updated_at BIGINT,
            identity_snapshot TEXT NOT NULL,
            captured_at BIGINT NOT NULL,
            PRIMARY KEY(runtime_id,entity_id)
        );",
    )?;
    super::world_resource::ensure_schema(client)?;
    super::world_economy::ensure_schema(client)?;
    super::world_trade::ensure_schema(client)?;
    super::world_work::ensure_schema(client)
}

fn upsert_runtime_seed(
    tx: &mut Transaction<'_>,
    world: &WhiteTowerRuntimeWorld,
    now_unix: i64,
) -> anyhow::Result<()> {
    for place in &world.places {
        let tags = serde_json::to_string(&place.tags)?;
        tx.execute(
            "INSERT INTO world_places
             (id,world_id,topology_node_ref,parent_place_id,name,kind,scale,canon,status,
              traversable,source_kind,zone,tags,description,legacy_grid_x,legacy_grid_y)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'active',TRUE,$9,$10,$11,$12,NULL,NULL)
             ON CONFLICT(id) DO UPDATE SET world_id=EXCLUDED.world_id,
             topology_node_ref=EXCLUDED.topology_node_ref,parent_place_id=EXCLUDED.parent_place_id,
             name=EXCLUDED.name,kind=EXCLUDED.kind,scale=EXCLUDED.scale,canon=EXCLUDED.canon,
             status='active',traversable=TRUE,source_kind=EXCLUDED.source_kind,zone=EXCLUDED.zone,
             tags=EXCLUDED.tags,description=EXCLUDED.description,legacy_grid_x=NULL,legacy_grid_y=NULL",
            &[
                &place.id,
                &place.world_id,
                &place.topology_node_ref,
                &place.parent_place_id,
                &place.name,
                &place.kind,
                &place.scale,
                &place.canon,
                &place.source_kind,
                &place.zone,
                &tags,
                &place.description,
            ],
        )?;
    }
    for route in &world.routes {
        tx.execute(
            "INSERT INTO world_routes
             (id,from_place_id,direction,to_place_id,mode,travel_cost,canon,status,source_kind)
             VALUES ($1,$2,$3,$4,$5,$6,$7,'active',$8)
             ON CONFLICT(id) DO UPDATE SET from_place_id=EXCLUDED.from_place_id,
             direction=EXCLUDED.direction,to_place_id=EXCLUDED.to_place_id,mode=EXCLUDED.mode,
             travel_cost=EXCLUDED.travel_cost,canon=EXCLUDED.canon,status='active',
             source_kind=EXCLUDED.source_kind",
            &[
                &route.id,
                &route.from_place_id,
                &route.direction,
                &route.to_place_id,
                &route.mode,
                &route.travel_cost,
                &route.canon,
                &route.source_kind,
            ],
        )?;
    }
    for profile in &world.area_profiles {
        let raw = serde_json::to_string(profile)?;
        tx.execute(
            "INSERT INTO world_place_profiles
             (runtime_id,place_id,centroid_x,centroid_y,terrain_class,elevation_band,profile)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT(runtime_id,place_id) DO UPDATE SET centroid_x=EXCLUDED.centroid_x,
             centroid_y=EXCLUDED.centroid_y,terrain_class=EXCLUDED.terrain_class,
             elevation_band=EXCLUDED.elevation_band,profile=EXCLUDED.profile",
            &[
                &WHITE_TOWER_RUNTIME_ID,
                &profile.place_id,
                &profile.centroid.x,
                &profile.centroid.y,
                &profile.terrain_class,
                &profile.elevation_band,
                &raw,
            ],
        )?;
    }
    for layer in &world.source_layers {
        let count = i64::try_from(layer.source_count)?;
        tx.execute(
            "INSERT INTO world_source_layers(runtime_id,layer,source_count,disposition,runtime_use)
             VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT(runtime_id,layer) DO UPDATE SET source_count=EXCLUDED.source_count,
             disposition=EXCLUDED.disposition,runtime_use=EXCLUDED.runtime_use",
            &[
                &WHITE_TOWER_RUNTIME_ID,
                &layer.layer,
                &count,
                &layer.disposition,
                &layer.runtime_use,
            ],
        )?;
    }
    super::world_resource::upsert_sources(tx, world, now_unix)?;
    Ok(())
}

fn load_legacy_places(client: &mut impl GenericClient) -> anyhow::Result<Vec<LegacyPlace>> {
    Ok(client
        .query(
            "SELECT id,legacy_grid_x,legacy_grid_y FROM world_places
             WHERE source_kind='legacy_grid' AND legacy_grid_x IS NOT NULL AND legacy_grid_y IS NOT NULL
             ORDER BY id",
            &[],
        )?
        .into_iter()
        .map(|row| LegacyPlace {
            id: row.get(0),
            x: row.get(1),
            y: row.get(2),
        })
        .collect())
}

fn build_legacy_mapping(
    places: &[LegacyPlace],
    profiles: &[RuntimeAreaProfile],
    spawn_place_id: &str,
) -> HashMap<String, String> {
    let Some(min_x) = places.iter().map(|place| place.x).min() else {
        return HashMap::new();
    };
    let max_x = places.iter().map(|place| place.x).max().unwrap_or(min_x);
    let min_y = places.iter().map(|place| place.y).min().unwrap_or(0);
    let max_y = places.iter().map(|place| place.y).max().unwrap_or(min_y);
    places
        .iter()
        .map(|place| {
            let point = normalized_legacy_point(place.x, place.y, min_x, max_x, min_y, max_y);
            let target = nearest_profile_id(profiles, &point)
                .unwrap_or(spawn_place_id)
                .to_string();
            (place.id.clone(), target)
        })
        .collect()
}

fn normalized_legacy_point(
    x: i32,
    y: i32,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
) -> NormalizedPoint {
    let width = f64::from((max_x - min_x).max(1));
    let height = f64::from((max_y - min_y).max(1));
    NormalizedPoint {
        x: f64::from(x - min_x) / width,
        // 舊格 y 向北增加；Azgaar y 由畫面上方向下增加。
        y: f64::from(max_y - y) / height,
    }
}

fn nearest_profile_id<'a>(
    profiles: &'a [RuntimeAreaProfile],
    point: &NormalizedPoint,
) -> Option<&'a str> {
    profiles
        .iter()
        .min_by(|left, right| {
            squared_distance(&left.centroid, point)
                .total_cmp(&squared_distance(&right.centroid, point))
                .then_with(|| left.place_id.cmp(&right.place_id))
        })
        .map(|profile| profile.place_id.as_str())
}

fn squared_distance(left: &NormalizedPoint, right: &NormalizedPoint) -> f64 {
    (left.x - right.x).powi(2) + (left.y - right.y).powi(2)
}

fn upsert_migration_map(
    tx: &mut Transaction<'_>,
    mapping: &HashMap<String, String>,
    now_unix: i64,
) -> anyhow::Result<()> {
    for (source, target) in mapping {
        tx.execute(
            "INSERT INTO world_place_migration_map(runtime_id,source_place_id,target_place_id,mapped_at)
             VALUES ($1,$2,$3,$4)
             ON CONFLICT(runtime_id,source_place_id) DO UPDATE SET
             target_place_id=EXCLUDED.target_place_id,mapped_at=EXCLUDED.mapped_at",
            &[&WHITE_TOWER_RUNTIME_ID, source, target, &now_unix],
        )?;
    }
    Ok(())
}

fn capture_location_backups(tx: &mut Transaction<'_>, now_unix: i64) -> anyhow::Result<()> {
    tx.execute(
        "INSERT INTO world_location_migration_backups
         (runtime_id,entity_id,old_place_id,old_grid_x,old_grid_y,old_room_id,
          old_journey_reason,old_destination_place_id,old_planned_path,old_consequence,
          old_journey_updated_at,identity_snapshot,captured_at)
         SELECT $1,e.id,e.place_id,e.grid_x,e.grid_y,er.room_id,j.reason,j.destination_place_id,
                j.planned_path,j.consequence,j.updated_at,
                (to_jsonb(e)-'place_id'-'grid_x'-'grid_y')::text,$2
         FROM entities e
         LEFT JOIN entity_rooms er ON er.entity_id=e.id
         LEFT JOIN entity_journeys j ON j.entity_id=e.id
         ON CONFLICT(runtime_id,entity_id) DO NOTHING",
        &[&WHITE_TOWER_RUNTIME_ID, &now_unix],
    )?;
    Ok(())
}

fn migrate_entities(
    tx: &mut Transaction<'_>,
    world: &WhiteTowerRuntimeWorld,
    mapping: &HashMap<String, String>,
) -> anyhow::Result<(u64, HashMap<String, String>)> {
    let rows = tx.query(
        "SELECT e.id,e.place_id,e.grid_x,e.grid_y,p.legacy_grid_x,p.legacy_grid_y,p.source_kind
         FROM entities e LEFT JOIN world_places p ON p.id=e.place_id ORDER BY e.id",
        &[],
    )?;
    let legacy_places = load_legacy_places(tx)?;
    let bounds = legacy_bounds(&legacy_places);
    let mut changed = 0;
    let mut targets = HashMap::with_capacity(rows.len());
    for row in rows {
        let entity_id: String = row.get(0);
        let old_place_id: Option<String> = row.get(1);
        let grid_x: Option<i32> = row.get(2);
        let grid_y: Option<i32> = row.get(3);
        let place_x: Option<i32> = row.get(4);
        let place_y: Option<i32> = row.get(5);
        let source_kind: Option<String> = row.get(6);
        if old_place_id.is_none() && grid_x.is_some() && grid_y.is_some() {
            continue;
        }
        let is_runtime = source_kind.as_deref() == Some("azgaar_runtime")
            && old_place_id
                .as_deref()
                .is_some_and(|id| world.graph().is_ok_and(|graph| graph.place(id).is_some()));
        let target = if is_runtime {
            old_place_id
                .clone()
                .unwrap_or_else(|| world.spawn_place_id.clone())
        } else if let Some(mapped) = old_place_id.as_ref().and_then(|id| mapping.get(id)) {
            mapped.clone()
        } else if let (Some(x), Some(y), Some((min_x, max_x, min_y, max_y))) =
            (grid_x.or(place_x), grid_y.or(place_y), bounds)
        {
            let point = normalized_legacy_point(x, y, min_x, max_x, min_y, max_y);
            nearest_profile_id(&world.area_profiles, &point)
                .unwrap_or(&world.spawn_place_id)
                .to_string()
        } else {
            world.spawn_place_id.clone()
        };
        if old_place_id.as_deref() != Some(&target) || grid_x.is_some() || grid_y.is_some() {
            changed += tx.execute(
                "UPDATE entities SET place_id=$1,grid_x=NULL,grid_y=NULL WHERE id=$2",
                &[&target, &entity_id],
            )?;
        }
        tx.execute(
            "INSERT INTO entity_rooms(entity_id,room_id) VALUES ($1,$2)
             ON CONFLICT(entity_id) DO UPDATE SET room_id=EXCLUDED.room_id",
            &[&entity_id, &target],
        )?;
        targets.insert(entity_id, target);
    }
    Ok((changed, targets))
}

fn legacy_bounds(places: &[LegacyPlace]) -> Option<(i32, i32, i32, i32)> {
    Some((
        places.iter().map(|place| place.x).min()?,
        places.iter().map(|place| place.x).max()?,
        places.iter().map(|place| place.y).min()?,
        places.iter().map(|place| place.y).max()?,
    ))
}

fn migrate_journeys(
    tx: &mut Transaction<'_>,
    world: &WhiteTowerRuntimeWorld,
    mapping: &HashMap<String, String>,
    entity_targets: &HashMap<String, String>,
    now_unix: i64,
) -> anyhow::Result<u64> {
    let graph = world.graph()?;
    let mut changed = tx.execute(
        "DELETE FROM entity_journeys WHERE entity_id IN (
            SELECT id FROM entities WHERE place_id IS NULL
        ) OR entity_id NOT IN (SELECT id FROM entities)",
        &[],
    )?;
    let rows = tx.query(
        "SELECT entity_id,reason,destination_place_id,planned_path,consequence
         FROM entity_journeys ORDER BY entity_id",
        &[],
    )?;
    for row in rows {
        let entity_id: String = row.get(0);
        let reason: String = row.get(1);
        let old_destination: Option<String> = row.get(2);
        let old_path_raw: String = row.get(3);
        let old_consequence: String = row.get(4);
        let Some(current) = entity_targets.get(&entity_id) else {
            changed += tx.execute("DELETE FROM entity_journeys WHERE entity_id=$1", &[&entity_id])?;
            continue;
        };
        let old_path: Vec<String> = serde_json::from_str(&old_path_raw).unwrap_or_default();
        if journey_path_is_valid(&graph, current, old_destination.as_deref(), &old_path) {
            continue;
        }
        let destination = old_destination
            .as_ref()
            .and_then(|id| {
                if graph.place(id).is_some() {
                    Some(id.clone())
                } else {
                    mapping.get(id).cloned()
                }
            })
            .or_else(|| (reason != "idle").then(|| current.clone()));
        let path = destination
            .as_deref()
            .and_then(|target| graph.find_path(current, target))
            .unwrap_or_default()
            .into_iter()
            .skip(1)
            .collect::<Vec<_>>();
        let path_raw = serde_json::to_string(&path)?;
        let consequence = if old_consequence.contains("旅程已換軌至白塔半島") {
            old_consequence
        } else if old_consequence.trim().is_empty() {
            "舊世界旅程已換軌至白塔半島".to_string()
        } else {
            format!(
                "{}；旅程已換軌至白塔半島",
                old_consequence.trim_end_matches('；')
            )
        };
        changed += tx.execute(
            "UPDATE entity_journeys SET destination_place_id=$1,planned_path=$2,
             consequence=$3,updated_at=$4 WHERE entity_id=$5",
            &[&destination, &path_raw, &consequence, &now_unix, &entity_id],
        )?;
    }
    Ok(changed)
}

fn journey_path_is_valid(
    graph: &crate::world::place::PlaceGraph,
    current: &str,
    destination: Option<&str>,
    path: &[String],
) -> bool {
    if graph.place(current).is_none()
        || destination.is_some_and(|target| graph.place(target).is_none())
    {
        return false;
    }
    let mut previous = current;
    for next in path {
        if graph.route_to(previous, next).is_none() {
            return false;
        }
        previous = next;
    }
    destination.is_none_or(|target| target == previous)
}

#[derive(Debug, Default)]
struct JourneyValidation {
    invalid: usize,
    examples: Vec<String>,
}

fn validate_journeys(
    client: &mut impl GenericClient,
    world: &WhiteTowerRuntimeWorld,
) -> anyhow::Result<JourneyValidation> {
    let graph = world.graph()?;
    let rows = client.query(
        "SELECT j.entity_id,e.place_id,j.destination_place_id,j.planned_path
         FROM entity_journeys j JOIN entities e ON e.id=j.entity_id",
        &[],
    )?;
    let mut validation = JourneyValidation::default();
    for row in rows {
        let entity_id: String = row.get(0);
        let current: Option<String> = row.get(1);
        let destination: Option<String> = row.get(2);
        let raw: String = row.get(3);
        let path = serde_json::from_str::<Vec<String>>(&raw);
        let valid = match (current.as_deref(), path.as_ref()) {
            (Some(current), Ok(path)) => {
                journey_path_is_valid(&graph, current, destination.as_deref(), path)
            }
            _ => false,
        };
        if !valid {
            validation.invalid += 1;
            if validation.examples.len() < 5 {
                let (path_len, first_step) = path
                    .as_ref()
                    .map(|path| (path.len(), path.first().map(String::as_str).unwrap_or("-")))
                    .unwrap_or((0, "invalid-json"));
                validation.examples.push(format!(
                    "{entity_id}@{}→{} path={path_len} first={first_step}",
                    current.as_deref().unwrap_or("none"),
                    destination.as_deref().unwrap_or("none")
                ));
            }
        }
    }
    Ok(validation)
}

fn entity_place(
    client: &mut impl GenericClient,
    entity_id: &str,
) -> anyhow::Result<Option<String>> {
    Ok(client
        .query_opt("SELECT place_id FROM entities WHERE id=$1", &[&entity_id])?
        .and_then(|row| row.get(0)))
}

fn codex_identity(client: &mut impl GenericClient) -> anyhow::Result<Option<String>> {
    Ok(client
        .query_opt(
            "SELECT (to_jsonb(e)-'place_id'-'grid_x'-'grid_y')::text
             FROM entities e WHERE id='codex'",
            &[],
        )?
        .map(|row| row.get(0)))
}

fn count(client: &mut impl GenericClient, query: &str) -> anyhow::Result<i64> {
    Ok(client.query_one(query, &[])?.get(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_north_maps_to_azgaar_top() {
        let point = normalized_legacy_point(0, 10, -10, 10, -10, 10);
        assert_eq!(point.x, 0.5);
        assert_eq!(point.y, 0.0);
    }

    #[test]
    fn mapping_is_complete_and_deterministic() {
        let world = builtin_white_tower_runtime_world().expect("runtime world");
        let legacy = vec![
            LegacyPlace {
                id: "a".into(),
                x: -1,
                y: 1,
            },
            LegacyPlace {
                id: "b".into(),
                x: 1,
                y: -1,
            },
        ];
        let first = build_legacy_mapping(&legacy, &world.area_profiles, &world.spawn_place_id);
        let second = build_legacy_mapping(&legacy, &world.area_profiles, &world.spawn_place_id);
        assert_eq!(first, second);
        assert_eq!(first.len(), legacy.len());
        assert!(
            first
                .values()
                .all(|id| world.places.iter().any(|place| &place.id == id))
        );
    }
}
