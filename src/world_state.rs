//! 通用世界事實推進核心；不認識白塔、玩家表面、舊 runtime 或持久層。

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

mod sequence;
use sequence::ensure_nonblank;
pub use sequence::{AdvanceStep, WorldRunReceipt, advance_sequence};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Fact {
    pub id: String,
    pub subject_ref: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Condition {
    pub fact_ref: String,
    pub equals: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Effect {
    pub fact_ref: String,
    pub set: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Transition {
    pub id: String,
    pub order: u32,
    pub actor_refs: Vec<String>,
    pub object_refs: Vec<String>,
    pub region_refs: Vec<String>,
    pub preconditions: Vec<Condition>,
    pub effects: Vec<Effect>,
    pub consequence: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FactChange {
    pub fact_ref: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppliedTransition {
    pub id: String,
    pub order: u32,
    pub actor_refs: Vec<String>,
    pub object_refs: Vec<String>,
    pub region_refs: Vec<String>,
    pub consequence: String,
    pub changes: Vec<FactChange>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdvanceReceipt {
    pub schema_version: u32,
    pub world_id: String,
    pub source_day_id: String,
    pub target_day_id: String,
    pub facts_before: Vec<Fact>,
    pub facts_after: Vec<Fact>,
    pub applied_transitions: Vec<AppliedTransition>,
    pub changed_fact_count: usize,
}

/// 依明確順序套用轉換，輸入只讀，成功時回傳前後狀態與逐步收據。
pub fn advance(
    world_id: &str,
    source_day_id: &str,
    target_day_id: &str,
    facts: &[Fact],
    transitions: &[Transition],
) -> Result<AdvanceReceipt> {
    validate_run_identity(world_id, source_day_id, target_day_id)?;
    let (mut values, fact_order) = load_facts(facts)?;
    let ordered = order_transitions(transitions)?;
    let mut applied = Vec::with_capacity(ordered.len());

    for transition in ordered {
        applied.push(apply_transition(transition, &mut values)?);
    }

    let facts_after = materialize_facts(facts, &fact_order, &values)?;
    let changed_fact_count = applied.iter().map(|item| item.changes.len()).sum();
    Ok(AdvanceReceipt {
        schema_version: 1,
        world_id: world_id.to_string(),
        source_day_id: source_day_id.to_string(),
        target_day_id: target_day_id.to_string(),
        facts_before: facts.to_vec(),
        facts_after,
        applied_transitions: applied,
        changed_fact_count,
    })
}

/// 驗證一次推進的世界與日次身分，避免空白或原地推進產生假收據。
fn validate_run_identity(world_id: &str, source_day_id: &str, target_day_id: &str) -> Result<()> {
    ensure_nonblank("world_id", world_id)?;
    ensure_nonblank("source_day_id", source_day_id)?;
    ensure_nonblank("target_day_id", target_day_id)?;
    ensure!(
        source_day_id != target_day_id,
        "source_day_id 與 target_day_id 不得相同"
    );
    Ok(())
}

/// 驗證並載入初始事實，回傳依 id 索引的值與原始順序；不修改呼叫端資料。
fn load_facts(facts: &[Fact]) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    ensure!(!facts.is_empty(), "facts 不可為空");
    let mut values = BTreeMap::new();
    let mut order = Vec::with_capacity(facts.len());
    for fact in facts {
        validate_fact(fact)?;
        ensure!(
            values.insert(fact.id.clone(), fact.value.clone()).is_none(),
            "facts 出現重複 id：{}",
            fact.id
        );
        order.push(fact.id.clone());
    }
    Ok((values, order))
}

/// 驗證單一事實的識別、主體、鍵和值，沒有副作用。
fn validate_fact(fact: &Fact) -> Result<()> {
    ensure_nonblank("fact.id", &fact.id)?;
    ensure_nonblank(&format!("{}.subject_ref", fact.id), &fact.subject_ref)?;
    ensure_nonblank(&format!("{}.key", fact.id), &fact.key)?;
    ensure_nonblank(&format!("{}.value", fact.id), &fact.value)
}

/// 依 order 建立確定性執行序，拒絕重複 id 或重複順序造成的不確定性。
fn order_transitions(transitions: &[Transition]) -> Result<Vec<&Transition>> {
    ensure!(!transitions.is_empty(), "transitions 不可為空");
    let mut ids = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for transition in transitions {
        validate_transition_shape(transition)?;
        ensure!(
            ids.insert(&transition.id),
            "重複 transition id：{}",
            transition.id
        );
        ensure!(
            orders.insert(transition.order),
            "重複 transition order：{}",
            transition.order
        );
    }
    let mut ordered: Vec<&Transition> = transitions.iter().collect();
    ordered.sort_by_key(|transition| transition.order);
    Ok(ordered)
}

/// 驗證轉換本身有身分、前置、效果與後果；不檢查內容領域引用。
fn validate_transition_shape(transition: &Transition) -> Result<()> {
    ensure_nonblank("transition.id", &transition.id)?;
    ensure!(
        !transition.preconditions.is_empty(),
        "{} 必須有前置事實",
        transition.id
    );
    ensure!(
        !transition.effects.is_empty(),
        "{} 必須有實際效果",
        transition.id
    );
    ensure_nonblank(
        &format!("{}.consequence", transition.id),
        &transition.consequence,
    )
}

/// 先驗證全部前置，再一次套用效果；失敗時不會回傳部分收據。
fn apply_transition(
    transition: &Transition,
    values: &mut BTreeMap<String, String>,
) -> Result<AppliedTransition> {
    for condition in &transition.preconditions {
        validate_condition(&transition.id, condition, values)?;
    }
    let changes = apply_effects(transition, values)?;
    Ok(AppliedTransition {
        id: transition.id.clone(),
        order: transition.order,
        actor_refs: transition.actor_refs.clone(),
        object_refs: transition.object_refs.clone(),
        region_refs: transition.region_refs.clone(),
        consequence: transition.consequence.clone(),
        changes,
    })
}

/// 核對一項前置條件是否引用現有事實且值相符，不修改狀態。
fn validate_condition(
    owner: &str,
    condition: &Condition,
    values: &BTreeMap<String, String>,
) -> Result<()> {
    ensure_nonblank(&format!("{owner}.fact_ref"), &condition.fact_ref)?;
    ensure_nonblank(&format!("{owner}.equals"), &condition.equals)?;
    let actual = values
        .get(&condition.fact_ref)
        .ok_or_else(|| anyhow::anyhow!("{owner} 引用不存在的 fact：{}", condition.fact_ref))?;
    ensure!(
        actual == &condition.equals,
        "{owner} 前置不成立：{} 需要 {}，實際為 {}",
        condition.fact_ref,
        condition.equals,
        actual
    );
    Ok(())
}

/// 驗證同一轉換不重複寫同一事實，再依宣告順序更新工作狀態並回傳差異。
fn apply_effects(
    transition: &Transition,
    values: &mut BTreeMap<String, String>,
) -> Result<Vec<FactChange>> {
    let mut affected = BTreeSet::new();
    let mut changes = Vec::with_capacity(transition.effects.len());
    for effect in &transition.effects {
        ensure_nonblank(&format!("{}.fact_ref", transition.id), &effect.fact_ref)?;
        ensure_nonblank(&format!("{}.set", transition.id), &effect.set)?;
        ensure!(
            affected.insert(&effect.fact_ref),
            "{} 重複修改同一 fact：{}",
            transition.id,
            effect.fact_ref
        );
        changes.push(apply_effect(&transition.id, effect, values)?);
    }
    Ok(changes)
}

/// 將一項效果寫入工作狀態並回傳前後值；不存在或沒有改變都會拒絕。
fn apply_effect(
    owner: &str,
    effect: &Effect,
    values: &mut BTreeMap<String, String>,
) -> Result<FactChange> {
    let before = values
        .get(&effect.fact_ref)
        .ok_or_else(|| anyhow::anyhow!("{owner} effect 引用不存在的 fact：{}", effect.fact_ref))?;
    ensure!(
        before != &effect.set,
        "{owner} 對 {} 沒有造成改變",
        effect.fact_ref
    );
    let change = FactChange {
        fact_ref: effect.fact_ref.clone(),
        before: before.clone(),
        after: effect.set.clone(),
    };
    values.insert(effect.fact_ref.clone(), effect.set.clone());
    Ok(change)
}

/// 依原始事實順序組裝下一狀態，保留主體與鍵，只替換已推進的值。
fn materialize_facts(
    original: &[Fact],
    order: &[String],
    values: &BTreeMap<String, String>,
) -> Result<Vec<Fact>> {
    let by_id: BTreeMap<&str, &Fact> = original
        .iter()
        .map(|fact| (fact.id.as_str(), fact))
        .collect();
    order
        .iter()
        .map(|id| {
            let source = by_id
                .get(id.as_str())
                .ok_or_else(|| anyhow::anyhow!("找不到原始 fact：{id}"))?;
            let value = values
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("找不到推進後 fact：{id}"))?;
            Ok(Fact {
                value: value.clone(),
                ..(*source).clone()
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "world_state/tests.rs"]
mod tests;
