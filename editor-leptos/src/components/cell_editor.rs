use leptos::prelude::*;
use crate::types::{HexCell, HexCoord, Terrain};
use crate::api::{self, CellPutReq};

#[component]
pub fn CellEditor(
    cells: Signal<Vec<HexCell>>,
    selected: ReadSignal<Option<HexCoord>>,
    #[prop(into)] on_saved: Callback<()>,
    #[prop(into)] on_deleted: Callback<HexCoord>,
) -> impl IntoView {
    let (saving, set_saving) = signal(false);
    let (message, set_message) = signal(String::new());

    let selected_cell = Memo::new(move |_| {
        let sel = selected.get()?;
        cells.get().into_iter().find(|c| c.coord == sel)
    });

    let (edit_name, set_edit_name) = signal(String::new());
    let (edit_zone, set_edit_zone) = signal(String::new());
    let (edit_terrain, set_edit_terrain) = signal(Terrain::Grassland);
    let (edit_tags, set_edit_tags) = signal(String::new());
    let (edit_desc, set_edit_desc) = signal(String::new());

    Effect::new(move || {
        if let Some(cell) = selected_cell.get() {
            set_edit_name.set(cell.name.clone());
            set_edit_zone.set(cell.zone.clone());
            set_edit_terrain.set(cell.terrain);
            set_edit_tags.set(cell.tags.join(", "));
            set_edit_desc.set(cell.description.clone());
        }
    });

    let on_save = move |_| {
        let Some(coord) = selected.get() else { return };
        let name = edit_name.get();
        let zone = edit_zone.get();
        let terrain = edit_terrain.get();
        let tags_str = edit_tags.get();
        let desc = edit_desc.get();

        set_saving.set(true);
        set_message.set(String::new());

        let tags: Vec<String> = tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let on_saved = on_saved;
        leptos::task::spawn_local(async move {
            let req = CellPutReq {
                q: coord.q,
                r: coord.r,
                terrain,
                name,
                zone,
                tags,
                description: desc,
            };
            match api::put_cell(&req).await {
                Ok(()) => {
                    set_message.set("已儲存".into());
                    on_saved.run(());
                }
                Err(e) => set_message.set(format!("儲存失敗：{e}")),
            }
            set_saving.set(false);
        });
    };

    let on_delete = move |_| {
        let Some(coord) = selected.get() else { return };
        set_saving.set(true);
        let on_deleted = on_deleted;
        leptos::task::spawn_local(async move {
            match api::delete_cell(coord.q, coord.r).await {
                Ok(()) => {
                    set_message.set("已刪除".into());
                    on_deleted.run(coord);
                }
                Err(e) => set_message.set(format!("刪除失敗：{e}")),
            }
            set_saving.set(false);
        });
    };

    view! {
        <div class="cell-editor">
            <Show when=move || selected.get().is_some() fallback=|| view! {
                <div class="editor-empty">
                    <p>"點擊格子進行編輯"</p>
                </div>
            }>
                <div class="editor-form">
                    <h3>{move || {
                        let sel = selected.get().unwrap_or(HexCoord { q: 0, r: 0 });
                        format!("編輯 ({}, {})", sel.q, sel.r)
                    }}</h3>

                    <label>"名稱"</label>
                    <input
                        type="text"
                        prop:value=move || edit_name.get()
                        on:input=move |ev| set_edit_name.set(event_target_value(&ev))
                    />

                    <label>"地形"</label>
                    <select
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            for t in Terrain::ALL {
                                if t.serde_name() == val {
                                    set_edit_terrain.set(*t);
                                    break;
                                }
                            }
                        }
                        prop:value=move || edit_terrain.get().serde_name().to_string()
                    >
                        {move || Terrain::ALL.iter().map(|t| {
                            let t = *t;
                            let name = t.serde_name().to_string();
                            let label = t.label().to_string();
                            view! {
                                <option value={name} selected=move || edit_terrain.get() == t>{label}</option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>

                    <label>"區域"</label>
                    <input
                        type="text"
                        prop:value=move || edit_zone.get()
                        on:input=move |ev| set_edit_zone.set(event_target_value(&ev))
                    />

                    <label>"標籤（逗號分隔）"</label>
                    <input
                        type="text"
                        prop:value=move || edit_tags.get()
                        on:input=move |ev| set_edit_tags.set(event_target_value(&ev))
                    />

                    <label>"描述"</label>
                    <textarea
                        rows="4"
                        prop:value=move || edit_desc.get()
                        on:input=move |ev| set_edit_desc.set(event_target_value(&ev))
                    />

                    <div class="editor-actions">
                        <button
                            class="btn-save"
                            on:click=on_save
                            disabled=move || saving.get()
                        >
                            {move || if saving.get() { "儲存中⋯" } else { "儲存" }}
                        </button>
                        <button
                            class="btn-delete"
                            on:click=on_delete
                            disabled=move || saving.get()
                        >
                            "刪除"
                        </button>
                    </div>

                    <Show when=move || !message.get().is_empty()>
                        <p class="editor-message">{move || message.get()}</p>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
