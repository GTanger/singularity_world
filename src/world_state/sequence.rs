//! 多階段世界推進；以單步引擎串接完整時段，不碰持久層或玩家表面。

use std::collections::BTreeSet;

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

use super::{AdvanceReceipt, Fact, Transition, advance};

/// 拒絕空白世界狀態字串，供單步與多步引擎共用，無副作用。
pub(super) fn ensure_nonblank(label: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{label} 不可為空白");
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdvanceStep {
    pub target_day_id: String,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WorldRunReceipt {
    pub schema_version: u32,
    pub world_id: String,
    pub source_day_id: String,
    pub target_day_id: String,
    pub facts_before: Vec<Fact>,
    pub facts_after: Vec<Fact>,
    pub steps: Vec<AdvanceReceipt>,
    pub total_transition_count: usize,
    pub total_changed_fact_count: usize,
}

/// 依序完成多個世界日次；任一步失敗時不回傳部分執行結果。
pub fn advance_sequence(
    world_id: &str,
    source_day_id: &str,
    facts: &[Fact],
    steps: &[AdvanceStep],
) -> Result<WorldRunReceipt> {
    validate_sequence(source_day_id, steps)?;
    let mut day_id = source_day_id.to_string();
    let mut current_facts = facts.to_vec();
    let mut receipts = Vec::with_capacity(steps.len());

    for step in steps {
        let receipt = advance(
            world_id,
            &day_id,
            &step.target_day_id,
            &current_facts,
            &step.transitions,
        )?;
        day_id.clone_from(&step.target_day_id);
        current_facts.clone_from(&receipt.facts_after);
        receipts.push(receipt);
    }
    Ok(build_receipt(
        world_id,
        source_day_id,
        facts,
        current_facts,
        receipts,
    ))
}

/// 驗證日次與事件在整段推進中皆唯一，避免重播時出現身分歧義。
fn validate_sequence(source_day_id: &str, steps: &[AdvanceStep]) -> Result<()> {
    ensure!(!steps.is_empty(), "advance steps 不可為空");
    let mut day_ids = BTreeSet::from([source_day_id]);
    let mut transition_ids = BTreeSet::new();
    for step in steps {
        ensure!(
            day_ids.insert(step.target_day_id.as_str()),
            "世界日次不得重複：{}",
            step.target_day_id
        );
        for transition in &step.transitions {
            ensure!(
                transition_ids.insert(transition.id.as_str()),
                "跨日 transition id 不得重複：{}",
                transition.id
            );
        }
    }
    Ok(())
}

/// 彙整每步收據為一份完整時段證據，不省略起訖事實。
fn build_receipt(
    world_id: &str,
    source_day_id: &str,
    facts_before: &[Fact],
    facts_after: Vec<Fact>,
    steps: Vec<AdvanceReceipt>,
) -> WorldRunReceipt {
    let total_transition_count = steps
        .iter()
        .map(|receipt| receipt.applied_transitions.len())
        .sum();
    let total_changed_fact_count = steps.iter().map(|receipt| receipt.changed_fact_count).sum();
    let target_day_id = steps
        .last()
        .map(|receipt| receipt.target_day_id.clone())
        .unwrap_or_else(|| source_day_id.to_string());
    WorldRunReceipt {
        schema_version: 1,
        world_id: world_id.to_string(),
        source_day_id: source_day_id.to_string(),
        target_day_id,
        facts_before: facts_before.to_vec(),
        facts_after,
        steps,
        total_transition_count,
        total_changed_fact_count,
    }
}
