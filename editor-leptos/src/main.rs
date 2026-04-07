use leptos::prelude::*;
mod api;
mod components;
mod types;

use crate::types::{HexCell, HexCoord, ToolMode, Terrain};

use components::toolbar::Toolbar;
use components::hex_grid::HexGrid;
use components::cell_editor::CellEditor;

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let (editor_open, set_editor_open) = signal(false);
    let (mobile_menu_open, set_mobile_menu_open) = signal(false);
    let (status, set_status) = signal("就緒".to_string());
    let (world_seed_str, set_world_seed_str) = signal("0".to_string());
    let (cells, set_cells) = signal::<Vec<HexCell>>(Vec::new());
    let (selected, set_selected) = signal::<Option<HexCoord>>(None);
    let (selected_many, set_selected_many) = signal::<Vec<HexCoord>>(Vec::new());
    let (brush_terrain, set_brush_terrain) = signal(Terrain::Plain);
    let (brush_size, set_brush_size) = signal(1u8);
    let (tool_mode, set_tool_mode) = signal(ToolMode::View);

    let cells_signal: Signal<Vec<HexCell>> = cells.into();

    let reload_grid = move || {
        let set_cells = set_cells;
        let set_status = set_status;
        leptos::task::spawn_local(async move {
            match api::load_grid().await {
                Ok(c) => set_cells.set(c.cells),
                Err(e) => set_status.set(format!("載入失敗：{e}")),
            }
        });
    };

    Effect::new(move || {
        reload_grid();
    });

    let on_reload = move |_| reload_grid();

    let on_apply_world_seed = move |_| {
        let seed = world_seed_str.get_untracked();
        let set_status = set_status;
        let reload_grid = reload_grid;
        leptos::task::spawn_local(async move {
            set_status.set("正在切換種子...".into());
            let seed_val = seed.parse::<u64>().unwrap_or(0);
            match api::put_world_seed(seed_val).await {
                Ok(_) => {
                    set_status.set(format!("種子已更新：{seed_val}（請重刷或重載）"));
                    reload_grid();
                }
                Err(e) => set_status.set(format!("生成失敗：{e}")),
            }
        });
    };

    let on_clear_selection = move |_| {
        set_selected.set(None);
        set_selected_many.set(Vec::new());
    };

    let on_paint = {
        let set_status = set_status;
        let reload_grid = reload_grid;
        move |coord: HexCoord| {
            let terrain = brush_terrain.get_untracked();
            leptos::task::spawn_local(async move {
                let req = api::CellPutReq {
                    q: coord.q,
                    r: coord.r,
                    terrain,
                    name: String::new(),
                    zone: String::new(),
                    tags: Vec::new(),
                    description: String::new(),
                };
                if let Err(e) = api::put_cells(&[req]).await {
                    set_status.set(format!("繪製失敗：{e}"));
                } else {
                    reload_grid();
                }
            });
        }
    };

    let on_erase = {
        let set_status = set_status;
        let reload_grid = reload_grid;
        move |coord: HexCoord| {
            leptos::task::spawn_local(async move {
                if let Err(e) = api::delete_cell(coord.q, coord.r).await {
                    set_status.set(format!("刪除失敗：{e}"));
                } else {
                    reload_grid();
                }
            });
        }
    };

    let on_select_toggle = move |coord: HexCoord| {
        set_selected_many.update(|v| {
            if let Some(pos) = v.iter().position(|c| *c == coord) {
                v.remove(pos);
            } else {
                v.push(coord);
            }
        });
        set_selected.set(Some(coord));
    };

    let on_select_add = move |coord: HexCoord| {
        set_selected_many.update(|v| {
            if !v.contains(&coord) {
                v.push(coord);
            }
        });
        set_selected.set(Some(coord));
    };




    let on_move_selection = move |(_from, _to): (HexCoord, HexCoord)| {
        set_status.set("移動功能目前已棄用".into());
    };

    let cell_count = Signal::derive(move || cells.get().len());

    let on_cell_saved = move |_| reload_grid();
    let on_cell_deleted = move |_| {
        set_selected.set(None);
        reload_grid();
    };

    view! {
        <div class="app-container">
            <Toolbar
                on_reload=on_reload
                on_clear_selection=on_clear_selection
                on_apply_world_seed=on_apply_world_seed
                set_status=set_status
                world_seed_str=world_seed_str
                set_world_seed_str=set_world_seed_str
                selected=selected
                cell_count=cell_count
                brush_terrain=brush_terrain
                set_brush_terrain=set_brush_terrain
                brush_size=brush_size
                set_brush_size=set_brush_size
                tool_mode=tool_mode
                set_tool_mode=set_tool_mode
                editor_open=editor_open
                set_editor_open=set_editor_open
                mobile_menu_open=mobile_menu_open
                set_mobile_menu_open=set_mobile_menu_open
            />
            <div class="main-layout">
                <HexGrid
                    cells=cells_signal
                    selected=selected
                    selected_many=selected_many
                    set_selected=set_selected
                    brush_size=brush_size
                    tool_mode=tool_mode
                    on_paint=on_paint
                    on_erase=on_erase
                    on_select_toggle=on_select_toggle
                    on_select_add=on_select_add
                    on_move_selection=on_move_selection
                />
            </div>
            <Show when=move || editor_open.get()>
                <CellEditor
                    cells=cells_signal
                    selected=selected
                    on_saved=on_cell_saved
                    on_deleted=on_cell_deleted
                />
            </Show>
        </div>
        <div class="status-bar">
            {move || status.get()}
            {move || {
                selected.get().map(|c| format!(" | 選取：({},{})", c.q, c.r)).unwrap_or_default()
            }}
            {move || {
                let n = selected_many.get().len();
                if n > 1 { format!(" | 多選：{} 格", n) } else { String::new() }
            }}
        </div>
    }
}
