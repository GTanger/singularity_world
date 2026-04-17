/**
 * Game UI v0.19.1 — 文字 MUD 介面：主輸出、標籤面板、方向鈕
 */
console.log('[game-ui] v0.19.1 loaded');

// --- 房間資料 ---
var currentRoomData = {
  description: '',
  entities: [],
  objects: [],
  exits: [],
  room_name: ''
};

// --- 移動狀態 ---
var mapState = { moving: false, playerX: 0, playerY: 0 };
window.mapState = mapState;

// --- DOM ---
var logEl = document.getElementById('log');
var mainOutput = document.getElementById('main-output');
var tabBar = document.getElementById('tab-bar');
var tabPanel = document.getElementById('tab-panel');
var roomInfo = document.getElementById('room-info');
var dpad = document.getElementById('dpad');
var activeTab = 'room';

// --- Log ---
var logStickToBottom = true;

function ensureLogScroll() {
  if (!mainOutput) return;
  mainOutput.addEventListener('scroll', function () {
    var delta = mainOutput.scrollHeight - mainOutput.clientHeight - mainOutput.scrollTop;
    logStickToBottom = delta <= 32;
  }, { passive: true });
}
ensureLogScroll();

function scrollLogToBottom() {
  if (mainOutput && logStickToBottom) {
    mainOutput.scrollTop = mainOutput.scrollHeight;
  }
}

function appendLog(text) {
  if (!logEl) return;
  var div = document.createElement('div');
  div.className = 'log-entry log-system';
  div.textContent = text;
  logEl.appendChild(div);
  scrollLogToBottom();
}
window.appendLog = appendLog;

function escapeHtml(s) {
  var d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

// --- 房間進入時寫入 log ---
function appendRoomEntry(data) {
  if (!logEl) return;
  var div = document.createElement('div');
  div.className = 'log-entry log-room-entry';

  var nameDiv = document.createElement('div');
  nameDiv.className = 'log-room-name';
  nameDiv.textContent = '【' + (data.room_name || '未知') + '】';
  div.appendChild(nameDiv);

  if (data.description) {
    var descDiv = document.createElement('div');
    descDiv.className = 'log-room-desc';
    descDiv.textContent = data.description;
    div.appendChild(descDiv);
  }

  var exits = data.exits || [];
  if (exits.length > 0) {
    var exitsDiv = document.createElement('div');
    exitsDiv.className = 'log-room-exits';
    var exitsText = '';
    for (var i = 0; i < exits.length; i++) {
      var ex = exits[i];
      if (i > 0) exitsText += '　';
      exitsText += ex.direction + '→' + (ex.to_room_name || '?');
    }
    exitsDiv.textContent = exitsText;
    div.appendChild(exitsDiv);
  }

  var myId = window.myPlayerId || '';
  var entities = (data.entities || []).filter(function (e) {
    return !(e.kind === 'player' && e.id === myId);
  });
  if (entities.length > 0) {
    var entDiv = document.createElement('div');
    entDiv.className = 'log-room-entities';
    var names = [];
    for (var ei = 0; ei < entities.length; ei++) {
      names.push(entities[ei].display_name || entities[ei].id);
    }
    entDiv.textContent = '在場：' + names.join('、');
    div.appendChild(entDiv);
  }

  var objects = data.objects || [];
  if (objects.length > 0) {
    var objDiv = document.createElement('div');
    objDiv.className = 'log-room-objects';
    var objNames = [];
    for (var oi = 0; oi < objects.length; oi++) {
      objNames.push(objects[oi].name);
    }
    objDiv.textContent = '地上：' + objNames.join('、');
    div.appendChild(objDiv);
  }

  logEl.appendChild(div);
  scrollLogToBottom();
}

// --- 房間面板 ---
function renderRoomPanel(data) {
  if (!roomInfo) return;
  var html = '';

  html += '<div class="ri-name">【' + escapeHtml(data.room_name || '未知') + '】</div>';
  if (data.description) {
    html += '<div class="ri-desc">' + escapeHtml(data.description) + '</div>';
  }

  var exits = data.exits || [];
  if (exits.length > 0) {
    html += '<div class="ri-section-title">出口</div>';
    for (var i = 0; i < exits.length; i++) {
      var ex = exits[i];
      html += '<span class="ri-exit" data-dir="' + escapeHtml(ex.direction) + '">'
        + escapeHtml(ex.direction) + '→' + escapeHtml(ex.to_room_name || '?') + '</span>';
    }
  }

  var myId = window.myPlayerId || '';
  var entities = (data.entities || []).filter(function (e) {
    return !(e.kind === 'player' && e.id === myId);
  });
  if (entities.length > 0) {
    html += '<div class="ri-section-title">在場</div>';
    for (var ei = 0; ei < entities.length; ei++) {
      var ent = entities[ei];
      html += '<div class="ri-entity" data-entity-id="' + escapeHtml(ent.id) + '">'
        + escapeHtml(ent.display_name || ent.id) + '</div>';
    }
  }

  var objects = data.objects || [];
  if (objects.length > 0) {
    html += '<div class="ri-section-title">物品</div>';
    for (var oi = 0; oi < objects.length; oi++) {
      var obj = objects[oi];
      html += '<div class="ri-object" data-object-id="' + escapeHtml(obj.id) + '">'
        + escapeHtml(obj.name) + '</div>';
    }
  }

  roomInfo.innerHTML = html;
}

// --- 方向鈕狀態 ---
var DIR_LIST = ['西北', '北', '東北', '西', '', '東', '西南', '南', '東南'];

function updateDpadExits(exits) {
  if (!dpad) return;
  var exitDirs = {};
  for (var i = 0; i < (exits || []).length; i++) {
    exitDirs[exits[i].direction] = true;
  }
  var buttons = dpad.querySelectorAll('.dir-btn');
  for (var bi = 0; bi < buttons.length; bi++) {
    var btn = buttons[bi];
    var dir = btn.getAttribute('data-dir');
    if (!dir) continue;
    if (exitDirs[dir]) {
      btn.classList.add('has-exit');
      btn.classList.remove('no-exit');
    } else {
      btn.classList.remove('has-exit');
      btn.classList.add('no-exit');
    }
  }
}

// --- server 推送入口 ---
window.updateRoomData = function (data) {
  currentRoomData.description = data.description || '';
  currentRoomData.room_name = data.room_name || '';
  currentRoomData.entities = data.entities || [];
  currentRoomData.objects = data.objects || [];
  currentRoomData.exits = data.exits || [];
};

window.onGridView = function (data) {
  mapState.playerX = data.player_x;
  mapState.playerY = data.player_y;
  mapState.moving = false;

  appendRoomEntry(data);
  renderRoomPanel(data);
  updateDpadExits(data.exits || []);
};

// --- 標籤切換 ---
function switchTab(tab) {
  if (activeTab === tab) {
    tabPanel.classList.toggle('expanded');
    return;
  }
  activeTab = tab;
  tabPanel.classList.add('expanded');

  var btns = tabBar.querySelectorAll('.tab-btn');
  for (var i = 0; i < btns.length; i++) {
    btns[i].classList.toggle('active', btns[i].getAttribute('data-tab') === tab);
  }

  var panes = tabPanel.querySelectorAll('.tab-pane');
  for (var pi = 0; pi < panes.length; pi++) {
    var on = panes[pi].getAttribute('data-tab') === tab;
    panes[pi].classList.toggle('active', on);
    panes[pi].hidden = !on;
  }

  if (tab === 'inventory' && window.gameSend) {
    window.gameSend({ type: 'get_inventory' });
  }
  if (tab === 'character' && window.gameSend) {
    var myId = window.myPlayerId || '';
    if (myId) window.gameSend({ type: 'get_entity_status', entity_id: myId });
  }
}

if (tabBar) {
  tabBar.addEventListener('click', function (e) {
    var btn = e.target.closest('.tab-btn');
    if (!btn) return;
    var tab = btn.getAttribute('data-tab');
    if (tab) switchTab(tab);
  });
}

// --- 方向鈕點擊 ---
function directionLabel(dir) {
  var valid = ['北', '東北', '東', '東南', '南', '西南', '西', '西北'];
  return valid.indexOf(dir) !== -1 ? dir : null;
}

function doMove(direction) {
  if (mapState.moving) {
    appendLog('⏳ 移動中…');
    return;
  }
  var dir = directionLabel(direction);
  if (!dir) return;

  mapState.moving = true;
  appendLog('→ 往' + dir + '移動');
  if (window.gameSend) {
    window.gameSend({ type: 'move', direction: dir });
  }
  setTimeout(function () { mapState.moving = false; }, 1500);
}

if (dpad) {
  dpad.addEventListener('click', function (e) {
    var btn = e.target.closest('.dir-btn');
    if (!btn) return;
    var dir = btn.getAttribute('data-dir');
    if (dir) doMove(dir);
  });
}

// look 鈕
var lookBtn = document.getElementById('look-btn');
if (lookBtn) {
  lookBtn.addEventListener('click', function () {
    switchTab('room');
  });
}

// --- 房間面板互動 ---
if (roomInfo) {
  roomInfo.addEventListener('click', function (e) {
    var exit = e.target.closest('.ri-exit');
    if (exit) {
      var dir = exit.getAttribute('data-dir');
      if (dir) doMove(dir);
      return;
    }
    var entity = e.target.closest('.ri-entity');
    if (entity && window.gameSend) {
      var eid = entity.getAttribute('data-entity-id');
      if (eid) window.gameSend({ type: 'do_action', entity_id: eid, action: 'Look' });
      return;
    }
    var obj = e.target.closest('.ri-object');
    if (obj && window.gameSend) {
      var oid = obj.getAttribute('data-object-id');
      if (oid) window.gameSend({ type: 'do_action', entity_id: oid, action: 'Look' });
      return;
    }
  });
}

// --- 玩家姓名點擊 → 切人物標籤 ---
var playerNameEl = document.getElementById('player-name');
if (playerNameEl) {
  playerNameEl.addEventListener('click', function () {
    switchTab('character');
  });
}

// --- 相容舊介面 ---
window.closeDrawers = function () {};
window.triggerLook = function () {};
