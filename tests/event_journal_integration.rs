//! `event::append` / `last_by_entity` / `events_in_range` 與 store 整合（runtime 用暫存目錄，不汙染 `data/runtime`）。

use singularity_world::event;
use singularity_world::store;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn append_and_query_event_log_isolated_runtime() {
    let root = project_root();
    let rooms = root.join("data/rooms");
    if !rooms.is_dir() {
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    let rt = tmp.path().to_str().unwrap();
    let data = root.join("data").to_str().unwrap().to_string();

    store::init(rooms.to_str().unwrap(), rt, &data).expect("init");

    let eid = format!("event_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros());
    let t = 1_700_000_000_i64;

    event::append(t, &eid, event::types::MOVE, "north").expect("append");
    event::append(t + 1, &eid, event::types::ARRIVED, "room_a").expect("append");

    let last = event::last_by_entity(&eid, event::types::MOVE, t + 2).expect("last");
    assert_eq!(last, "north");

    let rows = event::events_in_range(&eid, t, t + 1).expect("range");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].event_type, event::types::MOVE);
    assert_eq!(rows[1].payload, "room_a");
}
