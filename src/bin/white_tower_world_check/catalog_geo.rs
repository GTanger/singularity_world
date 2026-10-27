//! 白塔提案的邊界、地理、民生鏈、組織與物流內容驗證。

use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use super::refs::{ensure_nonblank, ensure_ref, ensure_refs, ensure_texts};
use super::{
    Boundaries, Corridor, Flow, Lifeline, Organization, PlayerProjection, Region, TravelTime,
};

/// 驗證正典、工作前提、未決項與禁止升格四組邊界都有實際文字。
pub(super) fn validate_boundaries(boundaries: &Boundaries) -> Result<()> {
    ensure_texts("boundaries.accepted", &boundaries.accepted)?;
    ensure_texts("boundaries.working_premises", &boundaries.working_premises)?;
    ensure_texts("boundaries.not_decided", &boundaries.not_decided)?;
    ensure_texts(
        "boundaries.forbidden_promotions",
        &boundaries.forbidden_promotions,
    )
}

/// 驗證玩家投影明確列出可見、隱藏與不變原則，不決定 UI 形式。
pub(super) fn validate_player_projection(projection: &PlayerProjection) -> Result<()> {
    ensure_texts("player_projection.expose", &projection.expose)?;
    ensure_texts(
        "player_projection.hide_by_default",
        &projection.hide_by_default,
    )?;
    ensure_nonblank("player_projection.invariant", &projection.invariant)
}

/// 驗證每個生活帶具有人口、聚居形態、承重作用及有效鄰區。
pub(super) fn validate_regions(regions: &[Region], region_ids: &BTreeSet<String>) -> Result<()> {
    for region in regions {
        ensure_nonblank(&format!("{}.name", region.id), &region.name)?;
        ensure!(region.population > 0, "{} 人口不可為零", region.id);
        ensure_nonblank(
            &format!("{}.settlement_form", region.id),
            &region.settlement_form,
        )?;
        ensure_texts(
            &format!("{}.load_bearing_roles", region.id),
            &region.load_bearing_roles,
        )?;
        ensure_refs(
            &format!("{}.neighbor_refs", region.id),
            &region.neighbor_refs,
            region_ids,
        )?;
        ensure!(
            !region.neighbor_refs.contains(&region.id),
            "{} 不得把自己列為鄰區",
            region.id
        );
    }
    Ok(())
}

/// 驗證每條主要連結至少接兩區，並保有運輸方式與限制。
pub(super) fn validate_corridors(
    corridors: &[Corridor],
    region_ids: &BTreeSet<String>,
) -> Result<()> {
    for corridor in corridors {
        ensure_nonblank(&format!("{}.name", corridor.id), &corridor.name)?;
        ensure!(
            corridor.region_refs.len() >= 2,
            "{} 至少要連接兩個生活帶",
            corridor.id
        );
        ensure_refs(
            &format!("{}.region_refs", corridor.id),
            &corridor.region_refs,
            region_ids,
        )?;
        ensure_texts(&format!("{}.modes", corridor.id), &corridor.modes)?;
        ensure_texts(
            &format!("{}.constraints", corridor.id),
            &corridor.constraints,
        )?;
    }
    Ok(())
}

/// 驗證路程起訖為不同有效地方，並具備常態量級與變動因素。
pub(super) fn validate_travel_times(
    times: &[TravelTime],
    place_ids: &BTreeSet<String>,
) -> Result<()> {
    for time in times {
        ensure_ref(&format!("{}.from_ref", time.id), &time.from_ref, place_ids)?;
        ensure_ref(&format!("{}.to_ref", time.id), &time.to_ref, place_ids)?;
        ensure!(time.from_ref != time.to_ref, "{} 起訖地不得相同", time.id);
        ensure_nonblank(&format!("{}.ordinary_range", time.id), &time.ordinary_range)?;
        ensure_texts(&format!("{}.modifiers", time.id), &time.modifiers)?;
    }
    Ok(())
}

/// 驗證每條民生鏈具備來源、服務區、處理過程、脆弱點與世界後果。
pub(super) fn validate_lifelines(
    lifelines: &[Lifeline],
    place_ids: &BTreeSet<String>,
    region_ids: &BTreeSet<String>,
) -> Result<()> {
    for lifeline in lifelines {
        ensure_nonblank(&format!("{}.name", lifeline.id), &lifeline.name)?;
        ensure_refs(
            &format!("{}.source_refs", lifeline.id),
            &lifeline.source_refs,
            place_ids,
        )?;
        ensure_refs(
            &format!("{}.serves_region_refs", lifeline.id),
            &lifeline.serves_region_refs,
            region_ids,
        )?;
        ensure_texts(&format!("{}.processes", lifeline.id), &lifeline.processes)?;
        ensure_texts(
            &format!("{}.vulnerabilities", lifeline.id),
            &lifeline.vulnerabilities,
        )?;
        ensure_texts(
            &format!("{}.consequences", lifeline.id),
            &lifeline.consequences,
        )?;
    }
    Ok(())
}

/// 驗證組織由有效居民與地區承載，且能力和限制都不留白。
pub(super) fn validate_organizations(
    organizations: &[Organization],
    region_ids: &BTreeSet<String>,
    resident_ids: &BTreeSet<String>,
) -> Result<()> {
    for organization in organizations {
        ensure_nonblank(&format!("{}.name", organization.id), &organization.name)?;
        ensure_refs(
            &format!("{}.region_refs", organization.id),
            &organization.region_refs,
            region_ids,
        )?;
        ensure_refs(
            &format!("{}.resident_refs", organization.id),
            &organization.resident_refs,
            resident_ids,
        )?;
        ensure_texts(
            &format!("{}.capabilities", organization.id),
            &organization.capabilities,
        )?;
        ensure_texts(&format!("{}.limits", organization.id), &organization.limits)?;
    }
    Ok(())
}

/// 驗證跨區流動的起訖、途經、返向、脆弱點、組織與物件引用。
pub(super) fn validate_flows(
    flows: &[Flow],
    place_ids: &BTreeSet<String>,
    organization_ids: &BTreeSet<String>,
    object_ids: &BTreeSet<String>,
) -> Result<()> {
    for flow in flows {
        ensure_nonblank(&format!("{}.name", flow.id), &flow.name)?;
        ensure_refs(
            &format!("{}.origin_refs", flow.id),
            &flow.origin_refs,
            place_ids,
        )?;
        ensure_refs(&format!("{}.via_refs", flow.id), &flow.via_refs, place_ids)?;
        ensure_refs(
            &format!("{}.destination_refs", flow.id),
            &flow.destination_refs,
            place_ids,
        )?;
        ensure_nonblank(&format!("{}.return_flow", flow.id), &flow.return_flow)?;
        ensure_texts(
            &format!("{}.vulnerabilities", flow.id),
            &flow.vulnerabilities,
        )?;
        ensure_refs(
            &format!("{}.organization_refs", flow.id),
            &flow.organization_refs,
            organization_ids,
        )?;
        ensure_refs(
            &format!("{}.object_refs", flow.id),
            &flow.object_refs,
            object_ids,
        )?;
    }
    Ok(())
}
