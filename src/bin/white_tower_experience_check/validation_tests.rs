//! 白塔東岸玩家行動、離線居民、投射與否證測試。

use std::path::PathBuf;

use anyhow::Result;

use super::super::{ExperienceFixture, SourceManifest, load_json};
use super::*;

fn fixture() -> Result<ExperienceFixture> {
    load_json(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/world/white_tower_east_coast_experience_v1.json"),
        "測試體驗 fixture",
    )
}

fn source() -> Result<SourceManifest> {
    load_json(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/world/white_tower_peninsula_v1.json"),
        "測試白塔來源",
    )
}

#[test]
fn bundled_fixture_closes_player_action_absence_and_return() -> Result<()> {
    let output = validate_experience(&fixture()?, &source()?)?;
    assert_eq!(output.verdict, "PROCEED");
    assert_eq!(output.variants.len(), 2);
    assert_eq!(output.branch_differences.len(), 7);
    assert!(
        output
            .variants
            .iter()
            .all(|variant| variant.resume.final_facts_match)
    );
    let leave = output
        .variants
        .iter()
        .find(|variant| variant.id == "leave")
        .expect("leave 分支必須存在");
    assert_eq!(leave.offscreen_resident_transition_count, 3);
    Ok(())
}

#[test]
fn rejects_player_action_when_local_precondition_is_false() -> Result<()> {
    let mut input = fixture()?;
    input
        .facts
        .iter_mut()
        .find(|fact| fact.id == PLAYER_LOCATION)
        .expect("玩家位置事實必須存在")
        .value = "somewhere_else".to_string();
    let error =
        validate_experience(&input, &source()?).expect_err("玩家不在現場時不得遠端幫忙或離開");
    assert!(error.to_string().contains("前置不成立"));
    Ok(())
}

#[test]
fn rejects_player_as_hidden_offscreen_worker() -> Result<()> {
    let mut input = fixture()?;
    let leave = input
        .variants
        .iter_mut()
        .find(|variant| variant.id == "leave")
        .expect("leave 分支必須存在");
    leave.steps[1].transitions[0]
        .actor_refs
        .push(input.player_actor_ref.clone());
    let error =
        validate_experience(&input, &source()?).expect_err("玩家離開後不得被偷偷用來推進工作");
    assert!(error.to_string().contains("偷偷當成行動者"));
    Ok(())
}

#[test]
fn rejects_visible_line_not_backed_by_current_fact() -> Result<()> {
    let mut input = fixture()?;
    let help = input
        .variants
        .iter_mut()
        .find(|variant| variant.id == "help")
        .expect("help 分支必須存在");
    help.return_projection[1].equals = "fully_open_without_limits".to_string();
    let error =
        validate_experience(&input, &source()?).expect_err("玩家文字不得比世界事實多說一步");
    assert!(error.to_string().contains("沒有事實支撐"));
    Ok(())
}

#[test]
fn rejects_required_branch_fact_that_does_not_diverge() -> Result<()> {
    let mut input = fixture()?;
    input.required_branch_difference_refs = vec!["F-EAST-PATIENT".to_string()];
    let error =
        validate_experience(&input, &source()?).expect_err("兩分支相同的病人事實不能冒充玩家後果");
    assert!(
        error
            .to_string()
            .contains("required_branch_difference_refs")
    );
    Ok(())
}

#[test]
fn rejects_codex_as_disposable_player_fixture() -> Result<()> {
    let mut input = fixture()?;
    input.player_actor_ref = "codex".to_string();
    let error =
        validate_experience(&input, &source()?).expect_err("永久角色 codex 不能被重置成測試玩家");
    assert!(error.to_string().contains("永久角色 codex"));
    Ok(())
}
