//! 白塔提案的歷史、居民、物件與外部接口內容驗證。

use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use super::refs::{
    ensure_nonblank, ensure_ref, ensure_refs, ensure_refs_allow_empty, ensure_texts,
};
use super::{ExternalInterface, HistoryNode, Resident, WorldObject};

/// 驗證每個歷史節點具備時間、改變、結果與有效地區引用。
pub(super) fn validate_history(
    history: &[HistoryNode],
    region_ids: &BTreeSet<String>,
) -> Result<()> {
    for node in history {
        ensure_nonblank(&format!("{}.age_before_d0", node.id), &node.age_before_d0)?;
        ensure_nonblank(&format!("{}.change", node.id), &node.change)?;
        ensure_nonblank(&format!("{}.result", node.id), &node.result)?;
        ensure_refs(
            &format!("{}.region_refs", node.id),
            &node.region_refs,
            region_ids,
        )?;
    }
    Ok(())
}

/// 驗證全部承重居民；每名居民的身體、知識、持物、關係與後果由單項函數處理。
pub(super) fn validate_residents(
    residents: &[Resident],
    region_ids: &BTreeSet<String>,
    resident_ids: &BTreeSet<String>,
    object_ids: &BTreeSet<String>,
) -> Result<()> {
    for resident in residents {
        validate_resident(resident, region_ids, resident_ids, object_ids)?;
    }
    Ok(())
}

/// 驗證一名居民的身分、位置、身體、知識、持物、關係、責任、意圖與既有後果。
fn validate_resident(
    resident: &Resident,
    region_ids: &BTreeSet<String>,
    resident_ids: &BTreeSet<String>,
    object_ids: &BTreeSet<String>,
) -> Result<()> {
    ensure_nonblank(&format!("{}.name", resident.id), &resident.name)?;
    ensure!(resident.age > 0, "{} age 必須大於零", resident.id);
    ensure_ref(
        &format!("{}.home_region_ref", resident.id),
        &resident.home_region_ref,
        region_ids,
    )?;
    ensure_ref(
        &format!("{}.current_region_ref", resident.id),
        &resident.current_region_ref,
        region_ids,
    )?;
    ensure_nonblank(
        &format!("{}.body_and_use", resident.id),
        &resident.body_and_use,
    )?;
    ensure_nonblank(&format!("{}.knowledge", resident.id), &resident.knowledge)?;
    validate_resident_links(resident, resident_ids, object_ids)?;
    ensure_texts(
        &format!("{}.obligations", resident.id),
        &resident.obligations,
    )?;
    ensure_nonblank(&format!("{}.intent", resident.id), &resident.intent)?;
    ensure_texts(&format!("{}.options", resident.id), &resident.options)?;
    ensure_nonblank(
        &format!("{}.prior_consequence", resident.id),
        &resident.prior_consequence,
    )
}

/// 驗證居民持物與人際引用有效，並禁止把自己列成關係對象。
fn validate_resident_links(
    resident: &Resident,
    resident_ids: &BTreeSet<String>,
    object_ids: &BTreeSet<String>,
) -> Result<()> {
    ensure_refs(
        &format!("{}.asset_refs", resident.id),
        &resident.asset_refs,
        object_ids,
    )?;
    ensure_refs_allow_empty(
        &format!("{}.linked_resident_refs", resident.id),
        &resident.linked_resident_refs,
        resident_ids,
    )?;
    ensure!(
        !resident.linked_resident_refs.contains(&resident.id),
        "{} 不得把自己列為關係對象",
        resident.id
    );
    Ok(())
}

/// 驗證物件的地區、所有者、保管者、狀態與依賴都有可追查身分。
pub(super) fn validate_objects(
    objects: &[WorldObject],
    region_ids: &BTreeSet<String>,
    resident_ids: &BTreeSet<String>,
    dependency_ids: &BTreeSet<String>,
) -> Result<()> {
    for object in objects {
        ensure_nonblank(&format!("{}.name", object.id), &object.name)?;
        ensure_nonblank(&format!("{}.kind", object.id), &object.kind)?;
        ensure_refs(
            &format!("{}.region_refs", object.id),
            &object.region_refs,
            region_ids,
        )?;
        ensure_nonblank(&format!("{}.owner", object.id), &object.owner)?;
        ensure_refs(
            &format!("{}.custodian_refs", object.id),
            &object.custodian_refs,
            resident_ids,
        )?;
        ensure_nonblank(&format!("{}.state", object.id), &object.state)?;
        ensure_refs(
            &format!("{}.dependency_refs", object.id),
            &object.dependency_refs,
            dependency_ids,
        )?;
    }
    Ok(())
}

/// 驗證每個外部世界接口都有地區、物流、交換內容與禁止捷徑。
pub(super) fn validate_external_interfaces(
    interfaces: &[ExternalInterface],
    region_ids: &BTreeSet<String>,
    flow_ids: &BTreeSet<String>,
) -> Result<()> {
    for interface in interfaces {
        ensure_nonblank(&format!("{}.name", interface.id), &interface.name)?;
        ensure_refs(
            &format!("{}.region_refs", interface.id),
            &interface.region_refs,
            region_ids,
        )?;
        ensure_refs(
            &format!("{}.flow_refs", interface.id),
            &interface.flow_refs,
            flow_ids,
        )?;
        ensure_texts(&format!("{}.exchanges", interface.id), &interface.exchanges)?;
        ensure_nonblank(
            &format!("{}.forbidden_shortcut", interface.id),
            &interface.forbidden_shortcut,
        )?;
    }
    Ok(())
}
