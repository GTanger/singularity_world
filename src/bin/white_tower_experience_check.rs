#![forbid(unsafe_code)]
//! 白塔東岸「行動—離開—回訪」程式體驗檢查器。
//!
//! 這不是正式 UI 或 runtime。它讓玩家與居民共用 `world_state` 的事實轉換，
//! 再驗證玩家離開後居民會繼續行動、兩種選擇留下不同世界結果，且玩家文字
//! 只投射當地可知事實。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use singularity_world::world_state::{AdvanceStep, Condition, Fact, WorldRunReceipt};

#[path = "white_tower_experience_check/command.rs"]
mod command;
#[path = "white_tower_experience_check/evidence.rs"]
mod evidence;
#[path = "white_tower_experience_check/projection.rs"]
mod projection;
#[path = "white_tower_experience_check/receipt.rs"]
mod receipt;
#[path = "white_tower_experience_check/source.rs"]
mod source;
#[path = "white_tower_experience_check/validation.rs"]
mod validation;

use command::parse_args;
use receipt::{ReceiptWriteStatus, write_verification_receipt};
use validation::validate_experience;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ExperienceFixture {
    schema_version: u32,
    id: String,
    name: String,
    artifact_role: String,
    status: String,
    canon: bool,
    runtime_authority: bool,
    stage: String,
    source_world: SourceWorld,
    player_actor_ref: String,
    hypothesis: String,
    falsifiers: Vec<String>,
    proof_boundary: ProofBoundary,
    minimum_offscreen_resident_transitions: usize,
    required_branch_difference_refs: Vec<String>,
    facts: Vec<Fact>,
    entry_projection: Vec<ProjectionLine>,
    variants: Vec<ExperienceVariant>,
    projection_policy: ProjectionPolicy,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SourceWorld {
    manifest_id: String,
    baseline_day_id: String,
    region_ref: String,
    resident_ref: String,
    site_ref: String,
    baseline_fact_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ProofBoundary {
    proven: Vec<String>,
    not_proven: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ExperienceVariant {
    id: String,
    label: String,
    action_transition_ref: String,
    return_transition_ref: String,
    steps: Vec<AdvanceStep>,
    immediate_projection: Vec<ProjectionLine>,
    return_projection: Vec<ProjectionLine>,
    expected_final: Vec<Condition>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct ProjectionLine {
    fact_ref: String,
    equals: String,
    text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ProjectionPolicy {
    allowed_fact_refs: Vec<String>,
    hidden_fact_refs: Vec<String>,
    forbidden_visible_tokens: Vec<String>,
    invariant: String,
}

#[derive(Debug, Deserialize)]
struct SourceManifest {
    id: String,
    status: String,
    canon: bool,
    regions: Vec<SourceEntity>,
    residents: Vec<SourceEntity>,
    objects: Vec<SourceEntity>,
    current_state: SourceCurrentState,
}

#[derive(Debug, Deserialize)]
struct SourceEntity {
    id: String,
}

#[derive(Debug, Deserialize)]
struct SourceCurrentState {
    continuation: Vec<SourceContinuation>,
}

#[derive(Debug, Deserialize)]
struct SourceContinuation {
    target_day_id: String,
    expected: Vec<Condition>,
}

#[derive(Debug, Serialize)]
struct ValidationOutput {
    verdict: &'static str,
    entry: TranscriptPhase,
    variants: Vec<VariantEvidence>,
    branch_differences: Vec<BranchDifference>,
    checks: ProofChecks,
}

#[derive(Debug, Serialize)]
struct VariantEvidence {
    id: String,
    label: String,
    run: WorldRunReceipt,
    immediate: TranscriptPhase,
    return_after_absence: TranscriptPhase,
    offscreen_resident_transition_count: usize,
    resume: ResumeEvidence,
}

#[derive(Debug, Serialize)]
struct TranscriptPhase {
    label: String,
    lines: Vec<ProjectionLine>,
}

#[derive(Debug, Serialize)]
struct ResumeEvidence {
    checkpoint_day_id: String,
    resumed_steps: usize,
    final_facts_match: bool,
    suffix_receipts_match: bool,
}

#[derive(Debug, Serialize)]
struct BranchDifference {
    fact_ref: String,
    help: String,
    leave: String,
}

#[derive(Debug, Serialize)]
struct ProofChecks {
    player_and_resident_share_transition_engine: bool,
    player_action_changes_authoritative_facts: bool,
    resident_progresses_while_player_is_absent: bool,
    immediate_feedback_is_fact_backed: bool,
    return_traces_are_fact_backed: bool,
    required_branch_results_diverge: bool,
    hidden_facts_are_not_projected: bool,
    checkpoint_resume_is_equivalent: bool,
    no_runtime_or_canon_promotion: bool,
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let fixture: ExperienceFixture = load_json(&args.fixture_path, "東岸體驗 fixture")?;
    let source: SourceManifest = load_json(&args.source_path, "白塔來源 manifest")?;
    let output = validate_experience(&fixture, &source)?;

    println!("white_tower_experience_check: ok");
    println!(
        "stage={} verdict={} canon={} runtime_authority={}",
        fixture.stage, output.verdict, fixture.canon, fixture.runtime_authority
    );
    println!(
        "shared_transition_engine=true variants={} branch_differences={}",
        output.variants.len(),
        output.branch_differences.len()
    );
    print_phase(&output.entry);
    for variant in &output.variants {
        print_phase(&variant.immediate);
        print_phase(&variant.return_after_absence);
    }

    if let Some(path) = args.receipt_path {
        let status = write_verification_receipt(&path, &fixture, &output)?;
        let label = match status {
            ReceiptWriteStatus::Created => "created",
            ReceiptWriteStatus::Unchanged => "unchanged",
        };
        println!("verification_receipt={label} path={}", path.display());
    }
    Ok(())
}

fn load_json<T>(path: &Path, label: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let raw =
        fs::read_to_string(path).with_context(|| format!("無法讀取{label}：{}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("{label}不是合法 JSON：{}", path.display()))
}

fn print_phase(phase: &TranscriptPhase) {
    println!("\n【{}】", phase.label);
    for line in &phase.lines {
        println!("{}", line.text);
    }
}
