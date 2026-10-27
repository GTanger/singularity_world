//! 分支差異、事實查找與中途重啟證據。

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};
use singularity_world::world_state::{AdvanceStep, Fact, WorldRunReceipt, advance_sequence};

use super::{
    BranchDifference, ExperienceFixture, ExperienceVariant, ResumeEvidence, VariantEvidence,
};

/// 從 D3 日終快照重跑後兩步，核對最終事實與後段逐步收據完全一致。
pub(super) fn verify_resume(
    fixture: &ExperienceFixture,
    variant: &ExperienceVariant,
    full: &WorldRunReceipt,
) -> Result<ResumeEvidence> {
    let checkpoint_index = 1;
    let checkpoint = &full.steps[checkpoint_index];
    let remaining: &[AdvanceStep] = &variant.steps[checkpoint_index + 1..];
    let resumed = advance_sequence(
        &fixture.id,
        &checkpoint.target_day_id,
        &checkpoint.facts_after,
        remaining,
    )?;
    ensure!(
        resumed.facts_after == full.facts_after,
        "{} 重啟後結局不同",
        variant.id
    );
    ensure!(
        resumed.steps == full.steps[checkpoint_index + 1..],
        "{} 重啟後的後段收據不同",
        variant.id
    );
    Ok(ResumeEvidence {
        checkpoint_day_id: checkpoint.target_day_id.clone(),
        resumed_steps: resumed.steps.len(),
        final_facts_match: true,
        suffix_receipts_match: true,
    })
}

/// 逐項核對必須分歧的世界與玩家事實，不接受只改選項標籤的假分支。
pub(super) fn compare_branches(
    fixture: &ExperienceFixture,
    help: &VariantEvidence,
    leave: &VariantEvidence,
) -> Result<Vec<BranchDifference>> {
    let required: BTreeSet<&str> = fixture
        .required_branch_difference_refs
        .iter()
        .map(String::as_str)
        .collect();
    ensure!(
        required.len() == fixture.required_branch_difference_refs.len() && required.len() >= 4,
        "required_branch_difference_refs 必須唯一且同時覆蓋世界與玩家代價"
    );
    let help_values = values(&help.run.facts_after);
    let leave_values = values(&leave.run.facts_after);
    let mut differences = Vec::new();
    for fact_ref in &fixture.required_branch_difference_refs {
        let help_value = help_values
            .get(fact_ref.as_str())
            .ok_or_else(|| anyhow::anyhow!("help 缺少比較事實：{fact_ref}"))?;
        let leave_value = leave_values
            .get(fact_ref.as_str())
            .ok_or_else(|| anyhow::anyhow!("leave 缺少比較事實：{fact_ref}"))?;
        ensure!(
            help_value != leave_value,
            "必要分支事實沒有差異：{fact_ref}；體驗假說被否證"
        );
        differences.push(BranchDifference {
            fact_ref: fact_ref.clone(),
            help: (*help_value).to_string(),
            leave: (*leave_value).to_string(),
        });
    }
    Ok(differences)
}

pub(super) fn value<'a>(facts: &'a [Fact], fact_ref: &str) -> Result<&'a str> {
    facts
        .iter()
        .find(|fact| fact.id == fact_ref)
        .map(|fact| fact.value.as_str())
        .ok_or_else(|| anyhow::anyhow!("找不到事實：{fact_ref}"))
}

fn values(facts: &[Fact]) -> BTreeMap<&str, &str> {
    facts
        .iter()
        .map(|fact| (fact.id.as_str(), fact.value.as_str()))
        .collect()
}
