//! 白塔提案檢查器共用的非空、唯一身分與引用驗證。

use std::collections::BTreeSet;

use anyhow::{Result, bail, ensure};

/// 將一組字串身分驗證為非空且唯一，回傳可供引用檢查的排序集合。
pub(super) fn unique_ids<'a>(
    label: &str,
    ids: impl Iterator<Item = &'a String>,
) -> Result<BTreeSet<String>> {
    let mut result = BTreeSet::new();
    for id in ids {
        ensure_nonblank(label, id)?;
        ensure!(result.insert(id.clone()), "{label} 出現重複 id：{id}");
    }
    ensure!(!result.is_empty(), "{label} 不可為空");
    Ok(result)
}

/// 驗證單一引用非空且存在於允許集合，不修改任何資料。
pub(super) fn ensure_ref(label: &str, value: &str, allowed: &BTreeSet<String>) -> Result<()> {
    ensure_nonblank(label, value)?;
    ensure!(allowed.contains(value), "{label} 引用不存在的 id：{value}");
    Ok(())
}

/// 驗證非空引用陣列中的每個值都存在且不重複。
pub(super) fn ensure_refs(
    label: &str,
    values: &[String],
    allowed: &BTreeSet<String>,
) -> Result<()> {
    ensure!(!values.is_empty(), "{label} 不可為空");
    ensure_refs_allow_empty(label, values, allowed)
}

/// 驗證可為空的引用陣列，內容若存在仍必須有效且唯一。
pub(super) fn ensure_refs_allow_empty(
    label: &str,
    values: &[String],
    allowed: &BTreeSet<String>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        ensure_ref(label, value, allowed)?;
        ensure!(seen.insert(value), "{label} 重複引用：{value}");
    }
    Ok(())
}

/// 驗證文字陣列非空，且每一個文字值都不是空白。
pub(super) fn ensure_texts(label: &str, values: &[String]) -> Result<()> {
    ensure!(!values.is_empty(), "{label} 不可為空");
    for value in values {
        ensure_nonblank(label, value)?;
    }
    Ok(())
}

/// 驗證一個業務字串不是空白，不產生副作用。
pub(super) fn ensure_nonblank(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} 不可為空白");
    }
    Ok(())
}
