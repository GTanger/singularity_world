#![forbid(unsafe_code)]
//! 一次完成 Azgaar 固定版本取得、白塔半島生成、原生匯出、驗證、安裝與暫存清理。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use anyhow::{Context, Result};

#[path = "white_tower_map_generate/runtime.rs"]
mod runtime;

use runtime::{
    free_port, install_and_build, prepare_azgaar, run_checked, select_node, staging_dir,
    start_server, wait_for_server,
};

struct Cleanup {
    staging: PathBuf,
    server: Option<Child>,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        // 無論中途哪一步失敗，都停止作者伺服器並清除本次受管暫存。
        if let Some(server) = self.server.as_mut() {
            let _ = server.kill();
            let _ = server.wait();
        }
        let _ = fs::remove_dir_all(&self.staging);
    }
}

fn main() -> Result<()> {
    // 生成只改兩個正式輸出；Azgaar checkout、node_modules、log 與 staging 全部短命。
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let staging = staging_dir()?;
    fs::create_dir_all(&staging)?;
    let mut cleanup = Cleanup {
        staging: staging.clone(),
        server: None,
    };
    let azgaar = prepare_azgaar(&staging)?;
    let node = select_node()?;
    install_and_build(&azgaar, &node)?;
    let port = free_port()?;
    cleanup.server = Some(start_server(&azgaar, &node, port, &staging)?);
    wait_for_server(cleanup.server.as_mut().expect("server"), port)?;
    generate_to_staging(&project, &azgaar, &node, port, &staging)?;
    stop_server(&mut cleanup.server)?;
    verify_staging(&project, &staging)?;
    install_outputs(&project, &staging)?;
    if let Err(error) = verify_installed(&project) {
        rollback_installed(&project, &staging);
        return Err(error);
    }
    println!(
        "白塔半島完成：Azgaar 原生完整資料已保留，既有輸出經局部約束調整、收據驗證並安裝；本次暫存與伺服器已排入自動清理。"
    );
    Ok(())
}

/// 呼叫現有橋接器，將完整原生包與物理橋接 JSON 寫入 staging。
fn generate_to_staging(
    project: &Path,
    azgaar: &Path,
    node: &Path,
    port: u16,
    staging: &Path,
) -> Result<()> {
    let package = staging.join("white_tower_azgaar_v1");
    fs::create_dir_all(&package)?;
    run_checked(
        Command::new(node)
            .current_dir(project)
            .arg(project.join("scripts/azgaar/white-tower-bridge-export.mjs"))
            .env("AZGAAR_ROOT", azgaar)
            .env(
                "AZGAAR_ORIGIN",
                format!("http://127.0.0.1:{port}/Fantasy-Map-Generator/"),
            )
            .env(
                "WHITE_TOWER_OUTPUT",
                staging.join("white_tower_physical_geography_v1.json"),
            )
            .env("WHITE_TOWER_PACKAGE_DIR", package),
        "白塔半島 Azgaar 匯出",
    )
}

/// 主動停止 Vite；Drop 仍是失敗路徑的第二道保險。
fn stop_server(server: &mut Option<Child>) -> Result<()> {
    if let Some(mut child) = server.take() {
        child.kill().context("停止 Azgaar Vite 失敗")?;
        child.wait()?;
    }
    Ok(())
}

/// 在覆蓋正式資料前驗證 staging 的所有收據與資料層。
fn verify_staging(project: &Path, staging: &Path) -> Result<()> {
    run_checked(
        Command::new("cargo")
            .current_dir(project)
            .args(["run", "--quiet", "--bin", "white-tower-package-check", "--"])
            .arg(staging.join("white_tower_azgaar_v1"))
            .arg(staging.join("white_tower_physical_geography_v1.json")),
        "staging 資料包驗證",
    )
}

/// 以可回復的 rename 安裝兩個輸出；任一步失敗都還原舊版。
fn install_outputs(project: &Path, staging: &Path) -> Result<()> {
    let world = project.join("data/world");
    let final_package = world.join("white_tower_azgaar_v1");
    let final_geography = world.join("white_tower_physical_geography_v1.json");
    let old_package = staging.join("previous-package");
    let old_geography = staging.join("previous-geography.json");
    if final_package.exists() {
        fs::rename(&final_package, &old_package)?;
    }
    if final_geography.exists() {
        fs::rename(&final_geography, &old_geography)?;
    }
    if let Err(error) = move_new_outputs(staging, &final_package, &final_geography) {
        restore_outputs(
            &final_package,
            &final_geography,
            &old_package,
            &old_geography,
        );
        return Err(error);
    }
    Ok(())
}

/// 搬入已驗證的新資料包與橋接檔。
fn move_new_outputs(staging: &Path, package: &Path, geography: &Path) -> Result<()> {
    fs::rename(staging.join("white_tower_azgaar_v1"), package)?;
    fs::rename(
        staging.join("white_tower_physical_geography_v1.json"),
        geography,
    )?;
    Ok(())
}

/// 安裝失敗時清除半套新檔並搬回原輸出。
fn restore_outputs(package: &Path, geography: &Path, old_package: &Path, old_geography: &Path) {
    let _ = fs::remove_dir_all(package);
    let _ = fs::remove_file(geography);
    if old_package.exists() {
        let _ = fs::rename(old_package, package);
    }
    if old_geography.exists() {
        let _ = fs::rename(old_geography, geography);
    }
}

/// 正式驗證失敗時仍以 staging 內的備份回復，不留下半套安裝。
fn rollback_installed(project: &Path, staging: &Path) {
    let world = project.join("data/world");
    restore_outputs(
        &world.join("white_tower_azgaar_v1"),
        &world.join("white_tower_physical_geography_v1.json"),
        &staging.join("previous-package"),
        &staging.join("previous-geography.json"),
    );
}

/// 正式安裝後重新編譯內嵌 JSON 並執行嚴格地理驗證。
fn verify_installed(project: &Path) -> Result<()> {
    run_checked(
        Command::new("cargo").current_dir(project).args([
            "run",
            "--quiet",
            "--bin",
            "white-tower-geography-check",
        ]),
        "正式白塔半島物理地理驗證",
    )
}
