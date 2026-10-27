//! 白塔東岸體驗檢查器的命令列參數。

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};

const DEFAULT_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/world/white_tower_east_coast_experience_v1.json"
);
const DEFAULT_SOURCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/world/white_tower_peninsula_v1.json"
);

pub(super) struct CliArgs {
    pub(super) fixture_path: PathBuf,
    pub(super) source_path: PathBuf,
    pub(super) receipt_path: Option<PathBuf>,
}

/// 接受一個可選 fixture、`--source` 與 `--receipt`；拒絕未知或重複參數。
pub(super) fn parse_args() -> Result<CliArgs> {
    let mut input = env::args_os().skip(1);
    let mut fixture_path = None;
    let mut source_path = None;
    let mut receipt_path = None;
    while let Some(arg) = input.next() {
        if arg == "--source" {
            ensure!(source_path.is_none(), "--source 不得重複");
            source_path = Some(PathBuf::from(input.next().context("--source 後缺少路徑")?));
            continue;
        }
        if arg == "--receipt" {
            ensure!(receipt_path.is_none(), "--receipt 不得重複");
            receipt_path = Some(PathBuf::from(input.next().context("--receipt 後缺少路徑")?));
            continue;
        }
        ensure!(
            !arg.to_string_lossy().starts_with('-'),
            "未知旗標：{}",
            arg.to_string_lossy()
        );
        ensure!(fixture_path.is_none(), "只能指定一個體驗 fixture");
        fixture_path = Some(PathBuf::from(arg));
    }
    Ok(CliArgs {
        fixture_path: fixture_path.unwrap_or_else(|| PathBuf::from(DEFAULT_FIXTURE)),
        source_path: source_path.unwrap_or_else(|| PathBuf::from(DEFAULT_SOURCE)),
        receipt_path,
    })
}
