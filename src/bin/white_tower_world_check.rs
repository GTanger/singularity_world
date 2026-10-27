#![forbid(unsafe_code)]
//! 白塔半島完整地區資料檢查器。
//!
//! 這支工具不接舊方格 runtime，也不決定座標、GUI 或玩家表面。它只驗證
//! data/world/white_tower_peninsula_v1.json 的承重關係，並實際套用
//! D0 -> D7 的無玩家確定性狀態轉換與中斷接續證據。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use singularity_world::world_state::{Condition, Fact, Transition, WorldRunReceipt};

#[path = "white_tower_world_check/catalog_geo.rs"]
mod catalog_geo;
#[path = "white_tower_world_check/catalog_people.rs"]
mod catalog_people;
#[path = "white_tower_world_check/command.rs"]
mod command;
#[path = "white_tower_world_check/policy.rs"]
mod policy;
#[path = "white_tower_world_check/receipt.rs"]
mod receipt;
#[path = "white_tower_world_check/refs.rs"]
mod refs;
#[path = "white_tower_world_check/validation.rs"]
mod validation;

use command::parse_args;
use policy::validate_manifest;
use receipt::{ReceiptWriteStatus, write_verification_receipt};

#[derive(Debug, Deserialize)]
struct Manifest {
    schema_version: u32,
    id: String,
    name: String,
    status: String,
    canon: bool,
    sources: Vec<String>,
    boundaries: Boundaries,
    total_population: u64,
    regions: Vec<Region>,
    corridors: Vec<Corridor>,
    travel_times: Vec<TravelTime>,
    lifelines: Vec<Lifeline>,
    organizations: Vec<Organization>,
    flows: Vec<Flow>,
    history: Vec<HistoryNode>,
    residents: Vec<Resident>,
    objects: Vec<WorldObject>,
    external_interfaces: Vec<ExternalInterface>,
    current_state: CurrentState,
    continuity_rules: Vec<String>,
    player_projection: PlayerProjection,
}

#[derive(Debug, Deserialize)]
struct Boundaries {
    accepted: Vec<String>,
    working_premises: Vec<String>,
    not_decided: Vec<String>,
    forbidden_promotions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Region {
    id: String,
    name: String,
    population: u64,
    settlement_form: String,
    load_bearing_roles: Vec<String>,
    neighbor_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Corridor {
    id: String,
    name: String,
    region_refs: Vec<String>,
    modes: Vec<String>,
    constraints: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TravelTime {
    id: String,
    from_ref: String,
    to_ref: String,
    ordinary_range: String,
    modifiers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Lifeline {
    id: String,
    name: String,
    source_refs: Vec<String>,
    serves_region_refs: Vec<String>,
    processes: Vec<String>,
    vulnerabilities: Vec<String>,
    consequences: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Organization {
    id: String,
    name: String,
    region_refs: Vec<String>,
    resident_refs: Vec<String>,
    capabilities: Vec<String>,
    limits: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Flow {
    id: String,
    name: String,
    origin_refs: Vec<String>,
    via_refs: Vec<String>,
    destination_refs: Vec<String>,
    return_flow: String,
    vulnerabilities: Vec<String>,
    organization_refs: Vec<String>,
    object_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct HistoryNode {
    id: String,
    age_before_d0: String,
    change: String,
    result: String,
    region_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Resident {
    id: String,
    name: String,
    age: u32,
    home_region_ref: String,
    current_region_ref: String,
    body_and_use: String,
    knowledge: String,
    asset_refs: Vec<String>,
    linked_resident_refs: Vec<String>,
    obligations: Vec<String>,
    intent: String,
    options: Vec<String>,
    prior_consequence: String,
}

#[derive(Debug, Deserialize)]
struct WorldObject {
    id: String,
    name: String,
    kind: String,
    region_refs: Vec<String>,
    owner: String,
    custodian_refs: Vec<String>,
    state: String,
    dependency_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExternalInterface {
    id: String,
    name: String,
    region_refs: Vec<String>,
    flow_refs: Vec<String>,
    exchanges: Vec<String>,
    forbidden_shortcut: String,
}

#[derive(Debug, Deserialize)]
struct CurrentState {
    day_id: String,
    region_snapshots: Vec<RegionSnapshot>,
    facts: Vec<Fact>,
    transitions: Vec<Transition>,
    expected_d1: Vec<Condition>,
    continuation: Vec<ContinuationStep>,
}

#[derive(Debug, Deserialize)]
struct ContinuationStep {
    target_day_id: String,
    transitions: Vec<Transition>,
    expected: Vec<Condition>,
}

#[derive(Debug, Deserialize)]
struct RegionSnapshot {
    region_ref: String,
    state: String,
    actor_refs: Vec<String>,
    object_refs: Vec<String>,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct PlayerProjection {
    expose: Vec<String>,
    hide_by_default: Vec<String>,
    invariant: String,
}

#[derive(Debug, PartialEq, Eq)]
struct ValidationSummary {
    regions: usize,
    population: u64,
    corridors: usize,
    travel_times: usize,
    lifelines: usize,
    organizations: usize,
    flows: usize,
    history: usize,
    residents: usize,
    objects: usize,
    external_interfaces: usize,
    d0_facts: usize,
    days: usize,
    transitions: usize,
    changed_facts: usize,
}

#[derive(Debug)]
struct ValidationOutput {
    summary: ValidationSummary,
    run: WorldRunReceipt,
    resume: ResumeEvidence,
}

#[derive(Debug, Serialize)]
struct ResumeEvidence {
    checkpoint_day_id: String,
    target_day_id: String,
    resumed_steps: usize,
    final_facts_match: bool,
    suffix_receipts_match: bool,
}

/// 載入白塔工作提案、完成政策與因果驗證，必要時輸出非 runtime 的工程收據。
fn main() -> Result<()> {
    let args = parse_args()?;
    let manifest = load_manifest(&args.manifest_path)?;
    let output = validate_manifest(&manifest)?;
    let summary = &output.summary;

    println!("white_tower_world_check: ok");
    println!(
        "manifest={} status={} canon={}",
        manifest.id, manifest.status, manifest.canon
    );
    println!(
        "population={} regions={} corridors={} travel_times={}",
        summary.population, summary.regions, summary.corridors, summary.travel_times
    );
    println!(
        "lifelines={} organizations={} flows={} history={}",
        summary.lifelines, summary.organizations, summary.flows, summary.history
    );
    println!(
        "residents={} objects={} external_interfaces={}",
        summary.residents, summary.objects, summary.external_interfaces
    );
    println!(
        "d0_facts={} days={} transitions={} changed_facts={} player_actors=0",
        summary.d0_facts, summary.days, summary.transitions, summary.changed_facts
    );
    println!(
        "resume_checkpoint={} target={} resumed_steps={} final_facts_match={} suffix_receipts_match={}",
        output.resume.checkpoint_day_id,
        output.resume.target_day_id,
        output.resume.resumed_steps,
        output.resume.final_facts_match,
        output.resume.suffix_receipts_match
    );
    if let Some(path) = args.receipt_path {
        let status = write_verification_receipt(&path, &manifest, &output.run, &output.resume)?;
        let label = match status {
            ReceiptWriteStatus::Created => "created",
            ReceiptWriteStatus::Unchanged => "unchanged",
        };
        println!("verification_receipt={label} path={}", path.display());
    }
    Ok(())
}

/// 從指定路徑讀取並解析白塔半島工作提案，不修改來源檔。
fn load_manifest(path: &Path) -> Result<Manifest> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("無法讀取白塔半島世界資料：{}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("白塔半島世界資料不是合法 JSON：{}", path.display()))
}
