//! 白塔東岸體驗假說、兩條分支與中斷接續的完整驗證。

use std::collections::BTreeSet;

use anyhow::{Result, ensure};
use singularity_world::world_state::{Fact, Transition, advance_sequence};

use super::evidence::{compare_branches, value, verify_resume};
use super::projection::{render_projection, validate_projection_policy};
use super::source::validate_source_link;
use super::{
    ExperienceFixture, ExperienceVariant, ProofChecks, SourceManifest, ValidationOutput,
    VariantEvidence,
};

const PLAYER_LOCATION: &str = "F-EAST-PLAYER-LOCATION";
const REQUIRED_PLAYER_PRECONDITIONS: [&str; 4] = [
    PLAYER_LOCATION,
    "F-EAST-PLAYER-TIME",
    "F-EAST-PLAYER-PERMISSION",
    "F-EAST-PLAYER-CHOICE",
];

/// 完成資料邊界、來源、兩分支、投射、離線推進、回訪差異與重啟等價驗證。
pub(super) fn validate_experience(
    fixture: &ExperienceFixture,
    source: &SourceManifest,
) -> Result<ValidationOutput> {
    validate_fixture_identity(fixture)?;
    validate_source_link(fixture, source)?;
    let fact_ids = validate_facts(&fixture.facts)?;
    validate_projection_policy(&fixture.projection_policy, &fact_ids)?;
    let entry = render_projection(
        "抵達東岸",
        &fixture.entry_projection,
        &fixture.facts,
        &fixture.projection_policy,
    )?;
    let variant_ids: BTreeSet<&str> = fixture
        .variants
        .iter()
        .map(|variant| variant.id.as_str())
        .collect();
    ensure!(
        fixture.variants.len() == 2 && variant_ids == BTreeSet::from(["help", "leave"]),
        "體驗原型必須恰好包含 help 與 leave 兩條可比較分支"
    );

    let mut variants = Vec::with_capacity(2);
    for variant in &fixture.variants {
        variants.push(run_variant(fixture, variant, &fact_ids)?);
    }
    let help = variants
        .iter()
        .find(|variant| variant.id == "help")
        .ok_or_else(|| anyhow::anyhow!("缺少 help 證據"))?;
    let leave = variants
        .iter()
        .find(|variant| variant.id == "leave")
        .ok_or_else(|| anyhow::anyhow!("缺少 leave 證據"))?;
    let branch_differences = compare_branches(fixture, help, leave)?;
    ensure!(
        help.return_after_absence.lines != leave.return_after_absence.lines,
        "兩條分支回訪文字相同，體驗假說被否證"
    );
    ensure!(
        leave.offscreen_resident_transition_count >= fixture.minimum_offscreen_resident_transitions,
        "玩家離開後的居民行動不足，世界仍像在等待玩家"
    );

    Ok(ValidationOutput {
        verdict: "PROCEED",
        entry,
        variants,
        branch_differences,
        checks: ProofChecks {
            player_and_resident_share_transition_engine: true,
            player_action_changes_authoritative_facts: true,
            resident_progresses_while_player_is_absent: true,
            immediate_feedback_is_fact_backed: true,
            return_traces_are_fact_backed: true,
            required_branch_results_diverge: true,
            hidden_facts_are_not_projected: true,
            checkpoint_resume_is_equivalent: true,
            no_runtime_or_canon_promotion: true,
        },
    })
}

fn validate_fixture_identity(fixture: &ExperienceFixture) -> Result<()> {
    ensure!(fixture.schema_version == 1, "體驗 schema_version 必須為 1");
    ensure!(
        fixture.artifact_role == "experience_prototype_fixture",
        "artifact_role 不得冒充 runtime 或正典"
    );
    ensure!(
        fixture.status == "working_proposal" && !fixture.canon && !fixture.runtime_authority,
        "體驗 fixture 必須保持非正典、非 runtime 的 working_proposal"
    );
    ensure!(
        fixture.stage == "program_only_experience_proof",
        "本檢查器只接受 program_only_experience_proof"
    );
    for (label, value) in [
        ("id", fixture.id.as_str()),
        ("name", fixture.name.as_str()),
        ("hypothesis", fixture.hypothesis.as_str()),
        ("player_actor_ref", fixture.player_actor_ref.as_str()),
    ] {
        ensure!(!value.trim().is_empty(), "{label} 不可空白");
    }
    ensure!(
        !fixture.player_actor_ref.to_lowercase().contains("codex"),
        "永久角色 codex 不得被當成體驗 fixture"
    );
    ensure!(
        fixture.falsifiers.len() >= 4,
        "體驗假說必須有至少四條否證條件"
    );
    ensure!(
        fixture
            .falsifiers
            .iter()
            .all(|value| !value.trim().is_empty()),
        "否證條件不可空白"
    );
    ensure!(
        !fixture.proof_boundary.proven.is_empty() && !fixture.proof_boundary.not_proven.is_empty(),
        "必須同時聲明已證明與未證明邊界"
    );
    Ok(())
}

fn validate_facts(facts: &[Fact]) -> Result<BTreeSet<String>> {
    ensure!(!facts.is_empty(), "體驗 facts 不可為空");
    let mut ids = BTreeSet::new();
    for fact in facts {
        ensure!(!fact.id.trim().is_empty(), "fact id 不可空白");
        ensure!(ids.insert(fact.id.clone()), "重複 fact id：{}", fact.id);
    }
    Ok(ids)
}

fn run_variant(
    fixture: &ExperienceFixture,
    variant: &ExperienceVariant,
    fact_ids: &BTreeSet<String>,
) -> Result<VariantEvidence> {
    ensure!(
        variant.steps.len() >= 4,
        "{} 必須包含行動與至少三日推進",
        variant.id
    );
    validate_actor_flow(fixture, variant)?;
    let run = advance_sequence(
        &fixture.id,
        &fixture.source_world.baseline_day_id,
        &fixture.facts,
        &variant.steps,
    )?;
    validate_expected_final(variant, &run.facts_after, fact_ids)?;
    ensure!(
        value(&run.steps[0].facts_after, PLAYER_LOCATION)? == "away_from_east_coast",
        "{} 行動後玩家必須離開，才能驗證離線世界",
        variant.id
    );
    ensure!(
        value(&run.facts_after, PLAYER_LOCATION)? == "returned_to_east_coast",
        "{} 最終必須由玩家實際回訪",
        variant.id
    );
    let immediate = render_projection(
        format!("{}｜當下", variant.label),
        &variant.immediate_projection,
        &run.steps[0].facts_after,
        &fixture.projection_policy,
    )?;
    let return_after_absence = render_projection(
        format!("{}｜D5 回來", variant.label),
        &variant.return_projection,
        &run.facts_after,
        &fixture.projection_policy,
    )?;
    ensure!(
        immediate.lines.len() >= 2,
        "{} 缺少立即局部回饋",
        variant.id
    );
    ensure!(
        return_after_absence.lines.len() >= 3,
        "{} 缺少足以辨認非重置世界的回訪痕跡",
        variant.id
    );
    let resume = verify_resume(fixture, variant, &run)?;
    let offscreen_resident_transition_count = ordered_transitions(variant).len().saturating_sub(2);
    Ok(VariantEvidence {
        id: variant.id.clone(),
        label: variant.label.clone(),
        run,
        immediate,
        return_after_absence,
        offscreen_resident_transition_count,
        resume,
    })
}

fn validate_actor_flow(fixture: &ExperienceFixture, variant: &ExperienceVariant) -> Result<()> {
    let ordered = ordered_transitions(variant);
    ensure!(
        ordered.len() >= 5,
        "{} 沒有形成行動—離開—回訪鏈",
        variant.id
    );
    let action = ordered[0];
    let returning = ordered[ordered.len() - 1];
    ensure!(
        action.id == variant.action_transition_ref
            && action.actor_refs.contains(&fixture.player_actor_ref),
        "{} 第一個事件必須是玩家行動",
        variant.id
    );
    ensure!(
        returning.id == variant.return_transition_ref
            && returning.actor_refs == [fixture.player_actor_ref.as_str()],
        "{} 最後一個事件必須只有玩家回訪",
        variant.id
    );
    let action_conditions: BTreeSet<&str> = action
        .preconditions
        .iter()
        .map(|condition| condition.fact_ref.as_str())
        .collect();
    for required in REQUIRED_PLAYER_PRECONDITIONS {
        ensure!(
            action_conditions.contains(required),
            "{} 玩家行動缺少前置事實：{required}",
            variant.id
        );
    }
    for transition in &ordered[1..ordered.len() - 1] {
        ensure!(
            !transition.actor_refs.contains(&fixture.player_actor_ref),
            "{} 玩家離開後仍被偷偷當成行動者",
            variant.id
        );
        ensure!(
            transition
                .actor_refs
                .contains(&fixture.source_world.resident_ref),
            "{} 玩家離開後的事件不是由方石推進",
            variant.id
        );
    }
    Ok(())
}

fn ordered_transitions(variant: &ExperienceVariant) -> Vec<&Transition> {
    let mut result = Vec::new();
    for step in &variant.steps {
        let mut transitions: Vec<&Transition> = step.transitions.iter().collect();
        transitions.sort_by_key(|transition| transition.order);
        result.extend(transitions);
    }
    result
}

fn validate_expected_final(
    variant: &ExperienceVariant,
    facts: &[Fact],
    fact_ids: &BTreeSet<String>,
) -> Result<()> {
    let expected_ids: BTreeSet<String> = variant
        .expected_final
        .iter()
        .map(|condition| condition.fact_ref.clone())
        .collect();
    ensure!(
        expected_ids.len() == variant.expected_final.len() && expected_ids == *fact_ids,
        "{} expected_final 必須恰好覆蓋全部事實",
        variant.id
    );
    for condition in &variant.expected_final {
        ensure!(
            value(facts, &condition.fact_ref)? == condition.equals,
            "{} 最終事實不符：{}",
            variant.id,
            condition.fact_ref
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
