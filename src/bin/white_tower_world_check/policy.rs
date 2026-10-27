//! 白塔半島工作提案的高階政策、數量、身分集合與驗證流程。

use std::collections::BTreeSet;

use anyhow::{Result, ensure};
use singularity_world::world_state::WorldRunReceipt;

use super::catalog_geo::{
    validate_boundaries, validate_corridors, validate_flows, validate_lifelines,
    validate_organizations, validate_player_projection, validate_regions, validate_travel_times,
};
use super::catalog_people::{
    validate_external_interfaces, validate_history, validate_objects, validate_residents,
};
use super::refs::{ensure_nonblank, ensure_texts, unique_ids};
use super::validation::validate_current_state;
use super::{Manifest, ValidationOutput, ValidationSummary};

struct IdSets {
    regions: BTreeSet<String>,
    corridors: BTreeSet<String>,
    travel_times: BTreeSet<String>,
    lifelines: BTreeSet<String>,
    organizations: BTreeSet<String>,
    flows: BTreeSet<String>,
    history: BTreeSet<String>,
    residents: BTreeSet<String>,
    objects: BTreeSet<String>,
    external: BTreeSet<String>,
    facts: BTreeSet<String>,
    transitions: BTreeSet<String>,
}

/// 依序驗證提案身分、固定數量、引用、內容與 D0→D7，回傳摘要及通用收據。
pub(super) fn validate_manifest(manifest: &Manifest) -> Result<ValidationOutput> {
    validate_identity(manifest)?;
    let ids = collect_ids(manifest)?;
    validate_counts(manifest)?;
    let global_ids = merge_global_ids(&ids)?;
    let population = validate_population(manifest, &ids.regions)?;
    validate_catalog(manifest, &ids, &global_ids)?;
    let (run, resume) = validate_current_state(
        &manifest.id,
        &manifest.current_state,
        &ids.regions,
        &ids.residents,
        &ids.objects,
        &global_ids,
    )?;
    Ok(build_output(manifest, population, run, resume))
}

/// 驗證資料版本、提案狀態、禁止升格邊界、連續性規則與玩家投影聲明。
fn validate_identity(manifest: &Manifest) -> Result<()> {
    ensure!(manifest.schema_version == 1, "schema_version 必須為 1");
    ensure_nonblank("manifest.id", &manifest.id)?;
    ensure_nonblank("manifest.name", &manifest.name)?;
    ensure!(
        manifest.status == "working_proposal",
        "status 必須保持 working_proposal，避免資料自動升格"
    );
    ensure!(!manifest.canon, "待審資料不得標成 canon");
    ensure_texts("sources", &manifest.sources)?;
    validate_boundaries(&manifest.boundaries)?;
    ensure_texts("continuity_rules", &manifest.continuity_rules)?;
    ensure!(
        manifest.continuity_rules.len() >= 6,
        "生活連續性規則不得少於 6 條"
    );
    validate_player_projection(&manifest.player_projection)
}

/// 從每一類資料建立唯一身分集合，供後續跨層引用與重複檢查使用。
fn collect_ids(manifest: &Manifest) -> Result<IdSets> {
    Ok(IdSets {
        regions: unique_ids("regions", manifest.regions.iter().map(|value| &value.id))?,
        corridors: unique_ids(
            "corridors",
            manifest.corridors.iter().map(|value| &value.id),
        )?,
        travel_times: unique_ids(
            "travel_times",
            manifest.travel_times.iter().map(|value| &value.id),
        )?,
        lifelines: unique_ids(
            "lifelines",
            manifest.lifelines.iter().map(|value| &value.id),
        )?,
        organizations: unique_ids(
            "organizations",
            manifest.organizations.iter().map(|value| &value.id),
        )?,
        flows: unique_ids("flows", manifest.flows.iter().map(|value| &value.id))?,
        history: unique_ids("history", manifest.history.iter().map(|value| &value.id))?,
        residents: unique_ids(
            "residents",
            manifest.residents.iter().map(|value| &value.id),
        )?,
        objects: unique_ids("objects", manifest.objects.iter().map(|value| &value.id))?,
        external: unique_ids(
            "external_interfaces",
            manifest.external_interfaces.iter().map(|value| &value.id),
        )?,
        facts: unique_ids(
            "current_state.facts",
            manifest.current_state.facts.iter().map(|value| &value.id),
        )?,
        transitions: unique_ids(
            "current_state.all_transitions",
            manifest
                .current_state
                .transitions
                .iter()
                .chain(
                    manifest
                        .current_state
                        .continuation
                        .iter()
                        .flat_map(|step| step.transitions.iter()),
                )
                .map(|value| &value.id),
        )?,
    })
}

/// 驗證本版白塔完整地區稿宣告的承重類別數量與 D0 覆蓋數量。
fn validate_counts(manifest: &Manifest) -> Result<()> {
    ensure!(manifest.regions.len() == 9, "白塔半島必須覆蓋 9 個生活帶");
    ensure!(manifest.corridors.len() == 7, "主要連結必須為 7 條");
    ensure!(manifest.travel_times.len() == 9, "路程量級必須覆蓋 9 組");
    ensure!(manifest.lifelines.len() == 7, "民生實體鏈必須為 7 條");
    ensure!(manifest.organizations.len() == 7, "承重組織必須為 7 種");
    ensure!(manifest.flows.len() == 6, "跨區閉環必須為 6 條");
    ensure!(manifest.history.len() == 8, "歷史承重節點必須為 8 個");
    ensure!(manifest.residents.len() == 13, "承重居民樣本必須為 13 名");
    ensure!(manifest.objects.len() == 13, "承重物件與場所必須為 13 件");
    ensure!(
        manifest.external_interfaces.len() == 4,
        "外部世界接口必須為 4 個"
    );
    ensure!(
        manifest.current_state.region_snapshots.len() == manifest.regions.len(),
        "D0 必須為每個生活帶提供一個局部快照"
    );
    ensure!(
        manifest.current_state.transitions.len() == 7,
        "D0 -> D1 因果鏈必須包含 7 個具體轉換"
    );
    ensure!(
        manifest.current_state.continuation.len() == 6,
        "D1 -> D7 必須包含 6 個連續日次"
    );
    Ok(())
}

/// 合併所有類別身分並拒絕跨類別撞名，避免同一 id 指向兩種世界存在。
fn merge_global_ids(ids: &IdSets) -> Result<BTreeSet<String>> {
    let mut global = BTreeSet::new();
    for (kind, values) in [
        ("region", &ids.regions),
        ("corridor", &ids.corridors),
        ("travel_time", &ids.travel_times),
        ("lifeline", &ids.lifelines),
        ("organization", &ids.organizations),
        ("flow", &ids.flows),
        ("history", &ids.history),
        ("resident", &ids.residents),
        ("object", &ids.objects),
        ("external_interface", &ids.external),
        ("fact", &ids.facts),
        ("transition", &ids.transitions),
    ] {
        for id in values {
            ensure!(global.insert(id.clone()), "跨類別重複 id：{id}（{kind}）");
        }
    }
    Ok(global)
}

/// 驗證各生活帶人口加總等於提案總人口與一千八百萬工作前提。
fn validate_population(manifest: &Manifest, region_ids: &BTreeSet<String>) -> Result<u64> {
    validate_regions(&manifest.regions, region_ids)?;
    let population = manifest
        .regions
        .iter()
        .map(|region| region.population)
        .sum();
    ensure!(
        population == manifest.total_population,
        "人口加總 {population} 與 total_population {} 不符",
        manifest.total_population
    );
    ensure!(
        population == 18_000_000,
        "白塔半島工作前提人口必須維持 18,000,000"
    );
    Ok(population)
}

/// 驗證提案中地理、民生、組織、人物、物件與外部接口的全部引用及內容承重。
fn validate_catalog(manifest: &Manifest, ids: &IdSets, global: &BTreeSet<String>) -> Result<()> {
    let mut places = ids.regions.clone();
    places.extend(ids.external.iter().cloned());
    validate_corridors(&manifest.corridors, &ids.regions)?;
    validate_travel_times(&manifest.travel_times, &places)?;
    validate_lifelines(&manifest.lifelines, &places, &ids.regions)?;
    validate_organizations(&manifest.organizations, &ids.regions, &ids.residents)?;
    validate_flows(&manifest.flows, &places, &ids.organizations, &ids.objects)?;
    validate_history(&manifest.history, &ids.regions)?;
    validate_residents(
        &manifest.residents,
        &ids.regions,
        &ids.residents,
        &ids.objects,
    )?;
    validate_objects(&manifest.objects, &ids.regions, &ids.residents, global)?;
    validate_external_interfaces(&manifest.external_interfaces, &ids.regions, &ids.flows)
}

/// 由已驗證 manifest 與七日推進收據建立精簡摘要，不修改來源資料。
fn build_output(
    manifest: &Manifest,
    population: u64,
    run: WorldRunReceipt,
    resume: super::ResumeEvidence,
) -> ValidationOutput {
    ValidationOutput {
        summary: ValidationSummary {
            regions: manifest.regions.len(),
            population,
            corridors: manifest.corridors.len(),
            travel_times: manifest.travel_times.len(),
            lifelines: manifest.lifelines.len(),
            organizations: manifest.organizations.len(),
            flows: manifest.flows.len(),
            history: manifest.history.len(),
            residents: manifest.residents.len(),
            objects: manifest.objects.len(),
            external_interfaces: manifest.external_interfaces.len(),
            d0_facts: manifest.current_state.facts.len(),
            days: run.steps.len(),
            transitions: run.total_transition_count,
            changed_facts: run.total_changed_fact_count,
        },
        run,
        resume,
    }
}
