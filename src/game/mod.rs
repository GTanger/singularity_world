// game 模組 — 遊戲時間、主迴圈 tick、視野與觀測（對齊 Go game/）。

mod chunk_view;
mod gametime;
mod movement_tick;
mod observe;
mod room;
mod tick;
mod view_sim;
mod zone;

pub use chunk_view::{get_chunk_and_entities_in_view, ChunkView};
pub use gametime::game_time_now;
pub use movement_tick::{advance_movement, entity_at, MovedResult};
pub use observe::{collapse, now_unix, observe_room, Observed, Observer};
pub use room::{ensure_entity_in_room, entity_kind_str, get_room_view, move_by_exit, RoomView};
pub use tick::spawn_loop;
pub use view_sim::{in_view_entity_ids, run_view_simulation, simulate_one_tick, Pos};
pub use zone::{chunk_bounds, chunk_index, in_chunk, in_view, CHUNK_SIZE, VIEW_RADIUS};
