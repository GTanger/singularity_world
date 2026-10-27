//! Azgaar 固定來源、Node runtime 與短命 Vite 伺服器管理。

use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};

const AZGAAR_REPOSITORY: &str = "https://github.com/Azgaar/Fantasy-Map-Generator.git";
const AZGAAR_COMMIT: &str = "73e43d2b44b9b85b42c5394a0476d90bb22d688e";
const NODE_FALLBACK: &str =
    "/home/tanger/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node";

/// 建立帶 pid 的受管暫存路徑，避免污染專案根目錄。
pub(super) fn staging_dir() -> Result<PathBuf> {
    let root = env::var_os("TANGER_TMP_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/tanger/.local/share/tanger/tmp"));
    ensure!(root.is_absolute(), "TANGER_TMP_ROOT 必須是絕對路徑");
    Ok(root.join(format!(
        "white-tower-azgaar-generate-{}",
        std::process::id()
    )))
}

/// 採用指定 checkout，或在本次暫存取得固定 upstream commit。
pub(super) fn prepare_azgaar(staging: &Path) -> Result<PathBuf> {
    if let Some(root) = env::var_os("AZGAAR_ROOT").map(PathBuf::from) {
        verify_commit(&root)?;
        return Ok(root);
    }
    let root = staging.join("azgaar-fmg");
    run_checked(
        Command::new("git")
            .args([
                "clone",
                "--filter=blob:none",
                "--no-checkout",
                AZGAAR_REPOSITORY,
            ])
            .arg(&root),
        "clone Azgaar",
    )?;
    run_checked(
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["checkout", AZGAAR_COMMIT]),
        "checkout Azgaar 固定 commit",
    )?;
    verify_commit(&root)?;
    Ok(root)
}

/// 驗證外部資料來源沒有漂移到其他版本。
fn verify_commit(root: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()?;
    ensure!(output.status.success(), "無法讀取 Azgaar commit");
    let commit = String::from_utf8(output.stdout)?.trim().to_string();
    ensure!(commit == AZGAAR_COMMIT, "Azgaar commit 不符：{commit}");
    Ok(())
}

/// 選擇 Node 24 以上；可顯式覆蓋，也能採用本機 Codex bundled runtime。
pub(super) fn select_node() -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = env::var_os("WHITE_TOWER_NODE_BIN") {
        candidates.push(PathBuf::from(path));
    }
    candidates.push(PathBuf::from("node"));
    candidates.push(PathBuf::from(NODE_FALLBACK));
    for candidate in candidates {
        if node_major(&candidate).is_some_and(|major| major >= 24) {
            return Ok(candidate);
        }
    }
    bail!("需要 Node.js 24 以上；可用 WHITE_TOWER_NODE_BIN 指定")
}

/// 讀取 Node major version；不可執行的候選直接略過。
fn node_major(node: &Path) -> Option<u32> {
    let output = Command::new(node).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .trim_start_matches('v')
        .split('.')
        .next()?
        .parse()
        .ok()
}

/// 使用同一 Node PATH 安裝 upstream 鎖定相依並執行原生 build。
pub(super) fn install_and_build(root: &Path, node: &Path) -> Result<()> {
    run_checked(
        npm_command(node).current_dir(root).args([
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
        ]),
        "npm ci",
    )?;
    run_checked(
        npm_command(node).current_dir(root).args(["run", "build"]),
        "Azgaar build",
    )
}

/// 建立會優先找到指定 Node 的 npm 命令。
fn npm_command(node: &Path) -> Command {
    let mut command = Command::new("npm");
    if let Some(parent) = node.parent().filter(|_| node.is_absolute()) {
        let mut paths = vec![parent.to_path_buf()];
        paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
        command.env(
            "PATH",
            env::join_paths(paths).unwrap_or_else(|_| OsString::from(parent)),
        );
    }
    command
}

/// 取得目前可用的本機 TCP port；釋放後立刻交給 Vite。
pub(super) fn free_port() -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

/// 啟動 Azgaar Vite 作者伺服器，輸出只留在本次暫存 log。
pub(super) fn start_server(root: &Path, node: &Path, port: u16, staging: &Path) -> Result<Child> {
    let log = File::create(staging.join("vite.log"))?;
    let error_log = log.try_clone()?;
    npm_command(node)
        .current_dir(root)
        .args(["run", "dev", "--", "--host", "127.0.0.1", "--port"])
        .arg(port.to_string())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(error_log))
        .spawn()
        .context("無法啟動 Azgaar Vite")
}

/// 等待作者伺服器可連線；提早退出會立即回報而非假裝完成。
pub(super) fn wait_for_server(server: &mut Child, port: u16) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        if let Some(status) = server.try_wait()? {
            bail!("Azgaar Vite 提早結束：{status}");
        }
        thread::sleep(Duration::from_millis(250));
    }
    bail!("等待 Azgaar Vite 超時")
}

/// 執行外部命令並保留原始輸出；非零狀態立即中止流程。
pub(super) fn run_checked(command: &mut Command, label: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("無法執行 {label}"))?;
    ensure!(status.success(), "{label} 失敗：{status}");
    Ok(())
}
