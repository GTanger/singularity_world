//! 白塔東岸體驗原型的確定性驗證收據。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;

use super::{ExperienceFixture, ProofBoundary, SourceWorld, ValidationOutput};

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
    source_fixture: SourceFixture<'a>,
    source_world: &'a SourceWorld,
    hypothesis: &'a str,
    falsifiers: &'a [String],
    proof_boundary: &'a ProofBoundary,
    verdict: &'a str,
    checks: &'a super::ProofChecks,
    entry: &'a super::TranscriptPhase,
    variants: &'a [super::VariantEvidence],
    branch_differences: &'a [super::BranchDifference],
}

#[derive(Serialize)]
struct SourceFixture<'a> {
    id: &'a str,
    name: &'a str,
    status: &'a str,
    stage: &'a str,
    canon: bool,
}

/// 將通過的體驗證據序列化；只建立新檔或接受逐位元相同的既有檔。
pub(super) fn write_verification_receipt(
    path: &Path,
    fixture: &ExperienceFixture,
    output: &ValidationOutput,
) -> Result<ReceiptWriteStatus> {
    ensure_evidence_path(path)?;
    let evidence = EvidenceReceipt {
        schema_version: 1,
        artifact_role: "verification_evidence",
        runtime_authority: false,
        source_fixture: SourceFixture {
            id: &fixture.id,
            name: &fixture.name,
            status: &fixture.status,
            stage: &fixture.stage,
            canon: fixture.canon,
        },
        source_world: &fixture.source_world,
        hypothesis: &fixture.hypothesis,
        falsifiers: &fixture.falsifiers,
        proof_boundary: &fixture.proof_boundary,
        verdict: output.verdict,
        checks: &output.checks,
        entry: &output.entry,
        variants: &output.variants,
        branch_differences: &output.branch_differences,
    };
    let mut bytes = serde_json::to_vec_pretty(&evidence).context("體驗收據序列化失敗")?;
    bytes.push(b'\n');
    write_new_or_identical(path, &bytes)
}

/// 證據不得寫入 runtime；其父目錄必須先由專案明確建立。
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
        "體驗驗證收據不得寫入 data/runtime"
    );
    Ok(())
}

fn write_new_or_identical(path: &Path, bytes: &[u8]) -> Result<ReceiptWriteStatus> {
    if path.exists() {
        let existing =
            fs::read(path).with_context(|| format!("無法讀取既有收據：{}", path.display()))?;
        ensure!(
            existing == bytes,
            "既有體驗收據內容不同，拒絕覆寫：{}",
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
    file.write_all(bytes).context("寫入暫存體驗收據失敗")?;
    file.sync_all().context("同步暫存體驗收據失敗")?;
    drop(file);
    rename_or_cleanup(&temporary, path)?;
    Ok(ReceiptWriteStatus::Created)
}

fn temporary_path(path: &Path) -> Result<PathBuf> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .context("收據檔名必須是有效 UTF-8")?;
    Ok(path.with_file_name(format!(".{name}.{}.tmp", std::process::id())))
}

fn rename_or_cleanup(temporary: &Path, target: &Path) -> Result<()> {
    if let Err(rename_error) = fs::rename(temporary, target) {
        if let Err(cleanup_error) = fs::remove_file(temporary) {
            bail!("體驗收據 rename 失敗：{rename_error}；暫存清理失敗：{cleanup_error}");
        }
        return Err(rename_error)
            .with_context(|| format!("無法完成體驗收據：{}", target.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load_json, validation::validate_experience};

    fn evidence() -> Result<(ExperienceFixture, ValidationOutput)> {
        let fixture: ExperienceFixture = load_json(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/world/white_tower_east_coast_experience_v1.json"),
            "測試 fixture",
        )?;
        let source = load_json(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/world/white_tower_peninsula_v1.json"),
            "測試來源",
        )?;
        let output = validate_experience(&fixture, &source)?;
        Ok((fixture, output))
    }

    #[test]
    fn creates_accepts_identical_and_rejects_different_receipt() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("receipt.json");
        let (fixture, output) = evidence()?;
        assert_eq!(
            write_verification_receipt(&path, &fixture, &output)?,
            ReceiptWriteStatus::Created
        );
        assert_eq!(
            write_verification_receipt(&path, &fixture, &output)?,
            ReceiptWriteStatus::Unchanged
        );
        fs::write(&path, b"different\n")?;
        assert!(write_verification_receipt(&path, &fixture, &output).is_err());
        Ok(())
    }

    #[test]
    fn rejects_runtime_directory() -> Result<()> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/runtime/forbidden-east-coast-experience-receipt.json");
        let (fixture, output) = evidence()?;
        let error = write_verification_receipt(&path, &fixture, &output)
            .expect_err("runtime 路徑必須被拒絕");
        assert!(error.to_string().contains("不得寫入 data/runtime"));
        assert!(!path.exists());
        Ok(())
    }
}
