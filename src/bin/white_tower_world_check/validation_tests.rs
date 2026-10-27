//! 白塔半島七日政策、接續與拒絕路徑測試。

use super::super::{Manifest, validate_manifest};
use super::*;

/// 從專案內建資料建立每個測試獨立擁有的白塔 manifest。
fn bundled_manifest() -> Result<Manifest> {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/world/white_tower_peninsula_v1.json"
    )))
    .context("內建白塔半島 manifest 解析失敗")
}

/// 驗證白塔政策層與通用引擎共同完成 D0 至 D7，且每日收據完整保留。
#[test]
fn bundled_manifest_closes_d0_to_d7() -> Result<()> {
    let manifest = bundled_manifest()?;
    let output = validate_manifest(&manifest)?;
    assert_eq!(output.summary.population, 18_000_000);
    assert_eq!(output.summary.regions, 9);
    assert_eq!(output.summary.days, 7);
    assert_eq!(output.run.source_day_id, "D0");
    assert_eq!(output.run.target_day_id, "D7");
    assert_eq!(output.run.steps.len(), 7);
    assert_eq!(output.run.facts_before.len(), 12);
    assert_eq!(output.run.facts_after.len(), 12);
    Ok(())
}

/// 驗證 D3 快照後重新啟動，可只跑 D4 至 D7 並得到同一最終世界狀態。
#[test]
fn bundled_manifest_resumes_from_d3() -> Result<()> {
    let manifest = bundled_manifest()?;
    let output = validate_manifest(&manifest)?;
    let steps = build_advance_steps(&manifest.current_state);
    let resumed = advance_sequence(
        &manifest.id,
        "D3",
        &output.run.steps[2].facts_after,
        &steps[3..],
    )?;
    assert_eq!(resumed.target_day_id, "D7");
    assert_eq!(resumed.facts_after, output.run.facts_after);
    Ok(())
}

/// 驗證地區人口不再平衡時，白塔政策層在推進前拒絕資料。
#[test]
fn rejects_population_that_no_longer_balances() -> Result<()> {
    let mut manifest = bundled_manifest()?;
    manifest.regions[0].population += 1;
    let error = validate_manifest(&manifest).expect_err("人口不平衡應被拒絕");
    assert!(error.to_string().contains("人口加總"));
    Ok(())
}

/// 驗證後段錯誤前置條件由通用引擎拒絕，不能產生假的完整七日收據。
#[test]
fn rejects_a_transition_with_false_precondition() -> Result<()> {
    let mut manifest = bundled_manifest()?;
    manifest.current_state.continuation[4].transitions[0].preconditions[0].equals =
        "unobserved_guess".to_string();
    let error = validate_manifest(&manifest).expect_err("錯誤前置條件應被拒絕");
    assert!(error.to_string().contains("前置"));
    Ok(())
}

/// 驗證承重居民位置若引用不存在地區，白塔政策層會拒絕資料。
#[test]
fn rejects_unknown_resident_location() -> Result<()> {
    let mut manifest = bundled_manifest()?;
    manifest.residents[0].current_region_ref = "BT-R-MISSING".to_string();
    let error = validate_manifest(&manifest).expect_err("不存在地區應被拒絕");
    assert!(error.to_string().contains("引用不存在的 id"));
    Ok(())
}
