//! 白塔半島工作提案的 D0→D7 政策驗證與通用世界推進接線。

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use singularity_world::world_state::{
    AdvanceReceipt, AdvanceStep, WorldRunReceipt, advance_sequence,
};

use super::refs::{ensure_nonblank, ensure_ref, ensure_refs, unique_ids};
use super::{Condition, CurrentState, ResumeEvidence, Transition};

/// 驗證白塔 D0 與七日政策，再交由通用引擎連續推進至 D7 並核對每日完整狀態。
pub(super) fn validate_current_state(
    world_id: &str,
    state: &CurrentState,
    region_ids: &BTreeSet<String>,
    resident_ids: &BTreeSet<String>,
    object_ids: &BTreeSet<String>,
    global_ids: &BTreeSet<String>,
) -> Result<(WorldRunReceipt, ResumeEvidence)> {
    ensure!(state.day_id == "D0", "current_state.day_id 必須為 D0");
    validate_timeline(state)?;
    validate_snapshots(state, region_ids, resident_ids, object_ids)?;
    validate_fact_subjects(state, global_ids)?;
    validate_transition_refs(state, region_ids, resident_ids, object_ids)?;

    let steps = build_advance_steps(state);
    let run = advance_sequence(world_id, &state.day_id, &state.facts, &steps)?;
    validate_expected_states(state, &run)?;
    let resume = verify_d3_resume(world_id, &steps, &run)?;
    Ok((run, resume))
}

/// 固定本版提案的連續日次為 D1 至 D7，避免缺日或重排被當成同一次驗證。
fn validate_timeline(state: &CurrentState) -> Result<()> {
    let actual: Vec<&str> = state
        .continuation
        .iter()
        .map(|step| step.target_day_id.as_str())
        .collect();
    ensure!(
        actual == ["D2", "D3", "D4", "D5", "D6", "D7"],
        "continuation 必須依序完整覆蓋 D2 至 D7"
    );
    Ok(())
}

/// 將白塔首日與後續六日政策組成通用引擎輸入；來源資料保持只讀。
fn build_advance_steps(state: &CurrentState) -> Vec<AdvanceStep> {
    let mut steps = Vec::with_capacity(state.continuation.len() + 1);
    steps.push(AdvanceStep {
        target_day_id: "D1".to_string(),
        transitions: state.transitions.clone(),
    });
    steps.extend(state.continuation.iter().map(|step| AdvanceStep {
        target_day_id: step.target_day_id.clone(),
        transitions: step.transitions.clone(),
    }));
    steps
}

/// 從 D3 日終快照只重跑 D4 至 D7，並核對最終事實與逐日收據完全等價。
fn verify_d3_resume(
    world_id: &str,
    steps: &[AdvanceStep],
    full: &WorldRunReceipt,
) -> Result<ResumeEvidence> {
    let checkpoint = &full.steps[2];
    let remaining = &steps[3..];
    let resumed = advance_sequence(
        world_id,
        &checkpoint.target_day_id,
        &checkpoint.facts_after,
        remaining,
    )?;
    ensure!(
        resumed.facts_after == full.facts_after,
        "D3 接續後的最終事實與完整 D0→D7 推進不一致"
    );
    ensure!(
        resumed.steps == full.steps[3..],
        "D3 接續後的逐日收據與完整推進後段不一致"
    );
    Ok(ResumeEvidence {
        checkpoint_day_id: checkpoint.target_day_id.clone(),
        target_day_id: resumed.target_day_id,
        resumed_steps: resumed.steps.len(),
        final_facts_match: true,
        suffix_receipts_match: true,
    })
}

/// 驗證每個生活帶都有快照，且全部承重居民與物件至少被一個快照涵蓋。
fn validate_snapshots(
    state: &CurrentState,
    region_ids: &BTreeSet<String>,
    resident_ids: &BTreeSet<String>,
    object_ids: &BTreeSet<String>,
) -> Result<()> {
    let snapshot_regions = unique_ids(
        "current_state.region_snapshots",
        state.region_snapshots.iter().map(|value| &value.region_ref),
    )?;
    ensure!(
        snapshot_regions == *region_ids,
        "D0 快照必須恰好覆蓋全部生活帶"
    );

    let mut residents = BTreeSet::new();
    let mut objects = BTreeSet::new();
    for snapshot in &state.region_snapshots {
        ensure_nonblank(&format!("{}.state", snapshot.region_ref), &snapshot.state)?;
        ensure_refs(
            &format!("{}.actor_refs", snapshot.region_ref),
            &snapshot.actor_refs,
            resident_ids,
        )?;
        ensure_refs(
            &format!("{}.object_refs", snapshot.region_ref),
            &snapshot.object_refs,
            object_ids,
        )?;
        ensure_nonblank(&format!("{}.scope", snapshot.region_ref), &snapshot.scope)?;
        residents.extend(snapshot.actor_refs.iter().cloned());
        objects.extend(snapshot.object_refs.iter().cloned());
    }
    ensure!(
        residents == *resident_ids,
        "D0 快照必須至少一次涵蓋全部承重居民"
    );
    ensure!(
        objects == *object_ids,
        "D0 快照必須至少一次涵蓋全部承重物件"
    );
    Ok(())
}

/// 驗證每個事實的主體確實存在於白塔提案的跨層身分集合。
fn validate_fact_subjects(state: &CurrentState, global_ids: &BTreeSet<String>) -> Result<()> {
    for fact in &state.facts {
        ensure_ref(
            &format!("{}.subject_ref", fact.id),
            &fact.subject_ref,
            global_ids,
        )?;
    }
    Ok(())
}

/// 驗證白塔轉換中的人物、物件與地區引用，並阻止玩家成為離線推進行動者。
fn validate_transition_refs(
    state: &CurrentState,
    region_ids: &BTreeSet<String>,
    resident_ids: &BTreeSet<String>,
    object_ids: &BTreeSet<String>,
) -> Result<()> {
    for transition in all_transitions(state) {
        ensure_refs(
            &format!("{}.actor_refs", transition.id),
            &transition.actor_refs,
            resident_ids,
        )?;
        ensure!(
            transition
                .actor_refs
                .iter()
                .all(|actor| !actor.to_lowercase().contains("player")),
            "{} 不得以玩家作為世界自行推進的行動者",
            transition.id
        );
        ensure_refs(
            &format!("{}.object_refs", transition.id),
            &transition.object_refs,
            object_ids,
        )?;
        ensure_refs(
            &format!("{}.region_refs", transition.id),
            &transition.region_refs,
            region_ids,
        )?;
    }
    Ok(())
}

/// 依資料順序串接首日與後續六日的全部轉換，供引用驗證使用。
fn all_transitions(state: &CurrentState) -> impl Iterator<Item = &Transition> {
    state.transitions.iter().chain(
        state
            .continuation
            .iter()
            .flat_map(|step| step.transitions.iter()),
    )
}

/// 核對七份日終完整狀態，拒絕漏列未變事實或日次與收據錯位。
fn validate_expected_states(state: &CurrentState, run: &WorldRunReceipt) -> Result<()> {
    ensure!(run.steps.len() == 7, "世界推進收據必須完整包含 D1 至 D7");
    let fact_ids = unique_ids(
        "current_state.facts",
        state.facts.iter().map(|value| &value.id),
    )?;
    validate_expected("expected_d1", &state.expected_d1, &run.steps[0], &fact_ids)?;
    for (declared, receipt) in state.continuation.iter().zip(&run.steps[1..]) {
        ensure!(
            declared.target_day_id == receipt.target_day_id,
            "{} 與推進收據日次不符",
            declared.target_day_id
        );
        validate_expected(
            &format!("expected_{}", declared.target_day_id.to_lowercase()),
            &declared.expected,
            receipt,
            &fact_ids,
        )?;
    }
    Ok(())
}

/// 以單日收據核對一份完整預期狀態，不接受只列改變項。
fn validate_expected(
    label: &str,
    expected: &[Condition],
    receipt: &AdvanceReceipt,
    fact_ids: &BTreeSet<String>,
) -> Result<()> {
    let expected_ids = unique_ids(label, expected.iter().map(|value| &value.fact_ref))?;
    ensure!(
        expected_ids == *fact_ids,
        "{label} 必須恰好覆蓋全部事實，包含改變與未改變者"
    );
    let values: BTreeMap<String, String> = receipt
        .facts_after
        .iter()
        .map(|fact| (fact.id.clone(), fact.value.clone()))
        .collect();
    for condition in expected {
        validate_condition(label, condition, &values, fact_ids)?;
    }
    Ok(())
}

/// 核對白塔預期條件引用現有事實且與通用引擎結果一致，不修改收據。
fn validate_condition(
    owner: &str,
    condition: &Condition,
    facts: &BTreeMap<String, String>,
    fact_ids: &BTreeSet<String>,
) -> Result<()> {
    ensure_ref(
        &format!("{owner}.condition.fact_ref"),
        &condition.fact_ref,
        fact_ids,
    )?;
    ensure_nonblank(&format!("{owner}.condition.equals"), &condition.equals)?;
    let actual = facts
        .get(&condition.fact_ref)
        .with_context(|| format!("{owner} 找不到 fact {}", condition.fact_ref))?;
    ensure!(
        actual == &condition.equals,
        "{owner} 預期不成立：{} 需要 {}，實際為 {}",
        condition.fact_ref,
        condition.equals,
        actual
    );
    Ok(())
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
