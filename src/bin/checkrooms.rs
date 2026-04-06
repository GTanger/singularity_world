#![forbid(unsafe_code)]
// checkrooms：房間 JSON 契約檢查 CLI（`src/bin/checkrooms.rs`，邏輯見 `src/roomcheck.rs`）。
// 用法：checkrooms [-sockets Move,Look] [-brackets] [-strict] [路徑…]

use std::env;
use std::process;

use singularity_world::roomcheck;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut sockets_str = "Move,Look".to_string();
    let mut brackets = false;
    let mut strict = false;
    let mut roots = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-sockets" | "--sockets" => {
                i += 1;
                if i < args.len() {
                    sockets_str = args[i].clone();
                }
            }
            "-brackets" | "--brackets" => brackets = true,
            "-strict" | "--strict" => strict = true,
            "-h" | "--help" => {
                eprintln!("用法：checkrooms [-sockets Move,Look] [-brackets] [-strict] [路徑…]");
                eprintln!("  -sockets  逗號分隔：要檢查的 sockets 名稱（預設 Move,Look）");
                eprintln!("  -brackets 額外檢查 description 內每個〔…〕是否對應某 objects[].name");
                eprintln!("  -strict   與 -brackets 併用：無對應 name 時視為錯誤（預設僅警告）");
                process::exit(0);
            }
            other => roots.push(other.to_string()),
        }
        i += 1;
    }

    let trigger = roomcheck::parse_socket_list(&sockets_str);
    if trigger.is_empty() {
        eprintln!("-sockets 不可為空");
        process::exit(1);
    }

    let opts = roomcheck::Options {
        trigger_sockets: trigger.clone(),
        check_brackets: brackets,
        strict_brackets: strict,
    };

    let wd = env::current_dir().unwrap_or_else(|e| {
        eprintln!("getwd: {e}");
        process::exit(1);
    });

    let result = roomcheck::run(&roots, &wd, &opts);
    if !roomcheck::print_result(&result, &trigger) {
        process::exit(1);
    }
}
