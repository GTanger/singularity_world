//! 通用世界事實推進核心的確定性與拒絕路徑測試。

use super::*;

/// 建立兩項事實，供每個測試獨立擁有並修改。
fn facts() -> Vec<Fact> {
    vec![
        Fact {
            id: "road".to_string(),
            subject_ref: "route-1".to_string(),
            key: "capacity".to_string(),
            value: "heavy".to_string(),
        },
        Fact {
            id: "cargo".to_string(),
            subject_ref: "batch-1".to_string(),
            key: "location".to_string(),
            value: "lowland".to_string(),
        },
    ]
}

/// 建立兩段相依轉換，第二段必須讀到第一段剛完成的狀態。
fn transitions() -> Vec<Transition> {
    vec![
        Transition {
            id: "move-cargo".to_string(),
            order: 20,
            actor_refs: vec!["worker-2".to_string()],
            object_refs: vec!["batch-1".to_string()],
            region_refs: vec!["region-1".to_string()],
            preconditions: vec![Condition {
                fact_ref: "road".to_string(),
                equals: "light".to_string(),
            }],
            effects: vec![Effect {
                fact_ref: "cargo".to_string(),
                set: "highland".to_string(),
            }],
            consequence: "貨物離開低地".to_string(),
        },
        Transition {
            id: "limit-road".to_string(),
            order: 10,
            actor_refs: vec!["worker-1".to_string()],
            object_refs: vec!["route-1".to_string()],
            region_refs: vec!["region-1".to_string()],
            preconditions: vec![Condition {
                fact_ref: "road".to_string(),
                equals: "heavy".to_string(),
            }],
            effects: vec![Effect {
                fact_ref: "road".to_string(),
                set: "light".to_string(),
            }],
            consequence: "道路完成限載".to_string(),
        },
    ]
}

/// 驗證引擎依 order 而非輸入排列執行，並完整保留前後事實與差異。
#[test]
fn advances_in_declared_order_and_records_receipt() -> Result<()> {
    let receipt = advance("world-1", "D0", "D1", &facts(), &transitions())?;
    assert_eq!(receipt.applied_transitions[0].id, "limit-road");
    assert_eq!(receipt.applied_transitions[1].id, "move-cargo");
    assert_eq!(receipt.changed_fact_count, 2);
    assert_eq!(receipt.facts_before[1].value, "lowland");
    assert_eq!(receipt.facts_after[1].value, "highland");
    Ok(())
}

/// 驗證相同輸入會得到逐位元可重現的同一份 JSON 收據。
#[test]
fn same_input_is_byte_deterministic() -> Result<()> {
    let first = advance("world-1", "D0", "D1", &facts(), &transitions())?;
    let second = advance("world-1", "D0", "D1", &facts(), &transitions())?;
    assert_eq!(first, second);
    assert_eq!(serde_json::to_vec(&first)?, serde_json::to_vec(&second)?);
    Ok(())
}

/// 驗證錯誤前置條件會讓整次推進失敗，而不是略過該步繼續。
#[test]
fn rejects_false_precondition() {
    let mut input = transitions();
    input[1].preconditions[0].equals = "closed".to_string();
    let error =
        advance("world-1", "D0", "D1", &facts(), &input).expect_err("錯誤前置條件應拒絕整次推進");
    assert!(error.to_string().contains("前置不成立"));
}

/// 驗證同一轉換不能對同一事實宣告兩個結果，避免順序暗藏語意。
#[test]
fn rejects_duplicate_effect_for_same_fact() {
    let mut input = transitions();
    input[1].effects.push(Effect {
        fact_ref: "road".to_string(),
        set: "closed".to_string(),
    });
    let error = advance("world-1", "D0", "D1", &facts(), &input)
        .expect_err("同一步重複修改同一事實應被拒絕");
    assert!(error.to_string().contains("重複修改"));
}

/// 驗證重複順序會被拒絕，不讓輸入陣列位置偷偷決定世界結果。
#[test]
fn rejects_duplicate_transition_order() {
    let mut input = transitions();
    input[0].order = input[1].order;
    let error = advance("world-1", "D0", "D1", &facts(), &input).expect_err("重複順序應被拒絕");
    assert!(error.to_string().contains("重複 transition order"));
}

/// 建立三天相依步驟，供多日、接續與失敗原子性測試共用。
fn sequence_steps() -> Vec<AdvanceStep> {
    vec![
        AdvanceStep {
            target_day_id: "D1".to_string(),
            transitions: transitions(),
        },
        AdvanceStep {
            target_day_id: "D2".to_string(),
            transitions: vec![Transition {
                id: "close-road".to_string(),
                order: 1,
                actor_refs: vec!["worker-1".to_string()],
                object_refs: vec!["route-1".to_string()],
                region_refs: vec!["region-1".to_string()],
                preconditions: vec![Condition {
                    fact_ref: "road".to_string(),
                    equals: "light".to_string(),
                }],
                effects: vec![Effect {
                    fact_ref: "road".to_string(),
                    set: "closed".to_string(),
                }],
                consequence: "道路完成封閉".to_string(),
            }],
        },
    ]
}

/// 驗證多日結果可由中途快照重新接續，且與一次跑完全段相同。
#[test]
fn sequence_can_resume_from_checkpoint() -> Result<()> {
    let steps = sequence_steps();
    let full = advance_sequence("world-1", "D0", &facts(), &steps)?;
    let resumed = advance_sequence("world-1", "D1", &full.steps[0].facts_after, &steps[1..])?;
    assert_eq!(full.target_day_id, "D2");
    assert_eq!(full.facts_after, resumed.facts_after);
    assert_eq!(full.total_transition_count, 3);
    assert_eq!(full.total_changed_fact_count, 3);
    Ok(())
}

/// 驗證後段錯誤不會修改呼叫端起始事實，也不會回傳半份多日收據。
#[test]
fn sequence_failure_leaves_input_unchanged() {
    let original = facts();
    let mut steps = sequence_steps();
    steps[1].transitions[0].preconditions[0].equals = "open".to_string();
    let error =
        advance_sequence("world-1", "D0", &original, &steps).expect_err("後段錯誤應拒絕完整時段");
    assert!(error.to_string().contains("前置不成立"));
    assert_eq!(original, facts());
}

/// 驗證多日重複事件身分會被拒絕，不能在收據中冒充兩次不同事件。
#[test]
fn sequence_rejects_duplicate_transition_identity() {
    let mut steps = sequence_steps();
    steps[1].transitions[0].id = steps[0].transitions[0].id.clone();
    let error =
        advance_sequence("world-1", "D0", &facts(), &steps).expect_err("跨日重複事件身分應被拒絕");
    assert!(error.to_string().contains("跨日 transition id"));
}
