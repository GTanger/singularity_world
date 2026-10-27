//! 白塔半島工作提案的推進驗證收據輸出。
//!
//! 收據是可重放的工程證據，不是 PostgreSQL runtime 權威，也不能升格正典。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use singularity_world::world_state::WorldRunReceipt;

use super::{Manifest, ResumeEvidence};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ReceiptWriteStatus {
    Created,
    Unchanged,
}

#[derive(Serialize)]
struct EvidenceReceipt<'a> {
    schema_version: u32,
    artifact_role: &'static str,
    runtime_authority: bool,
    source_manifest: SourceManifest<'a>,
    run: &'a WorldRunReceipt,
    resume_verification: &'a ResumeEvidence,
}

#[derive(Serialize)]
struct SourceManifest<'a> {
    id: &'a str,
    name: &'a str,
    status: &'a str,
    canon: bool,
}

/// 將驗證結果序列化為穩定 JSON；只建立新檔或接受完全相同的既有檔。
pub(super) fn write_verification_receipt(
    path: &Path,
    manifest: &Manifest,
    run: &WorldRunReceipt,
    resume: &ResumeEvidence,
) -> Result<ReceiptWriteStatus> {
    ensure_evidence_path(path)?;
    let evidence = EvidenceReceipt {
        schema_version: 2,
        artifact_role: "verification_evidence",
        runtime_authority: false,
        source_manifest: SourceManifest {
            id: &manifest.id,
            name: &manifest.name,
            status: &manifest.status,
            canon: manifest.canon,
        },
        run,
        resume_verification: resume,
    };
    let mut bytes = serde_json::to_vec_pretty(&evidence).context("驗證收據序列化失敗")?;
    bytes.push(b'\n');
    write_new_or_identical(path, &bytes)
}

/// 確認輸出父目錄存在且不屬於 data/runtime，避免證據檔冒充遊戲權威狀態。
fn ensure_evidence_path(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .context("收據路徑必須包含既有父目錄")?;
    let parent = parent
        .canonicalize()
        .with_context(|| format!("收據父目錄不存在：{}", parent.display()))?;
    let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/runtime")
        .canonicalize()
        .context("找不到 data/runtime 邊界")?;
    ensure!(
        !parent.starts_with(runtime),
        "驗證收據不得寫入 data/runtime；runtime 權威只認 PostgreSQL"
    );
    Ok(())
}

/// 若目標已存在，只接受內容完全相同；新檔以同目錄暫存再 rename，避免半寫收據。
fn write_new_or_identical(path: &Path, bytes: &[u8]) -> Result<ReceiptWriteStatus> {
    if path.exists() {
        let existing =
            fs::read(path).with_context(|| format!("無法讀取既有收據：{}", path.display()))?;
        ensure!(
            existing == bytes,
            "既有收據內容不同，拒絕直接覆寫：{}",
            path.display()
        );
        return Ok(ReceiptWriteStatus::Unchanged);
    }

    let temporary = temporary_path(path)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| format!("無法建立暫存收據：{}", temporary.display()))?;
    file.write_all(bytes).context("寫入暫存收據失敗")?;
    file.sync_all().context("同步暫存收據失敗")?;
    drop(file);
    rename_or_cleanup(&temporary, path)?;
    Ok(ReceiptWriteStatus::Created)
}

/// 由目標檔名與目前行程建立同目錄暫存路徑，不建立任何檔案。
fn temporary_path(path: &Path) -> Result<PathBuf> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .context("收據檔名必須是有效 UTF-8")?;
    Ok(path.with_file_name(format!(".{name}.{}.tmp", std::process::id())))
}

/// 將完整暫存檔原子移入目標；失敗時清掉本函數建立的暫存檔並回報兩層錯誤。
fn rename_or_cleanup(temporary: &Path, target: &Path) -> Result<()> {
    if let Err(rename_error) = fs::rename(temporary, target) {
        if let Err(cleanup_error) = fs::remove_file(temporary) {
            bail!("收據 rename 失敗：{rename_error}；暫存清理也失敗：{cleanup_error}");
        }
        return Err(rename_error).with_context(|| format!("無法完成收據：{}", target.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use singularity_world::world_state::{AdvanceStep, Fact, advance_sequence};

    /// 建立最小有效 Manifest，只供收據包裝測試讀取來源身分。
    fn manifest() -> Result<Manifest> {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/data/world/white_tower_peninsula_v1.json"
        )))
        .context("測試 manifest 解析失敗")
    }

    /// 建立沒有內容政策的最小多日收據，供檔案行為測試使用。
    fn run_receipt() -> Result<WorldRunReceipt> {
        let facts = vec![Fact {
            id: "fact-1".to_string(),
            subject_ref: "subject-1".to_string(),
            key: "state".to_string(),
            value: "before".to_string(),
        }];
        let transitions = vec![singularity_world::world_state::Transition {
            id: "transition-1".to_string(),
            order: 1,
            actor_refs: Vec::new(),
            object_refs: Vec::new(),
            region_refs: Vec::new(),
            preconditions: vec![singularity_world::world_state::Condition {
                fact_ref: "fact-1".to_string(),
                equals: "before".to_string(),
            }],
            effects: vec![singularity_world::world_state::Effect {
                fact_ref: "fact-1".to_string(),
                set: "after".to_string(),
            }],
            consequence: "環境自行改變".to_string(),
        }];
        advance_sequence(
            "test-world",
            "D0",
            &facts,
            &[AdvanceStep {
                target_day_id: "D1".to_string(),
                transitions,
            }],
        )
    }

    /// 建立通過的接續驗證摘要，供收據包裝測試使用。
    fn resume_evidence() -> ResumeEvidence {
        ResumeEvidence {
            checkpoint_day_id: "D0".to_string(),
            target_day_id: "D1".to_string(),
            resumed_steps: 1,
            final_facts_match: true,
            suffix_receipts_match: true,
        }
    }

    /// 驗證首次建立、同內容重跑與不同內容拒絕三條收據路徑。
    #[test]
    fn writes_new_accepts_identical_and_rejects_overwrite() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("receipt.json");
        let manifest = manifest()?;
        let receipt = run_receipt()?;
        assert_eq!(
            write_verification_receipt(&path, &manifest, &receipt, &resume_evidence())?,
            ReceiptWriteStatus::Created
        );
        assert_eq!(
            write_verification_receipt(&path, &manifest, &receipt, &resume_evidence())?,
            ReceiptWriteStatus::Unchanged
        );
        fs::write(&path, b"different\n")?;
        assert!(
            write_verification_receipt(&path, &manifest, &receipt, &resume_evidence()).is_err()
        );
        Ok(())
    }

    /// 驗證證據輸出在寫入前就拒絕 data/runtime 路徑。
    #[test]
    fn rejects_runtime_directory() -> Result<()> {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/runtime/forbidden-receipt.json");
        let error =
            write_verification_receipt(&path, &manifest()?, &run_receipt()?, &resume_evidence())
                .expect_err("runtime 路徑必須被拒絕");
        assert!(error.to_string().contains("不得寫入 data/runtime"));
        assert!(!path.exists());
        Ok(())
    }
}
