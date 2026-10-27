//! 將東岸體驗基線反查白塔半島 D2 提案，避免原型自行發明來源事實。

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};

use super::{ExperienceFixture, SourceEntity, SourceManifest};

/// 驗證來源身分、東岸引用與三項承重基線事實都存在且值相同。
pub(super) fn validate_source_link(
    fixture: &ExperienceFixture,
    source: &SourceManifest,
) -> Result<()> {
    let link = &fixture.source_world;
    ensure!(
        source.id == link.manifest_id,
        "體驗 fixture 指向錯誤來源世界"
    );
    ensure!(
        source.status == "working_proposal" && !source.canon,
        "白塔來源目前必須仍是非正典 working_proposal"
    );
    ensure_entity("region_ref", &link.region_ref, &source.regions)?;
    ensure_entity("resident_ref", &link.resident_ref, &source.residents)?;
    ensure_entity("site_ref", &link.site_ref, &source.objects)?;

    let required = BTreeSet::from([
        "F-EAST-MATERIAL".to_string(),
        "F-EAST-PATIENT".to_string(),
        "F-EAST-ROAD".to_string(),
    ]);
    let declared: BTreeSet<String> = link.baseline_fact_refs.iter().cloned().collect();
    ensure!(
        declared.len() == link.baseline_fact_refs.len() && declared == required,
        "東岸體驗基線必須恰好反查病人、道路與材料三項 D2 事實"
    );

    let day = source
        .current_state
        .continuation
        .iter()
        .find(|step| step.target_day_id == link.baseline_day_id)
        .ok_or_else(|| anyhow::anyhow!("白塔來源缺少基線日 {}", link.baseline_day_id))?;
    let source_values: BTreeMap<&str, &str> = day
        .expected
        .iter()
        .map(|condition| (condition.fact_ref.as_str(), condition.equals.as_str()))
        .collect();
    let fixture_values: BTreeMap<&str, &str> = fixture
        .facts
        .iter()
        .map(|fact| (fact.id.as_str(), fact.value.as_str()))
        .collect();
    for fact_ref in &link.baseline_fact_refs {
        let source_value = source_values
            .get(fact_ref.as_str())
            .copied()
            .ok_or_else(|| anyhow::anyhow!("白塔來源 D2 缺少事實：{fact_ref}"))?;
        let fixture_value = fixture_values
            .get(fact_ref.as_str())
            .copied()
            .ok_or_else(|| anyhow::anyhow!("體驗 fixture 缺少來源事實：{fact_ref}"))?;
        ensure!(
            source_value == fixture_value,
            "體驗基線與白塔來源不一致：{fact_ref} 來源為 {source_value}，fixture 為 {fixture_value}"
        );
    }
    Ok(())
}

fn ensure_entity(label: &str, id: &str, entities: &[SourceEntity]) -> Result<()> {
    ensure!(!id.trim().is_empty(), "{label} 不可空白");
    ensure!(
        entities.iter().any(|entity| entity.id == id),
        "{label} 引用不存在的白塔 id：{id}"
    );
    Ok(())
}
