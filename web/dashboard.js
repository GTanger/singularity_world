const state = {
    rooms: [],
    selectedId: '',
    selectedRoom: null,
    mgKey: localStorage.getItem('singularity_mg_key') || '',
};

const ui = {
    navItems: document.querySelectorAll('.nav-item'),
    sections: document.querySelectorAll('.page-section'),
    pageTitle: document.getElementById('page-title'),
    mgKeyInput: document.getElementById('mg-key-input'),
    statusBadge: document.getElementById('status-badge'),
    roomsContainer: document.getElementById('rooms-container'),
    roomSearch: document.getElementById('room-search'),
    toastContainer: document.getElementById('toast-container'),
    
    // Editor Fields
    fId: document.getElementById('f-id'),
    fName: document.getElementById('f-name'),
    fZone: document.getElementById('f-zone'),
    fTags: document.getElementById('f-tags'),
    fDesc: document.getElementById('f-desc'),
    fObjects: document.getElementById('f-objects'),
    objectsForm: document.getElementById('objects-form'),
};

// --- Utilities ---
function toast(msg, isError = false) {
    const el = document.createElement('div');
    el.className = `toast ${isError ? 'error' : ''}`;
    el.textContent = msg;
    ui.toastContainer.appendChild(el);
    setTimeout(() => el.remove(), 4000);
}

async function api(path, opt = {}) {
    const url = new URL(path, window.location.origin);
    if (state.mgKey) url.searchParams.set('mg_key', state.mgKey);
    
    const res = await fetch(url, {
        headers: { 'Content-Type': 'application/json' },
        ...opt,
    });
    
    if (res.status === 403) {
        toast('管理金鑰錯誤或未經授權', true);
        throw new Error('Unauthorized');
    }
    
    const txt = await res.text();
    let body = {};
    try { body = txt ? JSON.parse(txt) : {}; } catch(_) {}
    if (!res.ok) throw new Error(body.error || `HTTP ${res.status}`);
    return body;
}

// --- Navigation ---
function switchSection(targetId) {
    ui.sections.forEach(s => s.classList.toggle('active', s.id === targetId));
    ui.navItems.forEach(n => {
        if (n.classList.contains('back-link')) return;
        n.classList.toggle('active', n.dataset.target === targetId);
    });
    
    const navName = Array.from(ui.navItems).find(n => n.dataset.target === targetId)?.querySelector('span')?.textContent;
    ui.pageTitle.textContent = navName || '管理後臺';
    
    if (targetId === 'section-rooms') loadRooms();
}

ui.navItems.forEach(n => {
    n.onclick = () => {
        if (n.classList.contains('back-link')) return;
        switchSection(n.dataset.target);
    };
});

// --- Auth Handling ---
ui.mgKeyInput.value = state.mgKey;
ui.mgKeyInput.onchange = () => {
    state.mgKey = ui.mgKeyInput.value.trim();
    localStorage.setItem('singularity_mg_key', state.mgKey);
    toast('金鑰已儲存');
    loadRooms(); // Reload data with new key
};

// --- Room List ---
async function loadRooms() {
    try {
        ui.statusBadge.textContent = '載入中...';
        ui.statusBadge.className = 'badge';
        const data = await api('/api/rooms');
        state.rooms = data.rooms || [];
        renderRoomsTable();
        ui.statusBadge.textContent = '已連線';
        ui.statusBadge.className = 'badge connected';
    } catch (e) {
        ui.statusBadge.textContent = '連線失敗';
        ui.statusBadge.className = 'badge error';
    }
}

ui.roomSearch.oninput = renderRoomsTable;

function toggleZone(zone) {
    const collapsed = JSON.parse(localStorage.getItem('collapsed_zones') || '{}');
    collapsed[zone] = !collapsed[zone];
    localStorage.setItem('collapsed_zones', JSON.stringify(collapsed));
    renderRoomsTable();
}

function toggleFolder(path) {
    const collapsed = JSON.parse(localStorage.getItem('collapsed_folders') || '{}');
    collapsed[path] = !collapsed[path];
    localStorage.setItem('collapsed_folders', JSON.stringify(collapsed));
    renderRoomsTable();
}

function buildRoomTree(rooms, index = 0) {
    const node = { _rooms: [], _subs: {} };
    const groups = {};
    
    rooms.forEach(r => {
        const parts = r.id.split('_');
        if (index >= parts.length) {
            node._rooms.push(r);
        } else {
            const part = parts[index];
            if (!groups[part]) groups[part] = [];
            groups[part].push(r);
        }
    });
    
    for (const part in groups) {
        const groupRooms = groups[part];
        if (groupRooms.length > 1) {
            // 複數出現，建立子目錄繼續遞迴
            node._subs[part] = buildRoomTree(groupRooms, index + 1);
        } else {
            // 單一出現，不建立資料夾，直接放入當前房間清單
            node._rooms.push(groupRooms[0]);
        }
    }
    
    return node;
}


// 修正：修正 removeObject 引用錯誤
function renderTreeNode(name, node, path, depth) {
    const collapsed = JSON.parse(localStorage.getItem('collapsed_folders') || '{}');
    const isCollapsed = collapsed[path] || false;
    const subNames = Object.keys(node._subs).sort();
    
    let html = `
        <div class="tree-node ${isCollapsed ? 'collapsed' : ''}">
            <div class="tree-header" onclick="toggleFolder('${path}')" style="padding-left: ${depth * 1.25 + 1}rem">
                <svg class="zone-chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="6 9 12 15 18 9"></polyline></svg>
                <span class="node-name">${name}</span>
                <span class="node-count">${node._rooms.length + subNames.length}</span>
            </div>
            <div class="tree-content">
    `;

    // 先渲染子目錄
    subNames.forEach(sn => {
        html += renderTreeNode(sn, node._subs[sn], (path ? path + '_' + sn : sn), depth + 1);
    });

    // 再渲染當前層級的房間
    if (node._rooms.length > 0) {
        html += `
            <div class="table-wrap table-compact" style="margin-left: ${(depth + 1) * 0.8 + 0.2}rem; border-left: 1px solid var(--glass-border);">
                <table>
                    <tbody>
                        ${node._rooms.map(r => `
                            <tr>
                                <td style="width: 180px;"><code>${r.id}</code></td>
                                <td>${r.name}</td>
                                <td class="action-cell">
                                    <button class="btn btn-small btn-primary" onclick="openEditor('${r.id}')">編輯</button>
                                    <button class="btn btn-small btn-danger" onclick="deleteRoom('${r.id}')">刪除</button>
                                </td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            </div>
        `;
    }

    html += `</div></div>`;
    return html;
}

function renderRoomsTable() {
    const search = ui.roomSearch.value.toLowerCase();
    const filtered = state.rooms.filter(r => 
        r.id.toLowerCase().includes(search) || 
        r.name.toLowerCase().includes(search)
    ).sort((a, b) => a.id.localeCompare(b.id));

    if (filtered.length === 0) {
        ui.roomsContainer.innerHTML = '<div class="card glass" style="text-align:center; padding: 2rem; color: var(--text-secondary);">找不到匹配的房間</div>';
        return;
    }

    const tree = buildRoomTree(filtered);
    ui.roomsContainer.innerHTML = '';
    
    // 渲染根直接屬下的子目錄
    Object.keys(tree._subs).sort().forEach(sn => {
        ui.roomsContainer.innerHTML += renderTreeNode(sn, tree._subs[sn], sn, 0);
    });
    
    // 渲染根直接屬下的房間
    if (tree._rooms.length > 0) {
        ui.roomsContainer.innerHTML += `
            <div class="table-wrap table-compact" style="padding: 0.25rem 0;">
                <table>
                    <tbody>
                        ${tree._rooms.map(r => `
                            <tr>
                                <td style="width: 180px;"><code>${r.id}</code></td>
                                <td>${r.name}</td>
                                <td style="width: 140px; text-align: right;">
                                    <button class="btn btn-small btn-primary" onclick="openEditor('${r.id}')">編輯</button>
                                    <button class="btn btn-small btn-danger" onclick="deleteRoom('${r.id}')">刪除</button>
                                </td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            </div>
        `;
    }
}

window.toggleFolder = toggleFolder;
window.toggleZone = toggleZone;
window.openEditor = openEditor;
window.deleteRoom = deleteRoom;

async function deleteRoom(id) {
    if (!confirm(`確定刪除房間 ${id}？\n房內實體將會被移至大廳。`)) return;
    try {
        await api(`/api/rooms/${encodeURIComponent(id)}`, { method: 'DELETE' });
        toast('房間已刪除');
        loadRooms();
    } catch (e) {
        toast(`刪除失敗: ${e.message}`, true);
    }
}

async function renameRoom() {
    if (!state.selectedId) return;
    const oldId = state.selectedId;
    const newId = prompt(`請輸入房間 ${oldId} 的新 ID (建議英數與底線):`, oldId);
    if (!newId || newId === oldId) return;
    
    if (!confirm(`確定將房間 ID 從「${oldId}」重命名為「${newId}」嗎？\n這將級聯更新地圖上的所有引用（出口、物件），並重新命名原始 JSON 檔案。`)) return;
    
    try {
        await api(`/api/rooms/${encodeURIComponent(oldId)}/rename`, {
            method: 'POST',
            body: JSON.stringify({ new_id: newId })
        });
        toast(`成功重命名為 ${newId}`);
        state.selectedId = newId;
        await loadRooms();
        openEditor(newId);
    } catch (e) {
        toast(`重命名失敗: ${e.message}`, true);
    }
}

document.getElementById('btn-rename-id').onclick = renameRoom;

// --- Room Editor ---
async function openEditor(id) {
    try {
        // Find existing basic data or fetch if needed (though we have basic data from list)
        const basic = state.rooms.find(r => r.id === id);
        state.selectedId = id;
        
        // Fetch full data (objects etc are not in the list view)
        const data = await api(`/api/rooms/${encodeURIComponent(id)}`);
        // The API returns { room, exits }
        state.selectedRoom = data.room;
        state.selectedExits = data.exits || [];
        
        ui.fId.value = data.room.id;
        ui.fName.value = data.room.name || '';
        ui.fZone.value = data.room.zone || '';
        ui.fTags.value = (data.room.tags || []).join(', ');
        ui.fDesc.value = data.room.description || '';
        
        writeObjectsJson(data.room.objects || []);
        renderObjectsForm(data.room.objects || []);
        
        document.getElementById('nav-editor').style.display = 'flex';
        switchSection('section-editor');
        ui.pageTitle.textContent = `編輯房間: ${data.room.name || data.room.id}`;
    } catch (e) {
        toast(`載入編輯器失敗: ${e.message}`, true);
    }
}



// --- Objects Editor (Logic from room_editor.js) ---
function writeObjectsJson(objs) {
    ui.fObjects.value = JSON.stringify(objs || [], null, 2);
}

function readObjectsFromJson() {
    const val = ui.fObjects.value.trim();
    if (!val) return [];
    try {
        const data = JSON.parse(val);
        return Array.isArray(data) ? data : [];
    } catch(e) { return []; }
}

function normalizeObject(obj) {
    return {
        id: String(obj?.id || '').trim(),
        name: String(obj?.name || '').trim(),
        owner: String(obj?.owner != null ? obj.owner : '').trim(),
        sockets: Array.isArray(obj?.sockets) ? obj.sockets.map(s => String(s).trim()).filter(Boolean) : [],
        responses: obj?.responses && typeof obj.responses === 'object' ? obj.responses : {},
        move_to_room_id: String(obj?.move_to_room_id != null ? obj.move_to_room_id : '').trim(),
    };
}

function renderObjectsForm(objs) {
    ui.objectsForm.innerHTML = '';
    const sortedRooms = [...state.rooms].sort((a, b) => a.id.localeCompare(b.id));
    
    (objs || []).forEach((raw, idx) => {
        const obj = normalizeObject(raw);
        const row = document.createElement('div');
        row.className = 'obj-row';
        
        row.innerHTML = `
            <div class="obj-head">
                <input type="text" placeholder="ID" value="${obj.id}" data-idx="${idx}" data-key="id">
                <input type="text" placeholder="名稱" value="${obj.name}" data-idx="${idx}" data-key="name">
                <button class="btn btn-small btn-danger" onclick="removeObject(${idx})">刪除</button>
            </div>
            <div class="field">
                <label>Owner (可空)</label>
                <input type="text" value="${obj.owner}" data-idx="${idx}" data-key="owner">
            </div>
            <div class="field">
                <label>移動目標房 (Move 目標)</label>
                <select data-idx="${idx}" data-key="move_to_room_id">
                    <option value="">— 不切換房間 —</option>
                    ${sortedRooms.map(r => `<option value="${r.id}" ${r.id === obj.move_to_room_id ? 'selected' : ''}>${r.id}: ${r.name || '(無名稱)'}</option>`).join('')}
                </select>
            </div>
            <div class="behaviors-wrap" id="beh-wrap-${idx}"></div>
        `;
        
        ui.objectsForm.appendChild(row);
        renderBehaviors(row.querySelector('.behaviors-wrap'), obj, idx);
        
        // Bind inputs
        row.querySelectorAll('input, select').forEach(el => {
            el.oninput = () => {
                const list = readObjectsFromJson();
                list[idx][el.dataset.key] = el.value;
                writeObjectsJson(list);
            };
        });
    });
}

function renderBehaviors(container, obj, objIdx) {
    container.innerHTML = '<strong>行為</strong>';
    const rows = document.createElement('div');
    rows.className = 'beh-rows';
    
    obj.sockets.forEach(sock => {
        const brow = document.createElement('div');
        brow.className = 'beh-row';
        brow.innerHTML = `
            <span class="beh-lbl">${sock}</span>
            <textarea data-sock="${sock}">${obj.responses[sock] || ''}</textarea>
            <button class="btn-icon" onclick="removeSocket(${objIdx}, '${sock}')">✕</button>
        `;
        brow.querySelector('textarea').oninput = (e) => {
            const list = readObjectsFromJson();
            if (!list[objIdx].responses) list[objIdx].responses = {};
            list[objIdx].responses[sock] = e.target.value;
            writeObjectsJson(list);
        };
        rows.appendChild(brow);
    });
    
    const addBar = document.createElement('div');
    addBar.className = 'beh-add-bar';
    ['Look', 'Read', 'Use', 'Talk', 'Move'].forEach(b => {
        const btn = document.createElement('button');
        btn.className = 'btn btn-small';
        btn.textContent = `＋${b}`;
        btn.onclick = () => addSocket(objIdx, b);
        addBar.appendChild(btn);
    });
    
    container.appendChild(rows);
    container.appendChild(addBar);
}

function addSocket(objIdx, sock) {
    const list = readObjectsFromJson();
    if (!list[objIdx].sockets) list[objIdx].sockets = [];
    if (!list[objIdx].sockets.includes(sock)) {
        list[objIdx].sockets.push(sock);
        if (!list[objIdx].responses) list[objIdx].responses = {};
        if (!list[objIdx].responses[sock]) list[objIdx].responses[sock] = (sock === 'Look' ? '你看見它。' : '');
        writeObjectsJson(list);
        renderObjectsForm(list);
    }
}

function removeSocket(objIdx, sock) {
    const list = readObjectsFromJson();
    list[objIdx].sockets = (list[objIdx].sockets || []).filter(s => s !== sock);
    if (list[objIdx].responses) delete list[objIdx].responses[sock];
    writeObjectsJson(list);
    renderObjectsForm(list);
}

window.removeObject = (idx) => {
    const list = readObjectsFromJson();
    list.splice(idx, 1);
    writeObjectsJson(list);
    renderObjectsForm(list);
};

document.getElementById('btn-add-object').onclick = () => {
    const list = readObjectsFromJson();
    list.push({ id: `obj_${Date.now()}`, name: '新物件', sockets: ['Look'], responses: { Look: '你看見一個新的物件。' } });
    writeObjectsJson(list);
    renderObjectsForm(list);
};

// --- Save & Global Actions ---
async function saveRoom() {
    if (!state.selectedId) return;
    try {
        const payload = {
            name: ui.fName.value.trim(),
            description: ui.fDesc.value.trim(),
            tags: ui.fTags.value.split(',').map(t => t.trim()).filter(Boolean),
            zone: ui.fZone.value.trim(),
            objects: readObjectsFromJson(),
        };
        
        await api(`/api/rooms/${encodeURIComponent(state.selectedId)}`, {
            method: 'PUT',
            body: JSON.stringify(payload)
        });
        toast(' rooms 已儲存');
        loadRooms();
    } catch(e) { toast(`儲存失敗: ${e.message}`, true); }
}

document.getElementById('btn-save-room').onclick = saveRoom;
window.addEventListener('keydown', e => {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        saveRoom();
    }
});

document.getElementById('btn-wipe-entities').onclick = async () => {
    if (!confirm('!!! 警告 !!!\n確定要清除所有實體嗎？這將重置所有玩家與 NPC 的位置與數據。')) return;
    try {
        await api('/api/admin/wipe-entities', { method: 'POST' });
        toast('已清除所有實體');
    } catch(e) { toast(`操作失敗: ${e.message}`, true); }
};

document.getElementById('btn-reload-rooms').onclick = async () => {
    if (!confirm('!!! 警告 !!!\n確定要從檔案重新載入所有房間嗎？這將會根據 JSON 內容覆蓋資料庫中的資料。')) return;
    try {
        toast('正在從檔案同步到資料庫...');
        await api('/api/room-editor/reload', { method: 'POST' });
        toast('同步成功！');
        loadRooms(); // 重新整理列表
    } catch(e) { toast(`同步失敗: ${e.message}`, true); }
};

// --- Modal Add Room ---
document.getElementById('btn-show-add-room').onclick = () => {
    document.getElementById('modal-add-room').classList.add('active');
};

document.getElementById('btn-confirm-add-room').onclick = async () => {
    const id = document.getElementById('add-id').value.trim();
    const name = document.getElementById('add-name').value.trim();
    const desc = document.getElementById('add-desc').value.trim();
    if (!id) return toast('請填寫 ID', true);
    
    try {
        await api('/api/rooms', {
            method: 'POST',
            body: JSON.stringify({ id, name, description: desc })
        });
        toast('房間已建立');
        document.getElementById('modal-add-room').classList.remove('active');
        loadRooms();
    } catch(e) { toast(`建立失敗: ${e.message}`, true); }
};

// --- Init ---
loadRooms();
switchSection('section-dashboard');
