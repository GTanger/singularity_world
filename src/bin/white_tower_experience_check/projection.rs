//! 將權威事實投射為玩家可見的局部文字，並拒絕無事實支撐或越界內容。

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};
use singularity_world::world_state::Fact;

use super::{ProjectionLine, ProjectionPolicy, TranscriptPhase};

/// 驗證可見與隱藏事實恰好分割本原型的全部事實，避免默認洩漏或無主資料。
pub(super) fn validate_projection_policy(
    policy: &ProjectionPolicy,
    fact_ids: &BTreeSet<String>,
) -> Result<()> {
    ensure!(
        !policy.invariant.trim().is_empty(),
        "projection invariant 不可空白"
    );
    ensure!(
        !policy.forbidden_visible_tokens.is_empty(),
        "必須宣告玩家文字禁用標記"
    );
    let allowed = unique("allowed_fact_refs", &policy.allowed_fact_refs)?;
    let hidden = unique("hidden_fact_refs", &policy.hidden_fact_refs)?;
    ensure!(!hidden.is_empty(), "本體驗必須至少保留一項未取得的隱藏事實");
    ensure!(allowed.is_disjoint(&hidden), "可見與隱藏事實不得重疊");
    let partition: BTreeSet<String> = allowed.union(&hidden).cloned().collect();
    ensure!(
        partition == *fact_ids,
        "projection policy 必須恰好分類全部體驗事實"
    );
    for token in &policy.forbidden_visible_tokens {
        ensure!(!token.trim().is_empty(), "玩家文字禁用標記不可空白");
    }
    Ok(())
}

/// 逐行核對事實、知識邊界與禁用字，再保留原順序形成可檢視文字。
pub(super) fn render_projection(
    label: impl Into<String>,
    declared: &[ProjectionLine],
    facts: &[Fact],
    policy: &ProjectionPolicy,
) -> Result<TranscriptPhase> {
    ensure!(!declared.is_empty(), "玩家可見階段不可沒有文字");
    let values: BTreeMap<&str, &str> = facts
        .iter()
        .map(|fact| (fact.id.as_str(), fact.value.as_str()))
        .collect();
    let allowed: BTreeSet<&str> = policy
        .allowed_fact_refs
        .iter()
        .map(String::as_str)
        .collect();
    let hidden: BTreeSet<&str> = policy.hidden_fact_refs.iter().map(String::as_str).collect();
    let mut used = BTreeSet::new();

    for line in declared {
        ensure!(
            used.insert(line.fact_ref.as_str()),
            "同一玩家文字階段重複使用事實：{}",
            line.fact_ref
        );
        ensure!(
            allowed.contains(line.fact_ref.as_str()),
            "玩家文字引用未允許事實：{}",
            line.fact_ref
        );
        ensure!(
            !hidden.contains(line.fact_ref.as_str()),
            "玩家文字洩漏隱藏事實：{}",
            line.fact_ref
        );
        let actual = values
            .get(line.fact_ref.as_str())
            .copied()
            .ok_or_else(|| anyhow::anyhow!("玩家文字引用不存在事實：{}", line.fact_ref))?;
        ensure!(
            actual == line.equals,
            "玩家文字沒有事實支撐：{} 需要 {}，實際為 {}",
            line.fact_ref,
            line.equals,
            actual
        );
        validate_visible_text(&line.text, policy)?;
    }
    Ok(TranscriptPhase {
        label: label.into(),
        lines: declared.to_vec(),
    })
}

/// 玩家文字必須是單段自然文字，且不含後端識別或世界狀態看板措辭。
fn validate_visible_text(text: &str, policy: &ProjectionPolicy) -> Result<()> {
    ensure!(!text.trim().is_empty(), "玩家可見文字不可空白");
    ensure!(text.trim() == text, "玩家可見文字不得含前後空白");
    ensure!(!text.contains(['\n', '\r']), "玩家可見文字必須逐行宣告");
    for token in &policy.forbidden_visible_tokens {
        ensure!(!text.contains(token), "玩家可見文字含禁用標記：{token}");
    }
    Ok(())
}

/// 建立不重複且非空白的字串集合。
fn unique(label: &str, values: &[String]) -> Result<BTreeSet<String>> {
    let mut result = BTreeSet::new();
    for value in values {
        ensure!(!value.trim().is_empty(), "{label} 不可含空白值");
        ensure!(result.insert(value.clone()), "{label} 出現重複值：{value}");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ProjectionPolicy {
        ProjectionPolicy {
            allowed_fact_refs: vec!["road".to_string()],
            hidden_fact_refs: vec!["patient".to_string()],
            forbidden_visible_tokens: vec!["F-".to_string()],
            invariant: "只投射可知事實".to_string(),
        }
    }

    fn facts() -> Vec<Fact> {
        vec![
            Fact {
                id: "road".to_string(),
                subject_ref: "site".to_string(),
                key: "state".to_string(),
                value: "open".to_string(),
            },
            Fact {
                id: "patient".to_string(),
                subject_ref: "site".to_string(),
                key: "state".to_string(),
                value: "safe".to_string(),
            },
        ]
    }

    #[test]
    fn renders_only_fact_backed_visible_line() -> Result<()> {
        let ids = facts().iter().map(|fact| fact.id.clone()).collect();
        validate_projection_policy(&policy(), &ids)?;
        let phase = render_projection(
            "回訪",
            &[ProjectionLine {
                fact_ref: "road".to_string(),
                equals: "open".to_string(),
                text: "路已經通了。".to_string(),
            }],
            &facts(),
            &policy(),
        )?;
        assert_eq!(phase.lines.len(), 1);
        Ok(())
    }

    #[test]
    fn rejects_hidden_fact_even_when_value_is_true() {
        let error = render_projection(
            "回訪",
            &[ProjectionLine {
                fact_ref: "patient".to_string(),
                equals: "safe".to_string(),
                text: "病人平安。".to_string(),
            }],
            &facts(),
            &policy(),
        )
        .expect_err("未取得的病人消息不得顯示");
        assert!(error.to_string().contains("未允許事實"));
    }
}
