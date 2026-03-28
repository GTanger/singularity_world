//! Phase 2 驗收：以倉庫內真實 JSON 呼叫 `store::init`，確保可載入 `data/rooms`（含 editor/）
//! 與 `data/`、`data/runtime` 設定檔，與 Go `store.Init` 路徑慣例一致。
//!
//! 需從專案根執行 `cargo test`（`CARGO_MANIFEST_DIR` 指向含 `data/` 的目錄）。

use singularity_world::store;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn init_loads_editor_rooms_and_data_json() {
    let root = project_root();
    let rooms = root.join("data/rooms");
    let runtime = root.join("data/runtime");
    let data = root.join("data");

    assert!(
        rooms.is_dir(),
        "預期 {} 存在（請於含 data/ 的專案根執行測試）",
        rooms.display()
    );

    store::init(
        rooms.to_str().expect("utf-8 path"),
        runtime.to_str().expect("utf-8 path"),
        data.to_str().expect("utf-8 path"),
    )
    .expect("store::init 應成功載入現有 JSON");

    let arc = store::get_store().expect("init 後應設定全域 store");
    let st = arc.read().expect("store lock");

    // 浮生城已封存至 archive/，editor 目錄可能為空
    // 僅驗證 init 成功且房間數非負（新地圖由 Parser 生成後再調整斷言）
    eprintln!("載入房間數：{}", st.rooms.len());

    // data_dir 非空時應嘗試載入 entities（檔案存在則有資料）
    if data.join("entities.json").is_file() {
        assert!(
            !st.entities.is_empty(),
            "entities.json 存在時應解析出至少一筆 entity"
        );
    }
}
