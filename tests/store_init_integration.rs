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

    // data/rooms/editor 目前 598 個一房一檔；prune 後總房數仍應 ≥ editor 檔數
    const MIN_EDITOR_ROOMS: usize = 598;
    assert!(
        st.rooms.len() >= MIN_EDITOR_ROOMS,
        "房間數應至少 {}（editor 檔案數），實際 {}",
        MIN_EDITOR_ROOMS,
        st.rooms.len()
    );

    // 抽樣：editor 內一間應存在
    assert!(
        st.get_room("city_life_11f_lounge").is_some(),
        "預期載入 editor 房間 city_life_11f_lounge"
    );

    // data_dir 非空時應嘗試載入 entities（檔案存在則有資料）
    if data.join("entities.json").is_file() {
        assert!(
            !st.entities.is_empty(),
            "entities.json 存在時應解析出至少一筆 entity"
        );
    }
}
