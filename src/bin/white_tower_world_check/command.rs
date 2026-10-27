//! 白塔提案檢查器的命令列參數解析。

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};

const DEFAULT_MANIFEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/world/white_tower_peninsula_v1.json"
);

pub(super) struct CliArgs {
    pub(super) manifest_path: PathBuf,
    pub(super) receipt_path: Option<PathBuf>,
}

/// 解析一個可選 manifest 與一個可選 --receipt 路徑，拒絕未知旗標及多餘位置參數。
pub(super) fn parse_args() -> Result<CliArgs> {
    let mut input = env::args_os().skip(1);
    let mut manifest_path = None;
    let mut receipt_path = None;
    while let Some(arg) = input.next() {
        if arg == "--receipt" {
            ensure!(receipt_path.is_none(), "--receipt 不得重複");
            let value = input.next().context("--receipt 後缺少輸出路徑")?;
            receipt_path = Some(PathBuf::from(value));
            continue;
        }
        ensure!(
            !arg.to_string_lossy().starts_with('-'),
            "未知旗標：{}",
            arg.to_string_lossy()
        );
        ensure!(manifest_path.is_none(), "只能指定一個 manifest.json");
        manifest_path = Some(PathBuf::from(arg));
    }
    Ok(CliArgs {
        manifest_path: manifest_path.unwrap_or_else(|| PathBuf::from(DEFAULT_MANIFEST)),
        receipt_path,
    })
}
