use leptos::prelude::*;
use crate::types::{Terrain, ToolMode};

#[component]
pub fn Toolbar(
    #[prop(into)] on_reload: Callback<()>,
    #[prop(into)] on_save: Callback<()>,
    #[prop(into)] on_clear_selection: Callback<()>,
    cell_count: Signal<usize>,
    brush_terrain: ReadSignal<Terrain>,
    set_brush_terrain: WriteSignal<Terrain>,
    brush_size: ReadSignal<u8>,
    set_brush_size: WriteSignal<u8>,
    tool_mode: ReadSignal<ToolMode>,
    set_tool_mode: WriteSignal<ToolMode>,
    editor_open: ReadSignal<bool>,
    set_editor_open: WriteSignal<bool>,
) -> impl IntoView {
    let (busy, set_busy) = signal(false);

    let handle_reload = move |_| {
        set_busy.set(true);
        let on_reload = on_reload.clone();
        leptos::task::spawn_local(async move {
            match crate::api::reload_grid().await {
                Ok(n) => {
                    web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.set_title(&format!("六角格編輯器（{n} 格）")).into());
                    on_reload.run(());
                }
                Err(e) => {
                    web_sys::console::log_1(&format!("reload 失敗：{e}").into());
                }
            }
            set_busy.set(false);
        });
    };

    let handle_save = move |_| {
        set_busy.set(true);
        let on_save = on_save.clone();
        leptos::task::spawn_local(async move {
            match crate::api::save_grid().await {
                Ok(_n) => {
                    on_save.run(());
                }
                Err(e) => {
                    web_sys::console::log_1(&format!("save 失敗：{e}").into());
                }
            }
            set_busy.set(false);
        });
    };

    let handle_clear_selection = move |_| {
        on_clear_selection.run(());
    };

    let handle_toggle_editor = move |_| {
        set_editor_open.update(|v| *v = !*v);
    };

    view! {
        <div class="toolbar">
            <span class="toolbar-title">"六角格編輯器"</span>
            <span class="toolbar-count">{move || format!("{} 格", cell_count.get())}</span>
            <span class="toolbar-count">"模式"</span>
            <select
                class="toolbar-select"
                on:change=move |ev| {
                    let val = event_target_value(&ev);
                    let mode = match val.as_str() {
                        "erase" => ToolMode::Erase,
                        "select" => ToolMode::Select,
                        "move" => ToolMode::Move,
                        _ => ToolMode::Paint,
                    };
                    set_tool_mode.set(mode);
                }
                prop:value=move || tool_mode.get().as_str().to_string()
            >
                <option value="paint">"繪製"</option>
                <option value="erase">"刪除"</option>
                <option value="select">"選取"</option>
                <option value="move">"移動"</option>
            </select>
            <span class="toolbar-count">"筆刷"</span>
            <select
                class="toolbar-select"
                on:change=move |ev| {
                    let val = event_target_value(&ev);
                    for t in Terrain::ALL {
                        if t.serde_name() == val {
                            set_brush_terrain.set(*t);
                            break;
                        }
                    }
                }
                prop:value=move || brush_terrain.get().serde_name().to_string()
                disabled=move || tool_mode.get() != ToolMode::Paint
            >
                {move || Terrain::ALL.iter().map(|t| {
                    let t = *t;
                    let name = t.serde_name().to_string();
                    let label = t.label().to_string();
                    view! { <option value={name}>{label}</option> }
                }).collect::<Vec<_>>()}
            </select>
            <span class="toolbar-count">{move || format!("筆刷 {}", brush_size.get())}</span>
            <input
                class="toolbar-range"
                type="range"
                min="1"
                max="5"
                step="1"
                prop:value=move || brush_size.get().to_string()
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<u8>() {
                        set_brush_size.set(v.clamp(1, 5));
                    }
                }
            />
            <div class="toolbar-spacer" />
            <button class="toolbar-btn" on:click=handle_toggle_editor>
                {move || if editor_open.get() { "🧩 收合側欄" } else { "🧩 展開側欄" }}
            </button>
            <button class="toolbar-btn" on:click=handle_clear_selection>"🧹 清空選取"</button>
            <button class="toolbar-btn" on:click=handle_save disabled=move || busy.get()>"💾 儲存"</button>
            <button class="toolbar-btn" on:click=handle_reload disabled=move || busy.get()>"🔄 重新載入"</button>
        </div>
    }
}
