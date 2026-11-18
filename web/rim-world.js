(() => {
  'use strict';

  const $ = (id) => document.getElementById(id);
  const canvas = $('rim-canvas');
  const ctx = canvas.getContext('2d');

  // 鏡頭狀態：camX, camY 為 1m 細格世界座標；cellPx 為每 1 個 1m 細格之螢幕像素大小
  let camX = 0.0;
  let camY = 3.0;
  let cellPx = 32.0; // 預設近景 1m 視角；縮小 < 5.0 時平滑退回 20m 全島山海母圖
  const MIN_CELL_PX = 0.018;
  const MAX_CELL_PX = 72.0;

  let followMe = true;
  let meta = null;
  let basemapImg = null;

  // ─ 角色與建物輔助狀態（全視覺改由 CddaTiles 渲染） ─
  let selectedSkillIdx = 0;
  function buildingCodeAt(mx, my) {
    if (!Array.isArray(worldBuildings)) return '';
    for (const b of worldBuildings) {
      if (mx >= b.x0 && mx <= b.x1 && my >= b.y0 && my <= b.y1) return b.code || '';
    }
    return '';
  }

  const CHOP_DURATION = 0.52;
  const WORKING_ACT_RE = /^(親手採伐|採伐|親手築造|築造|拆除|打造|製作|烹製|赴火|拆招|迎擊|上前交鋒|揮鎬翻土|翻妥|播種|扦插|採收|採挖|搗藥|搗製|敷藥|赴火炙)/;

  let cellMap = new Map();
  let actors = [];
  let meDetail = null;
  let worldWeather = { code: 'clear', label: '晴風和暢', raining: false };
  let worldRooms = [];
  let worldBuildings = [];
  let indoorCellSet = new Set();
  const roomRoofAlpha = new Map();
  let bedBadgeQueue = [];
  let renderActors = new Map(); // id -> { x, y, srvX, srvY, facing, targetFacing, speed, running, path, walkPhase, moveBlend, runBlend, chopTimer, chopDidImpact, lastPop }
  let meMoveSeq = 0;
  let meMoveLockUntil = 0;
  let meCmdInFlight = 0;
  let meArriveHoldUntil = 0;
  const bambooShakeMap = new Map(); // "mx,my" -> remaining sec
  const leafParticles = []; // { x, y, vx, vy, rot, life }
  let selectedCell = null; // { mx, my }
  let activeModal = null; // null | 'self' | 'gear' | 'settings' | 'talk'
  let talkState = null;
  let talkBusy = false;
  let lastEventsKey = '';
  let lastInspectorSig = '';
  let lastModalSig = '';
  let prevFrameMs = 0;
  let pollCounter = 0;
  let forceCellsNext = true;
  let lastTap = { ms: 0, sx: -999, sy: -999, mx: -999, my: -999 };
  let longPressTimer = null;

  // 指標與雙指觸控狀態
  const pointers = new Map();
  let pinchDist0 = 0;
  let pinchCellPx0 = 32;
  let dragStart = null; // { sx, sy, camX0, camY0, moved, longPressed }

  function hash2d(mx, my) {
    let h = Math.imul(mx | 0, 374761393) + Math.imul(my | 0, 668265263);
    h = (h ^ (h >>> 13)) >>> 0;
    h = Math.imul(h, 1274126177) >>> 0;
    return (h ^ (h >>> 16)) >>> 0;
  }

  function hashStr(s) {
    let h = 2166136261 >>> 0;
    const str = String(s || '');
    for (let i = 0; i < str.length; i++) {
      h ^= str.charCodeAt(i);
      h = Math.imul(h, 16777619) >>> 0;
    }
    return h >>> 0;
  }

  function lerp(a, b, t) {
    return a + (b - a) * t;
  }

  function lerpPt(a, b, t) {
    return [lerp(a[0], b[0], t), lerp(a[1], b[1], t)];
  }

  function smoothstep(t) {
    return t * t * (3 - 2 * t);
  }

  function isWaterGround(g) {
    return g === 'stream' || g === 'spring' || g === 'deep';
  }

  function getEffectiveWeather() {
    if (window.__swQaWeather === 'rain') {
      return { code: 'rain', label: '瀟瀟山雨', raining: true };
    }
    if (window.__swQaWeather === 'clear') {
      return { code: 'clear', label: '晴風和暢', raining: false };
    }
    return worldWeather || { code: 'clear', label: '晴風和暢', raining: false };
  }

  function roomKeyOf(rm) {
    return `${rm.x0},${rm.y0},${rm.x1},${rm.y1}`;
  }

  function getRoomRoofAlphaAt(mx, my) {
    for (const rm of worldRooms) {
      if (mx >= rm.x0 && mx <= rm.x1 && my >= rm.y0 && my <= rm.y1) {
        return roomRoofAlpha.has(roomKeyOf(rm)) ? roomRoofAlpha.get(roomKeyOf(rm)) : 1.0;
      }
    }
    return 0.0;
  }

  function findBedSleeper(mx, my) {
    for (const a of actors) {
      if (a.faction === 'wild_beast' || a.act === '力竭倒地') continue;
      if (!/榻上安歇/.test(String(a.act || ''))) continue;
      const st = renderActors.get(a.id);
      const ax = st ? st.x : a.px;
      const ay = st ? st.y : a.py;
      if (Math.hypot(ax - mx, ay - my) <= 0.58) return a;
    }
    return null;
  }

  function isActorSleepingInBed(a, pos) {
    if (!a || a.faction === 'wild_beast' || a.act === '力竭倒地') return false;
    if (!/榻上安歇/.test(String(a.act || ''))) return false;
    const gx = Math.round(pos.x);
    const gy = Math.round(pos.y);
    const cell = cellMap.get(`${gx},${gy}`);
    return Boolean(cell && cell.s === 'bed' && Math.hypot(pos.x - gx, pos.y - gy) <= 0.58);
  }

  let viewportW = 1;
  let viewportH = 1;

  function clearLongPress() {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }
  }

  function showToast(msg) {
    const el = $('rim-toast');
    if (!el) return;
    el.textContent = msg;
    el.hidden = false;
    clearTimeout(showToast.timer);
    showToast.timer = setTimeout(() => { el.hidden = true; }, 2400);
  }

  function resize() {
    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    viewportW = Math.max(1, rect.width);
    viewportH = Math.max(1, rect.height);
    canvas.width = Math.round(viewportW * dpr);
    canvas.height = Math.round(viewportH * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  function screenToMicro(sx, sy) {
    const mx = camX + (sx - viewportW / 2) / cellPx;
    const my = camY - (sy - viewportH / 2) / cellPx;
    return { mx: Math.round(mx), my: Math.round(my), fx: mx, fy: my };
  }

  function microToScreen(mx, my) {
    return {
      x: viewportW / 2 + (mx - camX) * cellPx,
      y: viewportH / 2 - (my - camY) * cellPx,
    };
  }

  function tileEdges(mx, my) {
    const x0 = Math.round(viewportW / 2 + (mx - 0.5 - camX) * cellPx);
    const x1 = Math.round(viewportW / 2 + (mx + 0.5 - camX) * cellPx);
    const y0 = Math.round(viewportH / 2 - (my + 0.5 - camY) * cellPx);
    const y1 = Math.round(viewportH / 2 - (my - 0.5 - camY) * cellPx);
    return { x0, y0, x1, y1, w: x1 - x0, h: y1 - y0 };
  }

  async function loadSubstrateMeta() {
    try {
      const res = await fetch('/api/substrate/meta');
      if (!res.ok) return;
      const d = await res.json();
      if (d && d.available) {
        meta = d;
        const img = new Image();
        img.src = '/api/substrate/basemap.png?mode=terrain';
        img.onload = () => { basemapImg = img; };
      }
    } catch (_) {}
  }

  function resetWorldSessionState() {
    closeInspector();
    closeModal();
    actors = [];
    meDetail = null;
    cellMap = new Map();
    renderActors.clear();
    bambooShakeMap.clear();
    leafParticles.length = 0;
    lastEventsKey = '';
    lastInspectorSig = '';
    lastModalSig = '';
  }

  function refreshAuthClock() {
    if (!window.SwSkyTime) return;
    const info = window.SwSkyTime.applyDocumentSkyCss();
    const clk = $('auth-clock');
    if (clk && info) clk.textContent = info.clockText;
  }

  function enterAuthScreen(errMsg) {
    sessionStorage.removeItem('sw_tab_session_ok');
    resetWorldSessionState();
    const authEl = $('auth-screen');
    const appEl = $('rim-app');
    if (appEl) appEl.hidden = true;
    if (authEl) authEl.hidden = false;
    const idInput = $('auth-id');
    const pwInput = $('auth-password');
    const errEl = $('auth-message');
    const hintEl = $('auth-hint');
    if (idInput && !idInput.value) {
      idInput.value = localStorage.getItem('sw_last_id') || '';
    }
    if (pwInput) pwInput.value = '';
    if (errEl) errEl.textContent = errMsg || '';
    if (hintEl) hintEl.textContent = '請輸入 ID 與密碼登入';
    refreshAuthClock();
    if (idInput && idInput.value && pwInput) {
      try { pwInput.focus(); } catch (_) {}
    } else if (idInput) {
      try { idInput.focus(); } catch (_) {}
    }
  }

  async function sendCmd(payload) {
    try {
      const res = await fetch('/api/rim/cmd', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      return await res.json();
    } catch (_) {
      return null;
    }
  }

  function updatePersonalHud() {
    if (!meDetail) return;
    const nameEl = $('lbl-self-name');
    const stateEl = $('lbl-self-state');
    const gearEl = $('lbl-gear-sub');
    const selfBtn = $('btn-self-sheet');
    if (nameEl) nameEl.textContent = meDetail.name || '自身';
    if (stateEl) {
      const feels = Array.isArray(meDetail.feels) ? meDetail.feels : [];
      const urgentFeel = feels.find((f) => /微飢|飢腸|飢疲|微渴|舌燥|渴極|疲態|步履沉重/.test(f));
      const headTag = urgentFeel || meDetail.health || '氣息勻暢';
      stateEl.textContent = meDetail.autopilot
        ? '自主行事中'
        : `${headTag}・${meDetail.act || '駐足'}`;
    }
    if (selfBtn) {
      selfBtn.classList.toggle('rim-chip-warn', !!meDetail.autopilot);
    }
    if (gearEl) {
      const edge = meDetail.tool_note ? String(meDetail.tool_note).split('｜').pop() : '';
      const baseGear = `持：${meDetail.equipped_label || '空手'}`;
      gearEl.textContent = edge ? `${baseGear}・${edge}` : baseGear;
    }
    if (!selectedCell && meDetail.zone) {
      const zEl = $('rim-zone-title');
      if (zEl) zEl.textContent = meDetail.zone;
    }
    if (activeModal) {
      renderModal(false);
    }
  }

  let polling = false;
  let pendingForcePoll = false;
  async function pollState(forceCells = false) {
    const authEl = $('auth-screen');
    if (authEl && !authEl.hidden) return;
    if (polling) {
      if (forceCells) pendingForcePoll = true;
      return;
    }
    polling = true;
    try {
      pollCounter++;
      const wantCells = forceCells || forceCellsNext || (pollCounter % 3 === 1);
      forceCellsNext = false;
      const params = new URLSearchParams();
      if (cellPx >= 5.0 && wantCells) {
        const halfW = Math.min(40, Math.ceil(viewportW / cellPx / 2) + 4);
        const halfH = Math.min(40, Math.ceil(viewportH / cellPx / 2) + 4);
        const cx = Math.round(camX);
        const cy = Math.round(camY);
        params.set('min_mx', String(cx - halfW));
        params.set('max_mx', String(cx + halfW));
        params.set('min_my', String(cy - halfH));
        params.set('max_my', String(cy + halfH));
      }
      if (selectedCell) {
        params.set('sel_x', String(selectedCell.mx));
        params.set('sel_y', String(selectedCell.my));
      }
      const pollSeq = meMoveSeq;
      const pollStartedDuringCmd = meCmdInFlight > 0;
      const res = await fetch(`/api/rim/state?${params.toString()}`, { credentials: 'same-origin' });
      if (res.status === 401) {
        enterAuthScreen('登入狀態已失效，請重新輸入密碼登入');
        return;
      }
      if (res.ok) {
        const d = await res.json();
        if (d.sky_time && window.SwSkyTime) {
          window.SwSkyTime.syncFromServer(d.sky_time);
        }
        if (d.weather && typeof d.weather === 'object') {
          worldWeather = d.weather;
        }
        if (Array.isArray(d.buildings)) {
          worldBuildings = d.buildings;
        }
        if (Array.isArray(d.rooms)) {
          worldRooms = d.rooms;
          const nextIndoor = new Set();
          for (const rm of worldRooms) {
            if (Array.isArray(rm.cells)) {
              for (const pt of rm.cells) nextIndoor.add(`${pt[0]},${pt[1]}`);
            }
            if (Array.isArray(rm.doors)) {
              for (const dp of rm.doors) nextIndoor.add(`${dp[0]},${dp[1]}`);
            }
          }
          if (Array.isArray(d.cells)) {
            for (const c of d.cells) {
              if (c.s === 'arcade') nextIndoor.add(`${c.x},${c.y}`);
            }
          }
          indoorCellSet = nextIndoor;
        }
        const effWeather = getEffectiveWeather();
        const clk = $('rim-world-clock');
        if (clk) {
          const baseClk = window.SwSkyTime
            ? window.SwSkyTime.getClockInfo().clockText
            : (d.world_clock || clk.textContent);
          clk.textContent = effWeather.label ? `${baseClk}・${effWeather.label}` : baseClk;
        }
        if (d.me_detail) {
          meDetail = d.me_detail;
          updatePersonalHud();
        }
        if (cellPx >= 5.0 && wantCells && Array.isArray(d.cells)) {
          const nextMap = new Map();
          for (const c of d.cells) {
            nextMap.set(`${c.x},${c.y}`, c);
            if (c.s === 'arcade') indoorCellSet.add(`${c.x},${c.y}`);
          }
          cellMap = nextMap;
        }
        if (Array.isArray(d.actors)) {
          actors = d.actors;
          const activeIds = new Set();
          for (const a of actors) {
            activeIds.add(a.id);
            let st = renderActors.get(a.id);
            const srvPath = Array.isArray(a.path) ? a.path.map((p) => [p[0], p[1]]) : [];
            if (!st) {
              st = {
                x: a.px,
                y: a.py,
                srvX: a.px,
                srvY: a.py,
                facing: a.facing || 0,
                targetFacing: a.facing || 0,
                speed: a.speed || 1.35,
                running: !!a.running,
                autopilot: !!a.autopilot,
                srvMoving: srvPath.length > 0,
                path: srvPath,
                walkPhase: (Number(a.id) || 0) * 1.1,
                moveBlend: 0,
                runBlend: 0,
                chopTimer: 0,
                chopDidImpact: false,
                lastPop: a.pop || null,
              };
              renderActors.set(a.id, st);
            } else {
              const isManualMe = !!a.is_player && !a.autopilot;
              const staleMePoll =
                !!a.is_player && (pollSeq !== meMoveSeq || pollStartedDuringCmd || meCmdInFlight > 0);
              if (!staleMePoll) {
                st.srvX = a.px;
                st.srvY = a.py;
                st.srvMoving = srvPath.length > 0;
                st.autopilot = !!a.autopilot;
              }
              if (!staleMePoll && (!a.is_player || performance.now() >= meMoveLockUntil)) {
                st.speed = a.speed || 1.35;
                st.running = !!a.running;
                if (!isManualMe) {
                  st.path = srvPath.length > 0 ? trimPassedWaypoints(srvPath, st.x, st.y) : [];
                } else if (srvPath.length === 0) {
                  st.path = [];
                } else if (st.path && st.path.length > 0) {
                  const cg = st.path[st.path.length - 1];
                  const sg = srvPath[srvPath.length - 1];
                  if (cg[0] !== sg[0] || cg[1] !== sg[1]) {
                    st.path = trimPassedWaypoints(srvPath, st.x, st.y);
                  }
                } else if (
                  performance.now() >= meArriveHoldUntil &&
                  Math.hypot(srvPath[srvPath.length - 1][0] - st.x, srvPath[srvPath.length - 1][1] - st.y) > 0.35
                ) {
                  st.path = trimPassedWaypoints(srvPath, st.x, st.y);
                }
              }
              if (typeof a.facing === 'number' && !staleMePoll && (!isManualMe || !st.path || st.path.length === 0)) {
                st.targetFacing = a.facing;
              }
              const err = Math.hypot(a.px - st.x, a.py - st.y);
              if (!staleMePoll && (!isManualMe ? err > 2.8 : (performance.now() >= meMoveLockUntil && err > 4.5))) {
                st.x = a.px;
                st.y = a.py;
              }
              if (a.pop && (a.pop.startsWith('得') || a.pop.startsWith('建成')) && st.lastPop !== a.pop) {
                st.chopTimer = Math.max(st.chopTimer, CHOP_DURATION);
                st.chopDidImpact = false;
              }
              st.lastPop = a.pop || null;
            }
          }
          for (const id of renderActors.keys()) {
            if (!activeIds.has(id)) renderActors.delete(id);
          }
        }
        if (Array.isArray(d.events)) {
          const key = d.events.join('|');
          if (key !== lastEventsKey) {
            lastEventsKey = key;
            const wrap = $('rim-events');
            if (wrap) {
              wrap.innerHTML = '';
              for (const line of d.events.slice(0, 3)) {
                const div = document.createElement('div');
                div.className = 'rim-ev-item';
                div.textContent = line;
                wrap.appendChild(div);
              }
            }
          }
        }
        if (d.selected && selectedCell) {
          renderInspector(d.selected);
        }
        // 若行動裝置處於鎖屏節電導致 rAF 暫停，主動補繪一幀確保畫面即時同步
        if (!prevFrameMs || performance.now() - prevFrameMs > 180) {
          renderScene(performance.now());
        }
      }
    } catch (_) {
    } finally {
      polling = false;
      if (pendingForcePoll) {
        pendingForcePoll = false;
        pollState(true);
      }
    }
  }

  function closeInspector() {
    selectedCell = null;
    lastInspectorSig = '';
    const box = $('rim-inspector');
    if (box) box.hidden = true;
  }

  function openCellSocket(mx, my) {
    selectedCell = { mx, my };
    lastInspectorSig = '';
    pollState(true);
  }
  window.__swOpenSocketAt = openCellSocket;
  window.__swRenderActorSnapshot = () =>
    actors.map((a) => {
      const s = renderActors.get(a.id);
      return { id: a.id, name: a.name, px: s ? s.x : a.px, py: s ? s.y : a.py };
    });

  function renderInspector(sel) {
    const box = $('rim-inspector');
    if (!box) return;
    box.hidden = false;
    if (sel.zone) {
      const zEl = $('rim-zone-title');
      if (zEl) zEl.textContent = sel.zone;
    }
    const sig = JSON.stringify(sel);
    if (sig === lastInspectorSig) return;
    lastInspectorSig = sig;

    $('insp-title').textContent = sel.title || '地面';
    const portraitWrap = $('insp-portrait-wrap');
    const portraitCanvas = $('insp-portrait-canvas');
    if (portraitWrap && portraitCanvas) {
      if (sel.actor_profile && window.SwPortrait) {
        portraitWrap.hidden = false;
        window.SwPortrait.render(portraitCanvas, {
          mode: 'bust',
          width: 84,
          height: 98,
          id: sel.actor_profile.id,
          name: sel.actor_profile.name || sel.title || '角色',
          glyph: sel.actor_profile.glyph || (sel.actor_profile.name ? String(sel.actor_profile.name).slice(0, 1) : '人'),
          gender: sel.actor_profile.gender || 'M',
          role: sel.actor_profile.role || '',
          soul_seed: sel.actor_profile.soul_seed || '0',
          wearHat: sel.actor_profile.wear_hat !== false,
          showPack: Boolean(sel.actor_profile.show_pack || sel.actor_profile.carry),
          tool: sel.actor_profile.tool || null,
          hpRatio: typeof sel.actor_profile.hp_ratio === 'number' ? sel.actor_profile.hp_ratio : 1.0,
        });
      } else {
        portraitWrap.hidden = true;
      }
    }
    const tagsWrap = $('insp-tags');
    tagsWrap.innerHTML = '';
    for (const t of sel.tags || []) {
      const span = document.createElement('span');
      span.className = 'rim-tag';
      span.textContent = t;
      tagsWrap.appendChild(span);
    }
    let skWrap = tagsWrap.parentNode?.querySelector('.rim-insp-skills');
    if (!skWrap && tagsWrap.parentNode) {
      skWrap = document.createElement('div');
      skWrap.className = 'rim-insp-skills';
      tagsWrap.after(skWrap);
    }
    if (skWrap) {
      skWrap.innerHTML = '';
      const profSkills = Array.isArray(sel.actor_profile?.skills) ? sel.actor_profile.skills : [];
      skWrap.style.display = profSkills.length ? 'flex' : 'none';
      for (const sk of profSkills.slice(0, 2)) {
        const pill = document.createElement('span');
        pill.className = 'rim-tag rim-tag-skill';
        pill.textContent = typeof sk === 'string' ? sk : (sk?.label || '');
        if (sk && typeof sk === 'object' && sk.detail) pill.title = sk.detail;
        skWrap.appendChild(pill);
      }
    }
    const sockWrap = $('insp-sockets');
    sockWrap.innerHTML = '';
    for (const s of sel.sockets || []) {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'rim-socket-btn';
      btn.textContent = s.label;
      btn.onclick = async () => {
        if (s.tool) {
          await sendCmd({
            cmd: 'designate',
            x0: sel.mx,
            y0: sel.my,
            x1: sel.mx,
            y1: sel.my,
            tool: s.tool,
          });
          lastInspectorSig = '';
          pollState(true);
        } else if (s.cmd === 'talk') {
          openTalkModal(s.target_actor);
        } else if (s.cmd) {
          const r = await sendCmd({
            cmd: s.cmd,
            target_actor: s.target_actor,
            directive: s.directive,
            mx: sel.mx,
            my: sel.my,
            item_code: s.item_code,
          });
          if (r && r.msg) showToast(r.msg);
          if (s.cmd === 'shop_cmd' && r && r.ok) {
            applyServerPathToMe(r);
            if (Array.isArray(r.path) && r.path.length > 0) closeInspector();
          }
          lastInspectorSig = '';
          pollState(true);
        } else if (s.action) {
          if (['chop', 'build', 'craft_blade', 'craft_pick', 'craft_alloy_blade', 'craft_alloy_pick', 'craft_salve', 'craft_hat', 'craft_pack', 'cook_shoot', 'cook_tuber', 'cook_meat', 'drink', 'attack_actor'].includes(s.action)) {
            const me = actors.find((x) => x.is_player);
            const meSt = me ? renderActors.get(me.id) : null;
            if (me && meSt && Math.hypot(me.px - sel.mx, me.py - sel.my) <= 1.55) {
              meSt.targetFacing = Math.atan2(sel.my - meSt.y, sel.mx - meSt.x);
              meSt.chopTimer = CHOP_DURATION;
              meSt.chopDidImpact = false;
            }
          }
          const r = await dispatchPlayerAct({
            mx: sel.mx,
            my: sel.my,
            action: s.action === 'attack_actor' ? 'move' : s.action,
            target_actor: s.target_actor,
            run: false,
          });
          if (r && !r.ok && r.msg) {
            showToast(r.msg);
          } else if (s.action === 'move') {
            closeInspector();
          }
          lastInspectorSig = '';
          pollState(true);
        }
      };
      sockWrap.appendChild(btn);
    }
  }

  function applyServerPathToMe(r) {
    if (!r || !r.ok || !Array.isArray(r.path) || r.path.length === 0) return;
    const me = actors.find((a) => a.is_player);
    if (!me) return;
    const st = renderActors.get(me.id);
    if (!st) return;
    const mapped = r.path.map((p) => [p[0], p[1]]);
    const nextPath = trimPassedWaypoints(mapped, st.x, st.y);
    meMoveSeq += 1;
    meMoveLockUntil = performance.now() + 900;
    meArriveHoldUntil = 0;
    st.path = nextPath.length > 0 ? nextPath : mapped;
    st.srvMoving = true;
    st.autopilot = false;
    if (typeof r.speed === 'number' && r.speed > 0) st.speed = r.speed;
    st.running = Boolean(r.running);
    followMe = true;
    $('btn-focus-me')?.classList.add('rim-chip-active');
  }

  function closeModal() {
    activeModal = null;
    lastModalSig = '';
    const bd = $('rim-modal-backdrop');
    if (bd) bd.hidden = true;
  }

  function openModal(kind) {
    closeInspector();
    activeModal = kind;
    lastModalSig = '';
    const bd = $('rim-modal-backdrop');
    if (bd) bd.hidden = false;
    renderModal(true);
  }

  async function sendTalkRequest(targetActor, topic, message) {
    talkBusy = true;
    if (activeModal === 'talk') {
      lastModalSig = '';
      renderModal(true);
    }
    try {
      const res = await fetch('/api/rim/talk', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ target_actor: targetActor, topic, message }),
      });
      const data = await res.json();
      if (data && data.ok) {
        talkState = data;
        lastInspectorSig = '';
        pollState(true);
      } else if (data && data.msg) {
        showToast(data.msg);
      }
    } catch (_) {
      showToast('對方一時未能聽清');
    } finally {
      talkBusy = false;
      if (activeModal === 'talk') {
        lastModalSig = '';
        renderModal(true);
      }
    }
  }

  async function openTalkModal(targetActor) {
    talkState = null;
    openModal('talk');
    await sendTalkRequest(targetActor, 'open', '');
  }

  function buildModalHero(d, infoChild) {
    const hero = document.createElement('div');
    hero.className = 'rim-modal-hero';
    const pBox = document.createElement('div');
    pBox.className = 'rim-modal-portrait-box';
    const cv = document.createElement('canvas');
    pBox.appendChild(cv);
    hero.appendChild(pBox);

    const info = document.createElement('div');
    info.className = 'rim-modal-hero-info';
    if (infoChild) info.appendChild(infoChild);
    hero.appendChild(info);

    if (window.SwPortrait) {
      const meActor = actors.find((a) => a.is_player);
      window.SwPortrait.render(cv, {
        mode: 'full',
        width: 136,
        height: 206,
        id: d.id || 1,
        name: d.name || '自身',
        glyph: d.glyph || (d.name ? String(d.name).slice(0, 1) : '我'),
        gender: d.gender || 'M',
        role: d.role || (meActor && meActor.role) || '',
        soul_seed: d.soul_seed || '0',
        wearHat: typeof d.equipped_hat === 'boolean' ? d.equipped_hat : (d.wear_hat !== false),
        showPack: Boolean(d.equipped_pack || d.show_pack),
        tool: d.equipped_code ? (d.equipped_label || d.equipped_code) : null,
        hpRatio: typeof d.hp_ratio === 'number' ? d.hp_ratio : 1.0,
      });
    }
    return { hero, info };
  }

  function renderModal(force) {
    if (!activeModal) return;
    const d = meDetail || {
      id: 1,
      name: '自身',
      glyph: '我',
      gender: 'M',
      soul_seed: '0',
      wear_hat: true,
      equipped_hat: true,
      equipped_pack: true,
      hp_ratio: 1.0,
      health: '氣息勻暢',
      feels: ['腹中溫飽', '喉間潤澤', '神采奕奕'],
      magnesium_tag: '鎂晶數枚',
      act: '駐足待命',
      autopilot: false,
      directive: 'forage',
      equipped_code: null,
      equipped_label: '空手',
      skills: [{ label: '步履穩健', detail: '初入山林，踏勘漸穩。' }],
      recent_diary: [],
      inventory: [],
    };
    const sig = activeModal === 'talk'
      ? JSON.stringify({ mode: 'talk', talkState, talkBusy })
      : JSON.stringify({ mode: activeModal, d });
    if (!force && sig === lastModalSig) return;
    const focusId = document.activeElement ? document.activeElement.id : '';
    if (!force && (focusId === 'rim-byok-input' || focusId === 'rim-talk-input')) return;
    lastModalSig = sig;

    const titleEl = $('rim-modal-title');
    const bodyEl = $('rim-modal-body');
    if (!titleEl || !bodyEl) return;
    bodyEl.innerHTML = '';

    if (activeModal === 'self') {
      titleEl.textContent = `${d.name || '自身'}・自身狀態`;

      const secState = document.createElement('div');
      secState.innerHTML = `<div class="rim-sec-label">體魄與當下動靜</div>`;
      const stateRow = document.createElement('div');
      stateRow.className = 'rim-row-pills';
      const genderTag = d.gender === 'F' ? '女' : '男';
      const selfTags = [
        `印章：${d.glyph || '我'}・${genderTag}`,
        d.health || '氣息勻暢',
        d.act || '駐足待命',
        ...(Array.isArray(d.feels) ? d.feels : []),
        d.magnesium_tag || '鎂晶數枚',
      ];
      for (const txt of selfTags) {
        const sp = document.createElement('span');
        sp.className = 'rim-tag';
        sp.textContent = txt;
        stateRow.appendChild(sp);
      }
      secState.appendChild(stateRow);
      if (d.has_debt) {
        const repayRow = document.createElement('div');
        repayRow.className = 'rim-row-pills';
        const repayBtn = document.createElement('button');
        repayBtn.type = 'button';
        repayBtn.className = 'rim-chip-btn';
        repayBtn.textContent = '結清濟安診所診金';
        repayBtn.onclick = async () => {
          const r = await sendCmd({ cmd: 'shop_cmd', mx: 15, my: -1, item_code: 'repay_debt' });
          if (r && r.msg) showToast(r.msg);
          if (r && r.ok) applyServerPathToMe(r);
          closeModal();
          pollState(true);
        };
        repayRow.appendChild(repayBtn);
        secState.appendChild(repayRow);
      }

      const { hero } = buildModalHero(d, secState);
      bodyEl.appendChild(hero);

      const secSkills = document.createElement('div');
      secSkills.innerHTML = `<div class="rim-sec-label">自然身手（隨實地歷練精進，點按可察詳）</div>`;
      const skillsList = (Array.isArray(d.skills) && d.skills.length)
        ? d.skills
        : [{ label: '步履穩健', detail: '初入山林，踏勘漸穩。' }];
      if (selectedSkillIdx < 0 || selectedSkillIdx >= skillsList.length) selectedSkillIdx = 0;
      const skillRow = document.createElement('div');
      skillRow.className = 'rim-row-pills';
      const detailBox = document.createElement('div');
      detailBox.className = 'rim-skill-detail';
      const renderSkillDetail = () => {
        const cur = skillsList[selectedSkillIdx];
        const txt = typeof cur === 'string' ? cur : (cur?.detail || cur?.label || '');
        detailBox.textContent = txt;
        const chips = skillRow.querySelectorAll('.rim-tag-skill');
        chips.forEach((el, idx) => el.classList.toggle('is-active', idx === selectedSkillIdx));
      };
      skillsList.forEach((sk, idx) => {
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = idx === selectedSkillIdx ? 'rim-tag rim-tag-skill is-active' : 'rim-tag rim-tag-skill';
        btn.textContent = typeof sk === 'string' ? sk : (sk?.label || '');
        btn.onclick = () => {
          selectedSkillIdx = idx;
          renderSkillDetail();
        };
        skillRow.appendChild(btn);
      });
      renderSkillDetail();
      secSkills.appendChild(skillRow);
      secSkills.appendChild(detailBox);
      bodyEl.appendChild(secSkills);

      const secAuto = document.createElement('div');
      secAuto.innerHTML = `<div class="rim-sec-label">駐世行事模式與行事取向（暫離時依此自主活動）</div>`;
      const autoBtn = document.createElement('button');
      autoBtn.type = 'button';
      autoBtn.className = d.autopilot ? 'rim-chip-btn rim-chip-warn' : 'rim-chip-btn';
      autoBtn.textContent = d.autopilot
        ? '目前為自主行事（點擊收回親手操控）'
        : '切換為自主行事（暫離時自動巡走採作）';
      autoBtn.onclick = async () => {
        ++meMoveSeq;
        meMoveLockUntil = performance.now() + 350;
        meArriveHoldUntil = 0;
        await sendCmd({ cmd: 'toggle_autopilot' });
        lastModalSig = '';
        await pollState(true);
      };
      secAuto.appendChild(autoBtn);

      const dirGrid = document.createElement('div');
      dirGrid.className = 'rim-dir-grid';
      const activeDir = d.directive || 'forage';
      const dirOptions = [
        { code: 'forage', title: '林緣巡採', desc: '掘筍伐竹、隨飢補糧' },
        { code: 'craft', title: '百工營造', desc: '打製器具、拾材營建' },
        { code: 'guard', title: '護衛獵巡', desc: '巡哨警戒、護佑同袍' },
        { code: 'rest', title: '爐畔靜養', desc: '守火養息、調理創傷' },
      ];
      for (const opt of dirOptions) {
        const db = document.createElement('button');
        db.type = 'button';
        db.className = opt.code === activeDir ? 'rim-dir-btn is-active' : 'rim-dir-btn';
        db.dataset.directive = opt.code;
        db.innerHTML = `<strong>${opt.title}</strong><small>${opt.desc}</small>`;
        db.onclick = async () => {
          await sendCmd({ cmd: 'directive', directive: opt.code });
          lastModalSig = '';
          await pollState(true);
        };
        dirGrid.appendChild(db);
      }
      secAuto.appendChild(dirGrid);
      bodyEl.appendChild(secAuto);

      const secDiary = document.createElement('div');
      secDiary.innerHTML = `<div class="rim-sec-label">近期駐世經歷</div>`;
      const diaryList = document.createElement('div');
      diaryList.className = 'rim-diary-list';
      const diaryEntries = Array.isArray(d.recent_diary) ? d.recent_diary : [];
      if (!diaryEntries.length) {
        const empty = document.createElement('div');
        empty.className = 'rim-diary-item';
        empty.textContent = '初抵山林，尚無遠行記事';
        diaryList.appendChild(empty);
      } else {
        for (const entry of diaryEntries) {
          const item = document.createElement('div');
          item.className = 'rim-diary-item';
          item.textContent = entry;
          diaryList.appendChild(item);
        }
      }
      secDiary.appendChild(diaryList);
      bodyEl.appendChild(secDiary);

      const secPref = document.createElement('div');
      secPref.innerHTML = `<div class="rim-sec-label">帳號與偏好</div>`;
      const prefBtn = document.createElement('button');
      prefBtn.type = 'button';
      prefBtn.className = 'rim-chip-btn';
      prefBtn.textContent = '開啟偏好與金鑰設定';
      prefBtn.onclick = () => openModal('settings');
      secPref.appendChild(prefBtn);
      bodyEl.appendChild(secPref);
    } else if (activeModal === 'settings') {
      titleEl.textContent = '帳號與偏好設定';

      let hasKeyConfigured = false;
      const MASK_VAL = '••••••••••••';

      const secByok = document.createElement('div');
      secByok.className = 'rim-byok-box';
      const byokLbl = document.createElement('div');
      byokLbl.className = 'rim-sec-label';
      byokLbl.textContent = '進階對話金鑰（選填）';
      const byokMeta = document.createElement('div');
      byokMeta.style.cssText = 'font-size:11.5px;color:#6b5a47;line-height:1.4;';
      byokMeta.textContent = '供深度街坊交談選用；留空即使用本機常駐對話模型';
      const byokState = document.createElement('div');
      byokState.id = 'rim-byok-status';
      byokState.style.cssText = 'font-size:11.5px;color:#2a4a6b;font-weight:600;';
      byokState.textContent = '查詢金鑰狀態中…';
      const byokInp = document.createElement('input');
      byokInp.type = 'password';
      byokInp.id = 'rim-byok-input';
      byokInp.className = 'rim-byok-input';
      byokInp.placeholder = '貼上個人 OpenRouter API Key';
      byokInp.autocomplete = 'new-password';
      byokInp.spellcheck = false;
      byokInp.addEventListener('focus', () => {
        if (byokInp.value === MASK_VAL) {
          byokInp.value = '';
        }
      });
      byokInp.addEventListener('blur', () => {
        if (!byokInp.value.trim() && hasKeyConfigured) {
          byokInp.value = MASK_VAL;
        }
      });

      const byokBtns = document.createElement('div');
      byokBtns.className = 'rim-row-pills';
      const btnSaveKey = document.createElement('button');
      btnSaveKey.type = 'button';
      btnSaveKey.className = 'rim-socket-btn';
      btnSaveKey.textContent = '儲存金鑰';
      const btnClearKey = document.createElement('button');
      btnClearKey.type = 'button';
      btnClearKey.className = 'rim-chip-btn';
      btnClearKey.style.padding = '5px 10px';
      btnClearKey.textContent = '清除金鑰';

      const syncByokLabel = (info) => {
        if (!info || !info.ok) {
          byokState.textContent = '目前狀態：未設定個人金鑰';
          return;
        }
        hasKeyConfigured = Boolean(info.has_key);
        byokState.textContent = hasKeyConfigured
          ? '目前狀態：已設定個人金鑰'
          : '目前狀態：未設定個人金鑰（使用本機對話）';
        if (document.activeElement !== byokInp) {
          byokInp.value = hasKeyConfigured ? MASK_VAL : '';
        }
      };
      fetch('/api/rim/byok', { credentials: 'same-origin' })
        .then((r) => r.json())
        .then(syncByokLabel)
        .catch(() => { byokState.textContent = '目前狀態：未設定個人金鑰'; });

      const saveKeyValue = async (val) => {
        try {
          const res = await fetch('/api/rim/byok', {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ api_key: val }),
          });
          const data = await res.json();
          hasKeyConfigured = Boolean(data && data.has_key);
          if (byokInp) {
            byokInp.blur();
            byokInp.value = hasKeyConfigured ? MASK_VAL : '';
          }
          syncByokLabel(data);
          if (data.msg) showToast(data.msg);
        } catch (_) {
          showToast('儲存金鑰失敗');
        }
      };
      btnSaveKey.onclick = () => {
        const v = byokInp.value.trim();
        if (!v || /^[•*·]+$/.test(v)) {
          showToast('請貼上新的金鑰內容');
          return;
        }
        saveKeyValue(v);
      };
      btnClearKey.onclick = () => saveKeyValue('');

      byokBtns.append(btnSaveKey, btnClearKey);
      secByok.append(byokLbl, byokMeta, byokState, byokInp, byokBtns);
      bodyEl.appendChild(secByok);

      const secAcct = document.createElement('div');
      secAcct.innerHTML = `<div class="rim-sec-label">角色與登入</div>`;
      const acctRow = document.createElement('div');
      acctRow.className = 'rim-row-pills';
      const backBtn = document.createElement('button');
      backBtn.type = 'button';
      backBtn.className = 'rim-chip-btn';
      backBtn.textContent = '返回自身狀態';
      backBtn.onclick = () => openModal('self');
      const outBtn = document.createElement('button');
      outBtn.type = 'button';
      outBtn.className = 'rim-chip-btn';
      outBtn.textContent = '登出／切換角色';
      outBtn.onclick = async () => {
        try {
          await fetch('/api/rim/logout', { method: 'POST', credentials: 'same-origin' });
        } catch (_) {}
        enterAuthScreen('');
      };
      acctRow.append(backBtn, outBtn);
      secAcct.appendChild(acctRow);
      bodyEl.appendChild(secAcct);
    } else if (activeModal === 'talk') {
      if (!talkState || !talkState.npc) {
        titleEl.textContent = '當面交談';
        const waitEl = document.createElement('div');
        waitEl.className = 'rim-talk-bubble';
        waitEl.textContent = '走近寒暄中…';
        bodyEl.appendChild(waitEl);
        return;
      }
      const npc = talkState.npc;
      titleEl.textContent = `${npc.name}・當面交談`;

      const head = document.createElement('div');
      head.className = 'rim-modal-hero';
      const pBox = document.createElement('div');
      pBox.className = 'rim-modal-portrait-box';
      const cv = document.createElement('canvas');
      pBox.appendChild(cv);
      head.appendChild(pBox);
      if (window.SwPortrait) {
        window.SwPortrait.render(cv, {
          mode: 'bust',
          width: 92,
          height: 108,
          id: npc.id,
          name: npc.name,
          glyph: npc.glyph,
          gender: npc.gender,
          role: npc.role || '',
          soul_seed: npc.soul_seed,
          wearHat: npc.wear_hat !== false,
          showPack: Boolean(npc.show_pack),
          tool: npc.tool || null,
          hpRatio: typeof npc.hp_ratio === 'number' ? npc.hp_ratio : 1.0,
        });
      }
      const metaCol = document.createElement('div');
      metaCol.className = 'rim-modal-hero-info';
      const tagRow = document.createElement('div');
      tagRow.className = 'rim-row-pills';
      for (const t of npc.tags || []) {
        const sp = document.createElement('span');
        sp.className = 'rim-tag';
        sp.textContent = t;
        tagRow.appendChild(sp);
      }
      metaCol.appendChild(tagRow);
      if (talkState.meet_note) {
        const mn = document.createElement('div');
        mn.className = 'rim-meet-note';
        mn.textContent = talkState.meet_note;
        metaCol.appendChild(mn);
      }
      head.appendChild(metaCol);
      bodyEl.appendChild(head);

      const bubble = document.createElement('div');
      bubble.className = 'rim-talk-bubble';
      bubble.textContent = talkBusy ? '對方思索回應中…' : `「${talkState.reply || '山風正好，請說。'}」`;
      bodyEl.appendChild(bubble);

      const qRow = document.createElement('div');
      qRow.className = 'rim-row-pills';
      for (const [label, topicKey] of [
        ['寒暄問候', 'greet'],
        ['打聽風聞與去處', 'rumor'],
        ['詢問物資所需', 'needs'],
      ]) {
        const qb = document.createElement('button');
        qb.type = 'button';
        qb.className = 'rim-chip-btn';
        qb.disabled = talkBusy;
        qb.textContent = label;
        qb.onclick = () => sendTalkRequest(npc.id, topicKey, '');
        qRow.appendChild(qb);
      }
      bodyEl.appendChild(qRow);

      const form = document.createElement('form');
      form.className = 'rim-talk-form';
      const talkInp = document.createElement('input');
      talkInp.type = 'text';
      talkInp.id = 'rim-talk-input';
      talkInp.className = 'rim-talk-input';
      talkInp.placeholder = `對${npc.name}說句話…`;
      talkInp.maxLength = 80;
      const sendBtn = document.createElement('button');
      sendBtn.type = 'submit';
      sendBtn.className = 'rim-socket-btn';
      sendBtn.disabled = talkBusy;
      sendBtn.textContent = '開口';
      form.append(talkInp, sendBtn);
      form.onsubmit = (e) => {
        e.preventDefault();
        const txt = talkInp.value.trim();
        if (!txt || talkBusy) return;
        talkInp.value = '';
        sendTalkRequest(npc.id, 'free', txt);
      };
      bodyEl.appendChild(form);

      const secTrade = document.createElement('div');
      secTrade.innerHTML = `<div class="rim-sec-label">當面互通有無（隨身：${talkState.player_magnesium_tag || '鎂晶數枚'}）</div>`;
      const tradeCol = document.createElement('div');
      tradeCol.style.cssText = 'display:flex;flex-direction:column;gap:6px;';
      const offers = Array.isArray(talkState.npc_offers) ? talkState.npc_offers : [];
      const sellable = Array.isArray(talkState.player_sellable) ? talkState.player_sellable : [];
      const doTrade = async (itemCode, mode) => {
        try {
          const r = await fetch('/api/rim/trade', {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ target_actor: npc.id, item_code: itemCode, mode }),
          });
          const td = await r.json();
          if (td && td.ok) {
            talkState = { ...talkState, ...td };
            lastModalSig = '';
            renderModal(true);
            pollState(true);
            if (td.msg) showToast(td.msg);
          } else if (td && td.msg) {
            showToast(td.msg);
          }
        } catch (_) {
          showToast('當面交換未能成行');
        }
      };
      for (const it of offers) {
        const row = document.createElement('div');
        row.className = 'rim-inv-row';
        const mainWrap = document.createElement('span');
        mainWrap.className = 'rim-inv-main';
        if (window.CddaTiles) {
          mainWrap.appendChild(window.CddaTiles.createItemIconCanvas(it.code, 22));
        }
        const lbl = document.createElement('span');
        const mtag = it.market_tag ? `（${it.market_tag}）` : '';
        lbl.textContent = `對方現貨：${it.label} × ${it.count}${mtag}`;
        mainWrap.appendChild(lbl);
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'rim-socket-btn';
        btn.disabled = !it.can_afford;
        btn.textContent = `換取（${it.price_mg} 鎂）`;
        btn.onclick = () => doTrade(it.code, 'buy');
        row.append(mainWrap, btn);
        tradeCol.appendChild(row);
      }
      for (const it of sellable) {
        const row = document.createElement('div');
        row.className = 'rim-inv-row';
        const mainWrap = document.createElement('span');
        mainWrap.className = 'rim-inv-main';
        if (window.CddaTiles) {
          mainWrap.appendChild(window.CddaTiles.createItemIconCanvas(it.code, 22));
        }
        const lbl = document.createElement('span');
        const mtag = it.market_tag ? `（${it.market_tag}）` : '';
        lbl.textContent = `我方行囊：${it.label} × ${it.count}${mtag}`;
        mainWrap.appendChild(lbl);
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'rim-chip-btn';
        btn.style.cssText = 'padding:4px 9px;font-size:12px;';
        btn.disabled = !it.npc_can_pay;
        btn.textContent = it.price_mg > 0 ? `讓出（得 ${it.price_mg} 鎂）` : '倉滿暫停收購';
        btn.onclick = () => doTrade(it.code, 'sell');
        row.append(mainWrap, btn);
        tradeCol.appendChild(row);
      }
      if (offers.length === 0 && sellable.length === 0) {
        const emp = document.createElement('div');
        emp.className = 'rim-sec-label';
        emp.textContent = '雙方手邊暫無可交換物資';
        tradeCol.appendChild(emp);
      }
      secTrade.appendChild(tradeCol);
      bodyEl.appendChild(secTrade);
    } else if (activeModal === 'gear') {
      titleEl.textContent = '裝備／隨身行囊';

      const secHeld = document.createElement('div');
      secHeld.innerHTML = `<div class="rim-sec-label">當前隨身穿戴</div>`;
      const heldRow = document.createElement('div');
      heldRow.className = 'rim-inv-row';
      const heldMain = document.createElement('span');
      heldMain.className = 'rim-inv-main';
      if (d.equipped_code && window.CddaTiles) {
        heldMain.appendChild(window.CddaTiles.createItemIconCanvas(d.equipped_code, 24));
      }
      const heldName = document.createElement('strong');
      const heldBase = `手持：${d.equipped_label || '空手'}`;
      heldName.textContent = d.tool_note ? `${heldBase}（${d.tool_note}）` : heldBase;
      heldMain.appendChild(heldName);
      heldRow.appendChild(heldMain);

      if (d.equipped_code) {
        const unequipBtn = document.createElement('button');
        unequipBtn.type = 'button';
        unequipBtn.className = 'rim-socket-btn';
        unequipBtn.textContent = '收起（改為空手）';
        unequipBtn.onclick = async () => {
          await sendCmd({ cmd: 'equip', item_code: 'none' });
          lastModalSig = '';
          await pollState(true);
        };
        heldRow.appendChild(unequipBtn);
      }
      secHeld.appendChild(heldRow);

      for (const [partLbl, worn, code, wornName, bareName] of [
        ['頭戴', d.equipped_hat, 'hat', '細編竹笠', '未戴笠'],
        ['肩背', d.equipped_pack, 'pack', '竹編背簍', '輕裝無簍'],
      ]) {
        const wRow = document.createElement('div');
        wRow.className = 'rim-wear-line';
        const wSpan = document.createElement('span');
        wSpan.textContent = `${partLbl}：${worn ? wornName : bareName}`;
        wRow.appendChild(wSpan);
        if (worn) {
          const rmBtn = document.createElement('button');
          rmBtn.type = 'button';
          rmBtn.className = 'rim-chip-btn';
          rmBtn.style.cssText = 'padding:3px 9px;font-size:12px;';
          rmBtn.textContent = '卸下';
          rmBtn.onclick = async () => {
            const r = await sendCmd({ cmd: 'toggle_wear', item_code: code });
            if (r && r.msg) showToast(r.msg);
            lastModalSig = '';
            await pollState(true);
          };
          wRow.appendChild(rmBtn);
        }
        secHeld.appendChild(wRow);
      }

      const { hero } = buildModalHero(d, secHeld);
      bodyEl.appendChild(hero);

      const secBag = document.createElement('div');
      secBag.innerHTML = `<div class="rim-sec-label">隨身行囊物品（${d.magnesium_tag || '鎂晶數枚'}）</div>`;
      const invList = Array.isArray(d.inventory) ? d.inventory : [];
      if (invList.length === 0) {
        const empty = document.createElement('div');
        empty.className = 'rim-sec-label';
        empty.textContent = '行囊暫無物品（點擊地面散落物或採伐即可收入行囊）';
        secBag.appendChild(empty);
      } else {
        const col = document.createElement('div');
        col.style.display = 'flex';
        col.style.flexDirection = 'column';
        col.style.gap = '6px';
        for (const item of invList) {
          const row = document.createElement('div');
          row.className = 'rim-inv-row';
          const mainWrap = document.createElement('span');
          mainWrap.className = 'rim-inv-main';
          if (window.CddaTiles) {
            mainWrap.appendChild(window.CddaTiles.createItemIconCanvas(item.code, 24));
          }
          const lbl = document.createElement('span');
          lbl.textContent = `${item.label} × ${item.count}`;
          mainWrap.appendChild(lbl);
          row.appendChild(mainWrap);

          const btns = document.createElement('div');
          btns.className = 'rim-row-pills';
          if (item.is_tool) {
            const eqBtn = document.createElement('button');
            eqBtn.type = 'button';
            eqBtn.className = 'rim-socket-btn';
            eqBtn.textContent = '握持';
            eqBtn.onclick = async () => {
              await sendCmd({ cmd: 'equip', item_code: item.code });
              lastModalSig = '';
              await pollState(true);
            };
            btns.appendChild(eqBtn);
          }
          if (item.is_wearable) {
            const wrBtn = document.createElement('button');
            wrBtn.type = 'button';
            wrBtn.className = 'rim-socket-btn';
            wrBtn.textContent = '穿戴';
            wrBtn.onclick = async () => {
              const r = await sendCmd({ cmd: 'toggle_wear', item_code: item.code });
              if (r && r.msg) showToast(r.msg);
              lastModalSig = '';
              await pollState(true);
            };
            btns.appendChild(wrBtn);
          }
          if (item.is_edible) {
            const eatBtn = document.createElement('button');
            eatBtn.type = 'button';
            eatBtn.className = 'rim-socket-btn';
            eatBtn.textContent = '食用';
            eatBtn.onclick = async () => {
              const r = await sendCmd({ cmd: 'eat', item_code: item.code });
              if (r && r.msg) showToast(r.msg);
              lastModalSig = '';
              await pollState(true);
            };
            btns.appendChild(eatBtn);
          }
          if (item.is_medical) {
            const healBtn = document.createElement('button');
            healBtn.type = 'button';
            healBtn.className = 'rim-socket-btn';
            healBtn.textContent = '敷貼治傷';
            healBtn.onclick = async () => {
              const r = await sendCmd({ cmd: 'apply_salve', item_code: item.code });
              if (r && r.msg) showToast(r.msg);
              lastModalSig = '';
              await pollState(true);
            };
            btns.appendChild(healBtn);
          }
          const dropBtn = document.createElement('button');
          dropBtn.type = 'button';
          dropBtn.className = 'rim-chip-btn';
          dropBtn.style.padding = '4px 8px';
          dropBtn.style.fontSize = '12px';
          dropBtn.textContent = '擱置地上';
          dropBtn.onclick = async () => {
            await sendCmd({ cmd: 'drop', item_code: item.code });
            lastModalSig = '';
            await pollState(true);
          };
          btns.appendChild(dropBtn);

          row.appendChild(btns);
          col.appendChild(row);
        }
        secBag.appendChild(col);
      }
      bodyEl.appendChild(secBag);

      const hint = document.createElement('div');
      hint.className = 'rim-sec-label';
      hint.textContent = '單擊空地行走、雙擊疾奔；長按或右鍵地面與溪流可就地營造。';
      bodyEl.appendChild(hint);
    }
  }

  function hasWallAt(mx, my) {
    const c = cellMap.get(`${mx},${my}`);
    return c && (c.s === 'wall' || c.s === 'door');
  }

  function hasBridgeAt(mx, my) {
    const c = cellMap.get(`${mx},${my}`);
    return Boolean(c && c.s === 'bridge');
  }

  function isClientCellWalkable(mx, my, isBeast) {
    const c = cellMap.get(`${mx},${my}`);
    if (!c) return true;
    if (c.s === 'wall' || c.s === 'workbench' || c.s === 'campfire' || c.s === 'lamp') return false;
    if (c.s === 'door') return !isBeast;
    if (c.s === 'bridge') return true;
    if (c.g === 'stream' || c.g === 'spring' || c.g === 'deep' || c.g === 'cliff') return false;
    if (c.p === 'bamboo') return false;
    return true;
  }

  function isClientCircleWalkable(px, py, rEnv, isBeast) {
    const pts = [[0, 0], [rEnv, 0], [-rEnv, 0], [0, rEnv], [0, -rEnv]];
    for (let i = 0; i < pts.length; i++) {
      if (!isClientCellWalkable(Math.round(px + pts[i][0]), Math.round(py + pts[i][1]), isBeast)) {
        return false;
      }
    }
    return true;
  }

  function canClientOccupy(oldX, oldY, newX, newY, isBeast) {
    const ox = Math.round(oldX), oy = Math.round(oldY);
    const nx = Math.round(newX), ny = Math.round(newY);
    if (!isClientCellWalkable(nx, ny, isBeast)) return false;
    if (ox !== nx && oy !== ny) {
      if (!isClientCellWalkable(nx, oy, isBeast) || !isClientCellWalkable(ox, ny, isBeast)) {
        return false;
      }
    }
    const R = 0.15;
    if (isClientCircleWalkable(newX, newY, R, isBeast)) return true;
    if (!isClientCircleWalkable(oldX, oldY, R, isBeast)) {
      return Math.hypot(newX - nx, newY - ny) < Math.hypot(oldX - ox, oldY - oy);
    }
    return false;
  }

  function slideClientMove(px, py, dx, dy, isBeast) {
    if (Math.abs(dx) < 1e-6 && Math.abs(dy) < 1e-6) return [px, py];
    if (canClientOccupy(px, py, px + dx, py + dy, isBeast)) return [px + dx, py + dy];
    if (Math.abs(dx) > 1e-5 && canClientOccupy(px, py, px + dx, py, isBeast)) return [px + dx, py];
    if (Math.abs(dy) > 1e-5 && canClientOccupy(px, py, px, py + dy, isBeast)) return [px, py + dy];
    return [px, py];
  }

  function trimPassedWaypoints(rawPath, curX, curY) {
    if (!Array.isArray(rawPath) || rawPath.length === 0) return [];
    const pts = rawPath.map((p) => [p[0], p[1]]);
    let bestIdx = 0;
    let bestD = Infinity;
    for (let i = 0; i < pts.length; i++) {
      const d = Math.hypot(pts[i][0] - curX, pts[i][1] - curY);
      if (d < bestD) { bestD = d; bestIdx = i; }
    }
    const res = pts.slice(bestIdx);
    if (res.length >= 2) {
      const vx = res[1][0] - res[0][0];
      const vy = res[1][1] - res[0][1];
      if ((curX - res[0][0]) * vx + (curY - res[0][1]) * vy > 0 &&
          Math.hypot(res[0][0] - curX, res[0][1] - curY) < 0.95) {
        res.shift();
      }
    }
    if (res.length >= 1 && Math.hypot(res[0][0] - curX, res[0][1] - curY) < 0.12) {
      res.shift();
    }
    return res;
  }

  function findClientPath(sx, sy, gx, gy) {
    if (sx === gx && sy === gy) return [];
    const goalWalk = isClientCellWalkable(gx, gy, false);
    const dirs = [
      [0, 1, 10, false], [1, 0, 10, false], [0, -1, 10, false], [-1, 0, 10, false],
      [1, 1, 14, true], [1, -1, 14, true], [-1, -1, 14, true], [-1, 1, 14, true],
    ];
    const hFn = (x, y) => {
      const ax = Math.abs(x - gx), ay = Math.abs(y - gy);
      return Math.min(ax, ay) * 14 + (Math.max(ax, ay) - Math.min(ax, ay)) * 10;
    };
    const open = [[hFn(sx, sy), 0, sx, sy]];
    const bestG = new Map([[`${sx},${sy}`, 0]]);
    const came = new Map();
    let expanded = 0;
    while (open.length > 0 && expanded < 650) {
      let bi = 0;
      for (let i = 1; i < open.length; i++) if (open[i][0] < open[bi][0]) bi = i;
      const cur = open.splice(bi, 1)[0];
      const [, g, cx, cy] = cur;
      if (g > (bestG.get(`${cx},${cy}`) ?? Infinity)) continue;
      if ((cx === gx && cy === gy) || (!goalWalk && Math.abs(cx - gx) <= 1 && Math.abs(cy - gy) <= 1 && (cx === gx || cy === gy))) {
        const out = [];
        let kx = cx, ky = cy;
        while (kx !== sx || ky !== sy) {
          out.push([kx, ky]);
          const prev = came.get(`${kx},${ky}`);
          if (!prev) return null;
          kx = prev[0]; ky = prev[1];
        }
        out.reverse();
        return out;
      }
      expanded++;
      for (let i = 0; i < dirs.length; i++) {
        const [dx, dy, cost, isDiag] = dirs[i];
        const nx = cx + dx, ny = cy + dy;
        if (!isClientCellWalkable(nx, ny, false)) continue;
        if (isDiag && (!isClientCellWalkable(cx + dx, cy, false) || !isClientCellWalkable(cx, cy + dy, false))) continue;
        const ng = g + cost;
        const nKey = `${nx},${ny}`;
        if (ng < (bestG.get(nKey) ?? Infinity)) {
          bestG.set(nKey, ng);
          came.set(nKey, [cx, cy]);
          open.push([ng + hFn(nx, ny), ng, nx, ny]);
        }
      }
    }
    return null;
  }

  function isActorNearCell(mx, my, maxDist) {
    for (const a of actors) {
      const st = renderActors.get(a.id);
      const ax = st ? st.x : a.px;
      const ay = st ? st.y : a.py;
      if (Math.hypot(ax - mx, ay - my) <= maxDist) return true;
    }
    return false;
  }

  function drawMacroBasemap() {
    if (!meta || !basemapImg) return;
    // 1 個 20m 母格 = 20 個 1m 細格；母格 (gx, gy) 涵蓋細格 [gx*20 .. gx*20+19]
    const macroPx = cellPx * 20.0;
    const left = viewportW / 2 + (meta.min_gx * 20.0 - 0.5 - camX) * cellPx;
    const top = viewportH / 2 - ((meta.max_gy + 1) * 20.0 - 0.5 - camY) * cellPx;
    ctx.imageSmoothingEnabled = macroPx < 48;
    ctx.drawImage(basemapImg, left, top, meta.width * macroPx, meta.height * macroPx);

    // 鳥瞰尺度標繪地標名（自動避開過近重疊與中央主角棋子）
    if (cellPx < 1.8 && Array.isArray(meta.landmarks)) {
      ctx.font = 'bold 12px "LXGW WenKai Mono TC", serif';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'middle';
      const drawnPts = [];
      for (const lm of meta.landmarks) {
        const p = microToScreen(lm.gx * 20 + 10, lm.gy * 20 + 10);
        if (p.x < -80 || p.x > viewportW + 80 || p.y < -30 || p.y > viewportH + 30) continue;
        if (drawnPts.some((q) => Math.hypot(q.x - p.x, q.y - p.y) < 36)) continue;
        drawnPts.push(p);
        ctx.fillStyle = '#8b2020';
        ctx.beginPath();
        ctx.arc(p.x, p.y, 3.5, 0, Math.PI * 2);
        ctx.fill();
        const labelY = Math.hypot(p.x - viewportW / 2, p.y - viewportH / 2) < 28 ? p.y + 22 : p.y;
        ctx.lineWidth = 3;
        ctx.strokeStyle = 'rgba(247, 240, 227, 0.92)';
        ctx.strokeText(lm.name, p.x + 8, labelY);
        ctx.fillStyle = '#2c1e0f';
        ctx.fillText(lm.name, p.x + 8, labelY);
      }
    }
  }

  function drawMicroLayer(nowMs) {
    if (cellPx < 5.0) return;
    const alpha = Math.min(1.0, (cellPx - 5.0) / 5.0);
    const tiles = window.CddaTiles;
    const ready = Boolean(tiles && tiles.isReady());

    ctx.save();
    ctx.globalAlpha = alpha;

    const halfW = Math.min(40, Math.ceil(viewportW / cellPx / 2) + 2);
    const halfH = Math.min(40, Math.ceil(viewportH / cellPx / 2) + 2);
    const cx = Math.round(camX);
    const cy = Math.round(camY);

    const tl = microToScreen(cx - halfW - 0.5, cy + halfH + 0.5);
    const spanW = (halfW * 2 + 1) * cellPx;
    const spanH = (halfH * 2 + 1) * cellPx;
    ctx.fillStyle = ready ? '#2f3b28' : '#efe8d8';
    ctx.fillRect(tl.x, tl.y, spanW, spanH);

    bedBadgeQueue = [];

    // Pass 1A：純地形地塊（整數邊界消除縮放毛細縫）
    for (let my = cy + halfH; my >= cy - halfH; my--) {
      for (let mx = cx - halfW; mx <= cx + halfW; mx++) {
        const e = tileEdges(mx, my);
        if (e.x1 < 0 || e.x0 > viewportW || e.y1 < 0 || e.y0 > viewportH) continue;
        const c = cellMap.get(`${mx},${my}`);
        const g = c ? c.g : 'grass';
        if (ready) {
          tiles.drawGroundTile(ctx, g, mx, my, e.x0, e.y0, e.x1, e.y1, cellMap, hash2d);
        } else {
          ctx.fillStyle = isWaterGround(g) ? '#b8d0d8' : (g === 'sand' ? '#dfd2b8' : '#dfe6d2');
          ctx.fillRect(e.x0, e.y0, e.w, e.h);
        }
      }
    }

    // Pass 1B：單平面室內鋪地（住居塌塌米／工坊店鋪木地板＋北／西牆腳微陰影）
    const effWeather = getEffectiveWeather();
    for (const rm of worldRooms) {
      if (!Array.isArray(rm.cells) || !rm.cells.length) continue;
      const isHomeZone = rm.cells.some((pt) => cellMap.get(`${pt[0]},${pt[1]}`)?.s === 'bed');
      for (const pt of rm.cells) {
        const ix = pt[0];
        const iy = pt[1];
        const e = tileEdges(ix, iy);
        if (e.x1 < 0 || e.x0 > viewportW || e.y1 < 0 || e.y0 > viewportH) continue;
        if (ready) {
          tiles.drawInteriorFloor(ctx, isHomeZone, ix, e.x0, e.y0, e.x1, e.y1);
        } else {
          ctx.fillStyle = '#d8c8a8';
          ctx.fillRect(e.x0, e.y0, e.w, e.h);
        }
        if (iy === rm.y1 - 1) {
          ctx.fillStyle = 'rgba(20, 14, 10, 0.14)';
          ctx.fillRect(e.x0, e.y0, e.w, Math.max(2, e.h * 0.22));
        }
        if (ix === rm.x0 + 1) {
          ctx.fillStyle = 'rgba(20, 14, 10, 0.10)';
          ctx.fillRect(e.x0, e.y0, Math.max(2, e.w * 0.18), e.h);
        }
      }
    }

    // Pass 1C：石磚街巷、騎樓地坪、木橋、田壟與門檻鋪面
    for (let my = cy + halfH; my >= cy - halfH; my--) {
      for (let mx = cx - halfW; mx <= cx + halfW; mx++) {
        const c = cellMap.get(`${mx},${my}`);
        if (!c || !c.s) continue;
        const e = tileEdges(mx, my);
        if (e.x1 < 0 || e.x0 > viewportW || e.y1 < 0 || e.y0 > viewportH) continue;
        if (
          c.s === 'paving' || c.s === 'lamp' || c.s === 'arcade' || c.s === 'bridge' ||
          c.s === 'ridge' || c.s === 'herb_seed' || c.s === 'herb_plot' ||
          c.s === 'crop_seed' || c.s === 'crop_plot'
        ) {
          if (ready) {
            tiles.drawFloorOrRoad(ctx, c.s, mx, my, e.x0, e.y0, e.x1, e.y1, cellMap);
          }
        } else if (c.s === 'door') {
          if (ready) {
            tiles.drawFillTile(ctx, 'ground_floor', e.x0, e.y0, e.x1, e.y1);
          }
        }
      }
    }

    // Pass 1D：營火地面暖暈與散落物資
    for (let my = cy + halfH; my >= cy - halfH; my--) {
      for (let mx = cx - halfW; mx <= cx + halfW; mx++) {
        const c = cellMap.get(`${mx},${my}`);
        if (!c) continue;
        if (c.s === 'campfire') {
          const p = microToScreen(mx, my);
          const flicker = 1 + Math.sin(nowMs * 0.012) * 0.09;
          const wetCamp = effWeather.raining && !indoorCellSet.has(`${mx},${my}`);
          const rad = cellPx * (wetCamp ? 0.92 : 1.75) * flicker;
          const grad = ctx.createRadialGradient(p.x, p.y, 2, p.x, p.y, rad);
          grad.addColorStop(0, wetCamp ? 'rgba(208, 102, 34, 0.16)' : 'rgba(232, 132, 42, 0.28)');
          grad.addColorStop(1, 'rgba(232, 132, 42, 0)');
          ctx.fillStyle = grad;
          ctx.beginPath();
          ctx.arc(p.x, p.y, rad, 0, Math.PI * 2);
          ctx.fill();
        }
        if (c.i && c.s !== 'stockpile' && cellPx >= 13 && ready) {
          const e = tileEdges(mx, my);
          tiles.drawItemSprite(ctx, c.i, (e.x0 + e.x1) * 0.5, (e.y0 + e.y1) * 0.5, e.w);
        }
      }
    }

    // 接觸插座焦點柔圓標記
    if (selectedCell) {
      const sp = microToScreen(selectedCell.mx, selectedCell.my);
      ctx.strokeStyle = '#d9483b';
      ctx.lineWidth = 2.0;
      ctx.setLineDash([5, 3]);
      ctx.beginPath();
      ctx.arc(sp.x, sp.y, Math.max(8, cellPx * 0.52), 0, Math.PI * 2);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    ctx.restore();
  }

  function lerpAngle(cur, target, t) {
    let diff = ((target - cur + Math.PI) % (Math.PI * 2)) - Math.PI;
    if (diff < -Math.PI) diff += Math.PI * 2;
    return cur + diff * t;
  }

  function isActorWorking(a, isMoving) {
    return (
      !isMoving &&
      typeof a.act === 'string' &&
      !a.act.includes('完') &&
      !a.act.includes('待命') &&
      WORKING_ACT_RE.test(a.act)
    );
  }

  function findNearbyWorkTarget(x, y) {
    const cx = Math.round(x);
    const cy = Math.round(y);
    let bestBp = null;
    let bestDist = 99;
    let bestWork = null;
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        if (dx === 0 && dy === 0) continue;
        const nx = cx + dx;
        const ny = cy + dy;
        const c = cellMap.get(`${nx},${ny}`);
        if (!c) continue;
        const d = Math.hypot(nx - x, ny - y);
        if (c.b && d < bestDist) {
          bestDist = d;
          bestBp = { mx: nx, my: ny };
        } else if ((c.p === 'bamboo' || c.p === 'herb' || c.s === 'workbench' || c.s === 'ridge' || c.s === 'herb_plot' || c.s === 'crop_plot') && !bestWork) {
          bestWork = { mx: nx, my: ny };
        }
      }
    }
    return bestBp || bestWork;
  }

  function spawnLeafParticles(wx, wy) {
    const baseSeed = Math.round((wx * 31 + wy * 17) * 100);
    for (let i = 0; i < 6; i++) {
      if (leafParticles.length >= 48) leafParticles.shift();
      const ang = ((i * 1.047) + ((hash2d(baseSeed + i, i * 7) % 100) / 100) * 0.65);
      const spd = 1.0 + ((hash2d(i, baseSeed) % 100) / 100) * 1.4;
      leafParticles.push({
        x: wx,
        y: wy,
        vx: Math.cos(ang) * spd,
        vy: Math.sin(ang) * spd - 0.35,
        rot: ang,
        life: 0.62,
      });
    }
  }

  function triggerChopImpactFor(st) {
    const cx = Math.round(st.x);
    const cy = Math.round(st.y);
    const fx = Math.round(st.x + Math.cos(st.facing));
    const fy = Math.round(st.y + Math.sin(st.facing));
    const front = cellMap.get(`${fx},${fy}`);
    if (front && front.p === 'bamboo') {
      bambooShakeMap.set(`${fx},${fy}`, 0.28);
      spawnLeafParticles(fx, fy);
      return;
    }
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        if (dx === 0 && dy === 0) continue;
        const nx = cx + dx;
        const ny = cy + dy;
        const c = cellMap.get(`${nx},${ny}`);
        if (c && c.p === 'bamboo') {
          bambooShakeMap.set(`${nx},${ny}`, 0.28);
          spawnLeafParticles(nx, ny);
          return;
        }
      }
    }
  }

  function stepActorKinematics(dt) {
    const prevPos = new Map();
    for (const a of actors) {
      const st = renderActors.get(a.id);
      if (!st) continue;
      prevPos.set(a.id, [st.x, st.y]);
      const isBeast = a.faction === 'wild_beast';
      const isManualMe = !!a.is_player && !a.autopilot;
      const hadPath = !!(st.path && st.path.length > 0);
      let moveUx = Math.cos(st.targetFacing || 0);
      let moveUy = Math.sin(st.targetFacing || 0);

      if (hadPath) {
        let budget = (st.speed || 1.35) * dt;
        while (budget > 0.0002 && st.path.length > 0) {
          const [nx, ny] = st.path[0];
          const dx = nx - st.x;
          const dy = ny - st.y;
          const dist = Math.hypot(dx, dy);
          let crowded = false;
          for (const other of actors) {
            if (other.id === a.id) continue;
            const ost = renderActors.get(other.id);
            const ox = ost ? ost.x : other.px;
            const oy = ost ? ost.y : other.py;
            if (Math.hypot(ox - nx, oy - ny) < 0.86) {
              crowded = true;
              break;
            }
          }
          if (crowded) {
            if (st.path.length > 1 && dist <= 0.78) {
              st.path.shift();
              continue;
            }
            if (st.path.length === 1 && dist <= 0.98) {
              st.path.shift();
              budget = 0;
              break;
            }
          }
          if (dist > 0.005) {
            st.targetFacing = Math.atan2(dy, dx);
          }
          if (dist <= 0.0001) {
            st.path.shift();
            continue;
          }
          let ux = dx / dist;
          let uy = dy / dist;
          moveUx = ux;
          moveUy = uy;
          let steerThreat = false;
          for (const other of actors) {
            if (other.id === a.id) continue;
            const ost = renderActors.get(other.id);
            const ox = (ost ? ost.x : other.px) - st.x;
            const oy = (ost ? ost.y : other.py) - st.y;
            const fwd = ox * ux + oy * uy;
            const lat = Math.abs(-ox * uy + oy * ux);
            if (fwd > 0.05 && fwd < 1.05 && lat < 0.76) {
              steerThreat = true;
              break;
            }
          }
          if (!crowded && !steerThreat && dist <= budget && canClientOccupy(st.x, st.y, nx, ny, isBeast)) {
            st.x = nx;
            st.y = ny;
            st.path.shift();
            budget -= dist;
            continue;
          }
          if (steerThreat) {
            const sx = ux - uy * 0.32;
            const sy = uy + ux * 0.32;
            const sl = Math.hypot(sx, sy);
            if (sl > 1e-4) {
              ux = sx / sl;
              uy = sy / sl;
            }
          }
          const step = Math.min(dist, budget);
          const [nextX, nextY] = slideClientMove(st.x, st.y, ux * step, uy * step, isBeast);
          st.x = nextX;
          st.y = nextY;
          budget = 0;
        }
        if (a.is_player && st.path.length === 0) {
          meArriveHoldUntil = performance.now() + 700;
        }
        if (isManualMe) {
          if (meCmdInFlight === 0 && performance.now() >= meMoveLockUntil) {
            const ex = st.srvX - st.x;
            const ey = st.srvY - st.y;
            const errD = Math.hypot(ex, ey);
            const along = ex * moveUx + ey * moveUy;
            if (along < 0 && errD < 2.4) {
              const cxErr = ex - along * moveUx;
              const cyErr = ey - along * moveUy;
              const cDrift = Math.min(1.0, 1.2 * dt);
              st.x += cxErr * cDrift;
              st.y += cyErr * cDrift;
            } else {
              const drift = errD >= 2.4 ? Math.min(1.0, 1.8 * dt) : Math.min(1.0, 0.35 * dt);
              st.x += ex * drift;
              st.y += ey * drift;
            }
          }
        } else {
          const errD = Math.hypot(st.srvX - st.x, st.srvY - st.y);
          const drift = errD > 0.85 ? Math.min(1.0, 2.8 * dt) : Math.min(1.0, 0.14 * dt);
          st.x += (st.srvX - st.x) * drift;
          st.y += (st.srvY - st.y) * drift;
        }
      } else if (isManualMe) {
        const errD = Math.hypot(st.srvX - st.x, st.srvY - st.y);
        const holdBehind =
          meCmdInFlight > 0 ||
          performance.now() < meMoveLockUntil ||
          (st.srvMoving && performance.now() < meArriveHoldUntil && errD <= 2.4);
        if (!holdBehind && errD > 0.04) {
          const settle = 1.0 - Math.exp(-8.0 * dt);
          st.x += (st.srvX - st.x) * settle;
          st.y += (st.srvY - st.y) * settle;
        }
      } else {
        const settle = 1.0 - Math.exp(-10.0 * dt);
        st.x += (st.srvX - st.x) * settle;
        st.y += (st.srvY - st.y) * settle;
      }
    }

    // 客戶端同步 PBD 實心零重疊投影（嚴保螢幕繪製零重疊、不突兀穿透）
    const active = actors.filter((a) => renderActors.has(a.id));
    for (let iter = 0; iter < 4; iter++) {
      for (let i = 0; i < active.length; i++) {
        for (let j = i + 1; j < active.length; j++) {
          const ai = active[i];
          const aj = active[j];
          const si = renderActors.get(ai.id);
          const sj = renderActors.get(aj.id);
          const bi = ai.faction === 'wild_beast';
          const bj = aj.faction === 'wild_beast';
          const minSep = (bi || bj) ? 0.90 : 0.82;
          const dx = sj.x - si.x;
          const dy = sj.y - si.y;
          const dist = Math.hypot(dx, dy);
          if (dist >= minSep) continue;
          const [nx, ny] = dist > 0.001
            ? [dx / dist, dy / dist]
            : [Math.cos(i * 7 + j * 13), Math.sin(i * 7 + j * 13)];
          const overlap = minSep - dist + 0.002;
          const invMass = (a, st) => {
            if ((st.running || a.running) && st.path && st.path.length > 0) return 0.15;
            if (a.is_player && st.path && st.path.length > 0) return 0.32;
            if (st.path && st.path.length > 0) return 0.45;
            return 0.78;
          };
          const wiRaw = invMass(ai, si);
          const wjRaw = invMass(aj, sj);
          const wSum = Math.max(0.001, wiRaw + wjRaw);
          const wi = wiRaw / wSum;
          const wj = wjRaw / wSum;
          const [ix, iy] = slideClientMove(
            si.x, si.y, -nx * overlap * wi, -ny * overlap * wi, bi
          );
          si.x = ix;
          si.y = iy;
          const [jx, jy] = slideClientMove(
            sj.x, sj.y, nx * overlap * wj, ny * overlap * wj, bj
          );
          sj.x = jx;
          sj.y = jy;
          const dx2 = sj.x - si.x;
          const dy2 = sj.y - si.y;
          const dist2 = Math.hypot(dx2, dy2);
          if (dist2 < minSep) {
            const [nx2, ny2] = dist2 > 0.001 ? [dx2 / dist2, dy2 / dist2] : [nx, ny];
            const rem2 = minSep - dist2 + 0.001;
            const [fx, fy] = slideClientMove(si.x, si.y, -nx2 * rem2, -ny2 * rem2, bi);
            si.x = fx;
            si.y = fy;
            const dx3 = sj.x - si.x;
            const dy3 = sj.y - si.y;
            const dist3 = Math.hypot(dx3, dy3);
            if (dist3 < minSep) {
              const [nx3, ny3] = dist3 > 0.001 ? [dx3 / dist3, dy3 / dist3] : [nx2, ny2];
              const rem3 = minSep - dist3 + 0.001;
              const [gx, gy] = slideClientMove(sj.x, sj.y, nx3 * rem3, ny3 * rem3, bj);
              sj.x = gx;
              sj.y = gy;
            }
            const dx4 = sj.x - si.x;
            const dy4 = sj.y - si.y;
            if (Math.hypot(dx4, dy4) < minSep && Math.abs(dx4) < minSep) {
              const reqY = Math.sqrt(Math.max(0, minSep * minSep - dx4 * dx4)) + 0.002;
              const needY = Math.max(0, reqY - Math.abs(dy4));
              if (needY > 0) {
                const sy = dy4 >= 0 ? 1 : -1;
                const [ax, ay] = slideClientMove(si.x, si.y, 0, -sy * needY, bi);
                si.x = ax;
                si.y = ay;
                const leftY = Math.max(0, reqY - Math.abs(sj.y - si.y));
                if (leftY > 0) {
                  const [bx, by] = slideClientMove(sj.x, sj.y, 0, sy * leftY, bj);
                  sj.x = bx;
                  sj.y = by;
                }
              }
            }
            const dx5 = sj.x - si.x;
            const dy5 = sj.y - si.y;
            if (Math.hypot(dx5, dy5) < minSep && Math.abs(dy5) < minSep) {
              const reqX = Math.sqrt(Math.max(0, minSep * minSep - dy5 * dy5)) + 0.002;
              const needX = Math.max(0, reqX - Math.abs(dx5));
              if (needX > 0) {
                const sx = dx5 >= 0 ? 1 : -1;
                const [ax, ay] = slideClientMove(si.x, si.y, -sx * needX, 0, bi);
                si.x = ax;
                si.y = ay;
                const leftX = Math.max(0, reqX - Math.abs(sj.x - si.x));
                if (leftX > 0) {
                  const [bx, by] = slideClientMove(sj.x, sj.y, sx * leftX, 0, bj);
                  sj.x = bx;
                  sj.y = by;
                }
              }
            }
          }
        }
      }
    }

    for (const a of actors) {
      const st = renderActors.get(a.id);
      if (!st) continue;
      const [prevX, prevY] = prevPos.get(a.id) || [st.x, st.y];
      const stepDist = Math.hypot(st.x - prevX, st.y - prevY);
      const isMoving = (st.path && st.path.length > 0) || stepDist > 0.04 * dt;
      if (stepDist > 0.0001) {
        st.walkPhase = (st.walkPhase + (stepDist / 1.32) * Math.PI * 2) % (Math.PI * 200);
      }

      const expFactor = 1 - Math.exp(-12 * dt);
      st.moveBlend += ((isMoving ? 1 : 0) - st.moveBlend) * expFactor;
      st.runBlend += (((isMoving && (st.running || a.running)) ? 1 : 0) - st.runBlend) * expFactor;

      if (isActorWorking(a, isMoving) && st.chopTimer <= 0) {
        const tgt = findNearbyWorkTarget(st.x, st.y);
        if (tgt) {
          st.targetFacing = Math.atan2(tgt.my - st.y, tgt.mx - st.x);
        }
        st.chopTimer = CHOP_DURATION;
        st.chopDidImpact = false;
      }

      if (st.chopTimer > 0) {
        st.chopTimer = Math.max(0, st.chopTimer - dt);
        if (!st.chopDidImpact && st.chopTimer <= CHOP_DURATION * 0.50) {
          st.chopDidImpact = true;
          triggerChopImpactFor(st);
        }
      }

      st.facing = lerpAngle(st.facing, st.targetFacing, 1.0 - Math.exp(-14.0 * dt));
    }

    for (const [k, rem] of bambooShakeMap.entries()) {
      const next = rem - dt;
      if (next <= 0) bambooShakeMap.delete(k);
      else bambooShakeMap.set(k, next);
    }

    for (let i = leafParticles.length - 1; i >= 0; i--) {
      const lp = leafParticles[i];
      lp.x += lp.vx * dt;
      lp.y += lp.vy * dt;
      lp.vy -= 1.2 * dt;
      lp.rot += 2.6 * dt;
      lp.life -= dt;
      if (lp.life <= 0) leafParticles.splice(i, 1);
    }

    const me = actors.find((a) => a.is_player);
    if (me && followMe && !dragStart) {
      const meSt = renderActors.get(me.id);
      if (meSt) {
        const camFollow = 1.0 - Math.exp(-8.5 * dt);
        camX += (meSt.x - camX) * camFollow;
        camY += (meSt.y - camY) * camFollow;
      }
    }
  }

  // ═══ CDDA Chibi_Ultica / MSXotto+ 即時圖塊與紙娃娃繪製器官 ═══

  function drawCddaHuman(a, pos, sp, nowMs) {
    const tiles = window.CddaTiles;
    const S = Math.max(20, Math.round(cellPx));
    const mb = pos.moveBlend || 0;
    const showHat = a.is_player
      ? (meDetail ? Boolean(meDetail.equipped_hat) : a.wear_hat !== false)
      : (typeof a.wear_hat === 'boolean' ? a.wear_hat : Number(a.id) % 2 === 1);
    const showPack =
      Boolean(a.carry) ||
      Boolean(a.show_pack) ||
      Boolean(a.is_player && meDetail && meDetail.equipped_pack);
    const tool = a.is_player && meDetail
      ? (meDetail.equipped_code || meDetail.equipped_label || a.tool)
      : a.tool;

    ctx.save();

    if ((pos.running || a.running) && pos.path && pos.path.length > 0 && typeof pos.facing === 'number') {
      for (const k of [1, 2]) {
        const bx = sp.x - Math.cos(pos.facing) * (k * 0.26 * cellPx);
        const by = sp.y + Math.sin(pos.facing) * (k * 0.26 * cellPx);
        ctx.strokeStyle = k === 1 ? 'rgba(139, 32, 32, 0.36)' : 'rgba(139, 32, 32, 0.16)';
        ctx.lineWidth = 1.6;
        ctx.beginPath();
        ctx.ellipse(bx, by + cellPx * 0.24, cellPx * (0.30 - k * 0.04), cellPx * (0.16 - k * 0.02), 0, 0, Math.PI * 2);
        ctx.stroke();
      }
    }

    if (a.is_player) {
      ctx.strokeStyle = 'rgba(196, 44, 44, 0.85)';
      ctx.lineWidth = 2.0;
      ctx.beginPath();
      ctx.ellipse(sp.x, sp.y + cellPx * 0.28, cellPx * 0.34, cellPx * 0.18, 0, 0, Math.PI * 2);
      ctx.stroke();
    }

    const isDowned = a.act === '力竭倒地';
    if (isDowned) {
      ctx.save();
      ctx.setLineDash([4, 3]);
      ctx.strokeStyle = 'rgba(196, 44, 44, 0.88)';
      ctx.lineWidth = 2.0;
      ctx.beginPath();
      ctx.arc(sp.x, sp.y, cellPx * 0.46, 0, Math.PI * 2);
      ctx.stroke();
      ctx.restore();
    } else if (typeof a.hp_ratio === 'number' && a.hp_ratio < 0.99) {
      ctx.strokeStyle = '#c4322c';
      ctx.lineWidth = 2.2;
      ctx.beginPath();
      ctx.arc(sp.x, sp.y, cellPx * 0.44, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * a.hp_ratio);
      ctx.stroke();
    }

    let lungeX = 0;
    let lungeY = 0;
    let slashAlpha = 0;
    if (pos.chopTimer > 0) {
      const p = 1 - pos.chopTimer / CHOP_DURATION;
      const thrust = Math.sin(Math.min(1, p / 0.65) * Math.PI);
      const fac = typeof pos.facing === 'number' ? pos.facing : 0;
      lungeX = Math.cos(fac) * thrust * cellPx * 0.14;
      lungeY = -Math.sin(fac) * thrust * cellPx * 0.10;
      if (p > 0.22 && p < 0.68) {
        slashAlpha = 1.0 - Math.abs((p - 0.45) / 0.23);
      }
    }

    const bobY = -Math.abs(Math.sin(pos.walkPhase || 0)) * mb * (cellPx * 0.08);
    if (tiles && tiles.isReady()) {
      tiles.drawHumanPaperDoll(ctx, a, sp.x + lungeX, sp.y + cellPx * 0.08 + lungeY, S, {
        facing: pos.facing,
        wearHat: showHat,
        showPack,
        tool,
        bobY,
      });
    } else {
      ctx.fillStyle = a.is_player ? '#8b2020' : '#2a4a6b';
      ctx.beginPath();
      ctx.arc(sp.x, sp.y, Math.max(6, cellPx * 0.30), 0, Math.PI * 2);
      ctx.fill();
    }

    if (slashAlpha > 0) {
      const fac = typeof pos.facing === 'number' ? pos.facing : 0;
      const ang = -fac;
      ctx.strokeStyle = `rgba(255, 244, 214, ${0.78 * slashAlpha})`;
      ctx.lineWidth = Math.max(2, cellPx * 0.08);
      ctx.lineCap = 'round';
      ctx.beginPath();
      ctx.arc(sp.x, sp.y - cellPx * 0.12, cellPx * 0.48, ang - 0.85, ang + 0.85);
      ctx.stroke();
    }

    ctx.restore();
    void nowMs;
  }

  function drawCddaBeast(a, pos, sp) {
    const tiles = window.CddaTiles;
    ctx.save();
    if (typeof a.hp_ratio === 'number' && a.hp_ratio < 0.99) {
      ctx.strokeStyle = '#c4322c';
      ctx.lineWidth = 2.0;
      ctx.beginPath();
      ctx.arc(sp.x, sp.y, cellPx * 0.42, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * a.hp_ratio);
      ctx.stroke();
    }
    if (tiles && tiles.isReady()) {
      tiles.drawBeast(ctx, a, sp.x, sp.y + cellPx * 0.06, cellPx, pos.facing, pos.moveBlend, pos.walkPhase);
    } else {
      ctx.fillStyle = '#7c3f23';
      ctx.beginPath();
      ctx.arc(sp.x, sp.y, Math.max(5, cellPx * 0.25), 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  }

  function isCellUnderClosedRoof(mx, my) {
    for (const rm of worldRooms) {
      if (mx >= rm.x0 && mx <= rm.x1 && my >= rm.y0 + 1 && my <= rm.y1) {
        const al = roomRoofAlpha.has(roomKeyOf(rm)) ? roomRoofAlpha.get(roomKeyOf(rm)) : 1.0;
        if (al > 0.35) return true;
      }
    }
    return false;
  }

  function isSouthWallOfClosedRoom(mx, my) {
    for (const rm of worldRooms) {
      if (mx >= rm.x0 && mx <= rm.x1 && my === rm.y0) {
        const al = roomRoofAlpha.has(roomKeyOf(rm)) ? roomRoofAlpha.get(roomKeyOf(rm)) : 1.0;
        if (al > 0.35) return true;
      }
    }
    return false;
  }

  function isPosUnderOpaqueRoof(wx, wy) {
    for (const rm of worldRooms) {
      if (wx >= rm.x0 + 0.18 && wx <= rm.x1 - 0.18 && wy >= rm.y0 + 0.18 && wy <= rm.y1 - 0.18) {
        const al = roomRoofAlpha.has(roomKeyOf(rm)) ? roomRoofAlpha.get(roomKeyOf(rm)) : 1.0;
        if (al > 0.35) return true;
      }
    }
    return false;
  }

  function drawCddaRoofRect(x0, x1, y0, y1, alpha, bCode) {
    const tiles = window.CddaTiles;
    const visAlpha = alpha * Math.min(1.0, (cellPx - 5.0) / 5.0);
    if (visAlpha <= 0.02) return;

    const rx0 = x0;
    const rx1 = x1;
    const ry0 = Math.min(y1, y0 + 1);
    const ry1 = y1;
    const isRoofAt = (nx, ny) => nx >= rx0 && nx <= rx1 && ny >= ry0 && ny <= ry1;

    ctx.save();
    ctx.globalAlpha = visAlpha;

    // 屋簷垂落南面牆頂之柔和陰影條（保留南立面牆體與大門可見，不溢出街道）
    for (let mx = rx0; mx <= rx1; mx++) {
      const se = tileEdges(mx, y0);
      if (se.x1 < 0 || se.x0 > viewportW || se.y1 < 0 || se.y0 > viewportH) continue;
      ctx.fillStyle = 'rgba(18, 12, 8, 0.24)';
      ctx.fillRect(se.x0, se.y0, se.w, Math.max(3, Math.round(cellPx * 0.16)));
    }

    const ready = Boolean(tiles && tiles.isReady());
    for (let my = ry1; my >= ry0; my--) {
      for (let mx = rx0; mx <= rx1; mx++) {
        const e = tileEdges(mx, my);
        if (e.x1 < 0 || e.x0 > viewportW || e.y1 < 0 || e.y0 > viewportH) continue;
        if (ready) {
          tiles.drawRoofTile(ctx, mx, my, e.x0, e.y0, e.x1, e.y1, bCode, isRoofAt);
        } else {
          ctx.fillStyle = '#8b573b';
          ctx.fillRect(e.x0, e.y0, e.w, e.h);
        }
      }
    }
    ctx.restore();
  }

  function drawBuildingRoofsB(dt, nowMs) {
    if (cellPx < 5.0 || !worldRooms.length) return;
    const me = actors.find((a) => a.is_player);
    const meSt = me ? renderActors.get(me.id) : null;
    const meX = meSt ? meSt.x : (me ? me.px : -9999);
    const meY = meSt ? meSt.y : (me ? me.py : -9999);

    for (const rm of worldRooms) {
      const rk = roomKeyOf(rm);
      const doors = Array.isArray(rm.doors) ? rm.doors : [];
      const meInside = me && meX >= rm.x0 - 0.35 && meX <= rm.x1 + 0.35 && meY >= rm.y0 - 0.35 && meY <= rm.y1 + 0.35;
      const nearDoor = me && doors.some((dp) => Math.hypot(meX - dp[0], meY - dp[1]) <= 1.1);
      const selInside = selectedCell && selectedCell.mx >= rm.x0 && selectedCell.mx <= rm.x1 && selectedCell.my >= rm.y0 && selectedCell.my <= rm.y1;
      const targetAlpha = (meInside || nearDoor || selInside) ? 0.0 : 1.0;

      let alpha = roomRoofAlpha.has(rk) ? roomRoofAlpha.get(rk) : targetAlpha;
      alpha += (targetAlpha - alpha) * (1.0 - Math.exp(-9.0 * dt));
      if (Math.abs(alpha - targetAlpha) < 0.01) alpha = targetAlpha;
      roomRoofAlpha.set(rk, alpha);
    }

    const handledRooms = new Set();
    for (const b of (Array.isArray(worldBuildings) ? worldBuildings : [])) {
      const bRooms = worldRooms.filter((r) => r.x0 >= b.x0 && r.x1 <= b.x1 && r.y0 >= b.y0 && r.y1 <= b.y1);
      if (!bRooms.length) continue;
      for (const r of bRooms) handledRooms.add(roomKeyOf(r));
      const allClosed = bRooms.every((r) => (roomRoofAlpha.get(roomKeyOf(r)) ?? 1.0) > 0.94);
      if (allClosed && bRooms.length >= 2) {
        const minA = Math.min(...bRooms.map((r) => roomRoofAlpha.get(roomKeyOf(r)) ?? 1.0));
        drawCddaRoofRect(b.x0, b.x1, b.y0, b.y1, minA, b.code || '');
      } else {
        for (const rm of bRooms) {
          const alpha = roomRoofAlpha.get(roomKeyOf(rm)) ?? 1.0;
          drawCddaRoofRect(rm.x0, rm.x1, rm.y0, rm.y1, alpha, b.code || '');
        }
      }
    }

    for (const rm of worldRooms) {
      if (handledRooms.has(roomKeyOf(rm))) continue;
      const alpha = roomRoofAlpha.get(roomKeyOf(rm)) ?? 1.0;
      drawCddaRoofRect(rm.x0, rm.x1, rm.y0, rm.y1, alpha, buildingCodeAt(rm.x0, rm.y0));
    }

    // 繪製街鎮商住木牌匾（掀頂時移至建物屋脊上方免遮室內家具）
    if (cellPx >= 7.0 && Array.isArray(worldBuildings) && worldBuildings.length) {
      ctx.save();
      const fontSz = Math.round(Math.max(11, Math.min(14, cellPx * 0.44)));
      ctx.font = `700 ${fontSz}px "Noto Sans TC", serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      for (const b of worldBuildings) {
        const bp = microToScreen(b.label_x, b.label_y);
        if (bp.x < -120 || bp.x > viewportW + 120 || bp.y < -80 || bp.y > viewportH + 80) continue;
        let maxUnveiled = 0;
        for (const rm of worldRooms) {
          if (rm.x1 >= b.x0 && rm.x0 <= b.x1 && rm.y1 >= b.y0 && rm.y0 <= b.y1) {
            const al = roomRoofAlpha.has(roomKeyOf(rm)) ? roomRoofAlpha.get(roomKeyOf(rm)) : 1.0;
            if (al < 0.45) maxUnveiled = 1;
          }
        }
        const badgeAlpha = maxUnveiled ? 0.80 : 0.94;
        const py = maxUnveiled ? microToScreen(b.label_x, b.y1 + 0.15).y : bp.y;
        const tw = ctx.measureText(b.name).width + 14;
        const th = fontSz + 8;
        ctx.globalAlpha = badgeAlpha * Math.min(1.0, (cellPx - 7.0) / 4.0);
        ctx.fillStyle = '#efe3cd';
        ctx.strokeStyle = '#3c2b1e';
        ctx.lineWidth = 1.25;
        ctx.beginPath();
        ctx.roundRect(bp.x - tw / 2, py - th / 2, tw, th, 4);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = '#2c1e0f';
        ctx.fillText(b.name, bp.x, py + 0.5);
      }
      ctx.restore();
    }
    void nowMs;
  }

  function drawMountainRain(nowMs) {
    const effW = getEffectiveWeather();
    if (!effW.raining || cellPx < 8.0) return;
    const isLight = effW.code === 'drizzle';
    const tSec = nowMs * 0.001;

    const unveiledRects = [];
    for (const rm of worldRooms) {
      const rk = roomKeyOf(rm);
      const alpha = roomRoofAlpha.has(rk) ? roomRoofAlpha.get(rk) : 1.0;
      if (alpha <= 0.5) {
        const pTL = microToScreen(rm.x0 - 0.84, rm.y1 + 0.86);
        const pBR = microToScreen(rm.x1 + 0.84, rm.y0 - 0.86);
        unveiledRects.push([pTL.x, pTL.y, pBR.x - pTL.x, pBR.y - pTL.y]);
      }
    }

    ctx.save();
    // 雨水漣漪（略過已掀頂室內）
    const rippleCount = isLight ? 18 : 30;
    for (let i = 0; i < rippleCount; i++) {
      const seed = hash2d(i, 709);
      const phase = (tSec * 1.45 + (seed % 100) / 100) % 1.0;
      const rx = ((hash2d(i, 19) % 1000) / 1000) * viewportW;
      const ry = ((hash2d(i, 53) % 1000) / 1000) * viewportH;
      if (unveiledRects.some((r) => rx >= r[0] && rx <= r[0] + r[2] && ry >= r[1] && ry <= r[1] + r[3])) {
        continue;
      }
      ctx.strokeStyle = `rgba(82, 108, 122, ${(1 - phase) * (isLight ? 0.30 : 0.44)})`;
      ctx.lineWidth = 1.05;
      const rad = 1.5 + phase * (cellPx * 0.16);
      ctx.beginPath();
      ctx.ellipse(rx, ry, rad, rad * 0.56, 0, 0, Math.PI * 2);
      ctx.stroke();
    }

    // 細墨雨絲（裁剪排除掀頂室內）
    if (unveiledRects.length > 0) {
      ctx.beginPath();
      ctx.rect(0, 0, viewportW, viewportH);
      for (const r of unveiledRects) {
        ctx.rect(r[0], r[1], r[2], r[3]);
      }
      ctx.clip('evenodd');
    }

    ctx.strokeStyle = isLight ? 'rgba(72, 92, 104, 0.24)' : 'rgba(66, 86, 98, 0.34)';
    ctx.lineWidth = 1.1;
    ctx.beginPath();
    const dropCount = isLight ? 56 : 108;
    for (let i = 0; i < dropCount; i++) {
      const sx = (hash2d(i, 101) % 1000) / 1000;
      const sy = (hash2d(i, 211) % 1000) / 1000;
      const speed = (isLight ? 0.68 : 0.90) + ((hash2d(i, 307) % 35) / 100);
      const py = ((sy + tSec * speed) % 1.08 - 0.04) * viewportH;
      const px = (((sx - tSec * speed * 0.14) % 1.08 + 1.08) % 1.08) * viewportW;
      const len = cellPx * (0.34 + ((hash2d(i, 409) % 20) / 100));
      ctx.moveTo(px, py);
      ctx.lineTo(px - len * 0.24, py + len);
    }
    ctx.stroke();
    ctx.restore();
  }

  const STRUCT_Y_SET = new Set([
    'wall', 'door', 'campfire', 'workbench', 'stockpile', 'bed', 'lamp',
    'herb_seed', 'herb_plot', 'crop_seed', 'crop_plot',
  ]);

  function drawActors(nowMs, pass = 'all') {
    const includeBase = pass === 'all' || pass === 'base';
    const includeFront = pass === 'all' || pass === 'front';
    const tiles = window.CddaTiles;

    // 本尊尋路預覽線
    const me = actors.find((a) => a.is_player);
    const meSt = me ? renderActors.get(me.id) : null;
    const isMeRunning = !!(meSt ? meSt.running : (me && me.running));
    const rawPath = (meSt && meSt.path && meSt.path.length > 0) ? meSt.path : (me && me.path ? me.path : []);
    if (includeBase && me && rawPath.length > 0) {
      const cur = meSt || { x: me.px, y: me.py };
      const drawPath = trimPassedWaypoints(rawPath, cur.x, cur.y);
      if (drawPath.length > 0) {
        ctx.save();
        ctx.strokeStyle = isMeRunning ? 'rgba(218, 48, 48, 0.88)' : 'rgba(248, 240, 224, 0.72)';
        ctx.lineWidth = isMeRunning ? 2.6 : 1.8;
        ctx.setLineDash(isMeRunning ? [7, 3] : [4, 5]);
        ctx.beginPath();
        const s0 = microToScreen(cur.x, cur.y);
        ctx.moveTo(s0.x, s0.y);
        const [p0x, p0y] = drawPath[0];
        if (Math.abs(p0x - cur.x) > 0.35 && Math.abs(p0y - cur.y) > 0.35) {
          const cxR = Math.round(cur.x);
          const cyR = Math.round(cur.y);
          const optA = cellMap.get(`${Math.round(p0x)},${cyR}`)?.s === 'wall'
            ? [cxR, Math.round(p0y)]
            : [Math.round(p0x), cyR];
          const se = microToScreen(optA[0], optA[1]);
          ctx.lineTo(se.x, se.y);
        }
        for (const [px, py] of drawPath) {
          const sp = microToScreen(px, py);
          ctx.lineTo(sp.x, sp.y);
        }
        ctx.stroke();
        ctx.restore();
      }
    }

    const isMacroView = cellPx < 5.0;
    const isNearView = cellPx >= 12.0;

    if (cellPx >= 5.0) {
      const renderItems = [];
      const blueprintMarkers = [];
      const labelDrawList = [];
      const halfW = Math.min(40, Math.ceil(viewportW / cellPx / 2) + 2);
      const halfH = Math.min(40, Math.ceil(viewportH / cellPx / 2) + 2);
      const cx = Math.round(camX);
      const cy = Math.round(camY);

      for (let my = cy + halfH; my >= cy - halfH; my--) {
        for (let mx = cx - halfW; mx <= cx + halfW; mx++) {
          const c = cellMap.get(`${mx},${my}`);
          if (!c) continue;
          const e = tileEdges(mx, my);
          if (e.x1 < -cellPx || e.x0 > viewportW + cellPx || e.y1 < -cellPx * 2 || e.y0 > viewportH + cellPx) {
            continue;
          }
          const covered = isCellUnderClosedRoof(mx, my);
          const isWallStruct = c.s === 'wall' || c.s === 'door';
          const inBase = covered || (isWallStruct && isSouthWallOfClosedRoom(mx, my));
          if ((inBase && includeBase) || (!inBase && includeFront)) {
            if (c.s === 'wall') {
              renderItems.push({ kind: 'wall', sortY: e.y1, rank: 1, mx, my, e });
            } else if (c.s === 'door') {
              renderItems.push({ kind: 'door', sortY: e.y1, rank: 1, mx, my, e });
            } else if (c.s && STRUCT_Y_SET.has(c.s)) {
              const isCrop = c.s === 'herb_seed' || c.s === 'herb_plot' || c.s === 'crop_seed' || c.s === 'crop_plot';
              renderItems.push({
                kind: 'struct',
                sortY: e.y1,
                rank: isCrop ? 0 : 2,
                mx,
                my,
                sKind: c.s,
                c,
                e,
              });
            }
            if (c.p === 'bamboo' || c.p === 'shoot' || c.p === 'herb' || c.p === 'stone') {
              renderItems.push({ kind: 'prop', sortY: e.y1, rank: 2, mx, my, pType: c.p, e });
            }
          }
          if (c.b && ((covered && includeBase) || (!covered && includeFront))) {
            blueprintMarkers.push({ mx, my, c, e });
          }
        }
      }

      if (isNearView) {
        for (const a of actors) {
          const pos = renderActors.get(a.id) || {
            x: a.px,
            y: a.py,
            facing: a.facing || 0,
            walkPhase: 0,
            moveBlend: 0,
            runBlend: 0,
            chopTimer: 0,
          };
          const sp = microToScreen(pos.x, pos.y);
          if (sp.x < -70 || sp.x > viewportW + 70 || sp.y < -70 || sp.y > viewportH + 70) continue;
          if (isActorSleepingInBed(a, pos)) continue;
          const covered = isPosUnderOpaqueRoof(pos.x, pos.y);
          if ((covered && !includeBase) || (!covered && !includeFront)) continue;
          const rank = a.is_player ? 4 : 3;
          renderItems.push({
            kind: 'actor',
            sortY: Math.round(sp.y + cellPx * 0.42),
            rank,
            a,
            pos,
            sp,
          });
          const badge = a.pop || a.carry;
          if (badge && !covered) {
            labelDrawList.push({ a, sp, badge });
          }
        }
      }

      renderItems.sort((u, v) => (u.sortY - v.sortY) || (u.rank - v.rank));

      for (const item of renderItems) {
        const e = item.e;
        if (item.kind === 'wall') {
          if (tiles && tiles.isReady()) {
            tiles.drawWallTile(ctx, item.mx, item.my, e.x0, e.y0, e.x1, e.y1, buildingCodeAt(item.mx, item.my), hasWallAt);
          } else {
            ctx.fillStyle = '#8d7a64';
            ctx.fillRect(e.x0, e.y0, e.w, e.h);
          }
        } else if (item.kind === 'door') {
          if (tiles && tiles.isReady()) {
            tiles.drawDoorTile(ctx, e.x0, e.y0, e.x1, e.y1, isActorNearCell(item.mx, item.my, 1.05));
          } else {
            ctx.fillStyle = '#b48a5a';
            ctx.fillRect(e.x0, e.y0, e.w, e.h);
          }
        } else if (item.kind === 'struct') {
          const sleeper = item.sKind === 'bed' ? findBedSleeper(item.mx, item.my) : null;
          if (tiles && tiles.isReady()) {
            tiles.drawStructureProp(
              ctx,
              item.sKind,
              item.mx,
              item.my,
              e.x0,
              e.y0,
              e.x1,
              e.y1,
              indoorCellSet.has(`${item.mx},${item.my}`),
              buildingCodeAt(item.mx, item.my),
              nowMs,
              item.c.i,
              sleeper,
            );
          }
          if (item.sKind === 'bed' && sleeper) {
            bedBadgeQueue.push({
              mx: item.mx,
              my: item.my,
              cx: (e.x0 + e.x1) * 0.5,
              by: e.y0 - 10,
              txt: `${sleeper.name || sleeper.glyph || ''}・榻上安歇`,
              isPlayer: !!sleeper.is_player,
            });
          }
        } else if (item.kind === 'prop') {
          if (tiles && tiles.isReady()) {
            tiles.drawNaturalProp(
              ctx,
              item.pType,
              item.mx,
              item.my,
              e.x0,
              e.y0,
              e.x1,
              e.y1,
              nowMs,
              bambooShakeMap.get(`${item.mx},${item.my}`) || 0,
              hash2d,
            );
          }
        } else if (item.a.faction === 'wild_beast') {
          drawCddaBeast(item.a, item.pos, item.sp);
        } else {
          drawCddaHuman(item.a, item.pos, item.sp, nowMs);
        }
      }

      if (includeFront) {
        for (const lp of leafParticles) {
          const lpt = microToScreen(lp.x, lp.y);
          ctx.save();
          ctx.globalAlpha = Math.max(0, lp.life / 0.62);
          ctx.translate(lpt.x, lpt.y);
          ctx.rotate(lp.rot);
          ctx.fillStyle = '#7fa36b';
          ctx.strokeStyle = '#2c1e0f';
          ctx.lineWidth = 0.8;
          ctx.beginPath();
          ctx.ellipse(0, 0, 4.8, 1.9, 0, 0, Math.PI * 2);
          ctx.fill();
          ctx.stroke();
          ctx.restore();
        }
      }

      for (const bm of blueprintMarkers) {
        const isChop = bm.c.b.k === 'chop' || bm.c.b.k === 'decon';
        const color = isChop ? '#c4322c' : '#2a4a6b';
        const be = bm.e;
        ctx.save();
        ctx.strokeStyle = color;
        ctx.lineWidth = 1.6;
        ctx.setLineDash([4, 3]);
        ctx.strokeRect(be.x0 + 2, be.y0 + 2, be.w - 4, be.h - 4);
        ctx.setLineDash([]);
        if (cellPx >= 16) {
          const tag = bm.c.b.k === 'chop' ? '伐' : (bm.c.b.k === 'decon' ? '拆' : '築');
          const bcx = (be.x0 + be.x1) * 0.5;
          const bcy = (be.y0 + be.y1) * 0.5;
          ctx.fillStyle = 'rgba(247, 240, 227, 0.92)';
          ctx.strokeStyle = color;
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.roundRect(bcx - 8, bcy - 8, 16, 16, 3);
          ctx.fill();
          ctx.stroke();
          ctx.fillStyle = color;
          ctx.font = 'bold 10px "Noto Sans TC", sans-serif';
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillText(tag, bcx, bcy + 0.5);
        }
        ctx.restore();
      }

      if (isNearView) {
        for (const lbl of labelDrawList) {
          ctx.font = '600 11px "Noto Sans TC", sans-serif';
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          const tw = ctx.measureText(lbl.badge).width + 10;
          const by = lbl.sp.y - cellPx * 0.96 - 10;
          ctx.fillStyle = 'rgba(247, 240, 227, 0.94)';
          ctx.strokeStyle = lbl.a.pop ? '#8b2020' : '#a89880';
          ctx.lineWidth = 1;
          ctx.fillRect(lbl.sp.x - tw / 2, by - 8, tw, 16);
          ctx.strokeRect(lbl.sp.x - tw / 2, by - 8, tw, 16);
          ctx.fillStyle = lbl.a.pop ? '#8b2020' : '#2c1e0f';
          ctx.fillText(lbl.badge, lbl.sp.x, by + 0.5);
        }

        if (includeFront && bedBadgeQueue.length) {
          ctx.font = '600 10.5px "Noto Sans TC", sans-serif';
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          for (const bl of bedBadgeQueue) {
            if (getRoomRoofAlphaAt(bl.mx, bl.my) > 0.5) continue;
            const tw = ctx.measureText(bl.txt).width + 10;
            ctx.fillStyle = 'rgba(247, 240, 227, 0.94)';
            ctx.strokeStyle = bl.isPlayer ? '#8b2020' : '#a89880';
            ctx.lineWidth = 1;
            ctx.fillRect(bl.cx - tw / 2, bl.by - 8, tw, 16);
            ctx.strokeRect(bl.cx - tw / 2, bl.by - 8, tw, 16);
            ctx.fillStyle = bl.isPlayer ? '#8b2020' : '#2c1e0f';
            ctx.fillText(bl.txt, bl.cx, bl.by + 0.5);
          }
        }
        return;
      }
    }

    // 中景過渡（5 <= cellPx < 12）與遠景鳥瞰（cellPx < 5）角色標記
    const sortedActors = [...actors].sort((a, b) => {
      const rank = (x) => (x.is_player ? 3 : (x.faction === 'wild_beast' ? 1 : 2));
      return rank(a) - rank(b);
    });

    for (const a of sortedActors) {
      const pos = renderActors.get(a.id) || { x: a.px, y: a.py, facing: a.facing || 0 };
      const covered = !isMacroView && isPosUnderOpaqueRoof(pos.x, pos.y);
      if ((covered && !includeBase) || (!covered && !includeFront)) continue;
      const sp = microToScreen(pos.x, pos.y);
      if (sp.x < -40 || sp.x > viewportW + 40 || sp.y < -40 || sp.y > viewportH + 40) continue;

      const isBeast = a.faction === 'wild_beast';
      if (isMacroView) {
        if (a.is_player) {
          ctx.fillStyle = '#f7f0e3';
          ctx.strokeStyle = '#8b2020';
          ctx.lineWidth = 2.4;
          ctx.beginPath();
          ctx.arc(sp.x, sp.y, 10.5, 0, Math.PI * 2);
          ctx.fill();
          ctx.stroke();

          ctx.fillStyle = '#8b2020';
          ctx.beginPath();
          ctx.arc(sp.x, sp.y, 4.4, 0, Math.PI * 2);
          ctx.fill();
        } else {
          ctx.fillStyle = isBeast ? '#7c3f23' : '#2a4a6b';
          ctx.strokeStyle = '#f7f0e3';
          ctx.lineWidth = 1.2;
          ctx.beginPath();
          ctx.arc(sp.x, sp.y, 4.0, 0, Math.PI * 2);
          ctx.fill();
          ctx.stroke();
        }
        continue;
      }

      const tokenR = Math.max(5.5, cellPx * 0.48);
      const strokeColor = a.is_player ? '#8b2020' : (isBeast ? '#7c3f23' : '#2a4a6b');
      const fillColor = isBeast ? '#efe2d1' : '#f7f0e3';

      ctx.fillStyle = 'rgba(44, 30, 15, 0.14)';
      ctx.beginPath();
      ctx.arc(sp.x, sp.y + 1.5, tokenR, 0, Math.PI * 2);
      ctx.fill();

      ctx.fillStyle = fillColor;
      ctx.strokeStyle = strokeColor;
      ctx.lineWidth = a.is_player ? 2.2 : 1.6;
      ctx.beginPath();
      ctx.arc(sp.x, sp.y, tokenR, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();

      if (typeof a.hp_ratio === 'number' && a.hp_ratio < 0.99) {
        ctx.strokeStyle = '#8b2020';
        ctx.lineWidth = 1.8;
        ctx.beginPath();
        ctx.arc(sp.x, sp.y, tokenR + 2.4, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * a.hp_ratio);
        ctx.stroke();
      }

      if (typeof pos.facing === 'number') {
        const fx = sp.x + Math.cos(pos.facing) * (tokenR - 0.8);
        const fy = sp.y - Math.sin(pos.facing) * (tokenR - 0.8);
        ctx.fillStyle = strokeColor;
        ctx.beginPath();
        ctx.arc(fx, fy, 2.2, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }

  function drawScaleBar() {
    const badge = $('rim-scale-badge');
    const zBtn = $('btn-zoom-toggle');
    if (cellPx >= 5.0) {
      if (badge) badge.textContent = '近景 1m';
      if (zBtn) zBtn.textContent = '鳥瞰';
    } else {
      if (badge) badge.textContent = '鳥瞰 20m';
      if (zBtn) zBtn.textContent = '近景';
    }

    const candidates = [
      [5000, '5 km'],
      [1000, '1 km'],
      [200, '200 m'],
      [20, '20 m'],
      [10, '10 m'],
      [5, '5 m'],
    ];
    let pick = candidates[candidates.length - 1];
    for (const c of candidates) {
      if (c[0] * cellPx <= 130) {
        pick = c;
        break;
      }
    }
    const barW = Math.max(24, pick[0] * cellPx);
    const x = 14;
    const y = viewportH - 14;
    ctx.save();
    ctx.strokeStyle = 'rgba(239, 232, 216, 0.88)';
    ctx.lineWidth = 4.2;
    ctx.beginPath();
    ctx.moveTo(x, y - 5);
    ctx.lineTo(x, y);
    ctx.lineTo(x + barW, y);
    ctx.lineTo(x + barW, y - 5);
    ctx.stroke();
    ctx.strokeStyle = '#2c1e0f';
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.font = '600 11px "Noto Sans TC", sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'bottom';
    ctx.lineWidth = 3.5;
    ctx.strokeStyle = 'rgba(239, 232, 216, 0.92)';
    ctx.strokeText(pick[1], x + barW / 2, y - 4);
    ctx.fillStyle = '#2c1e0f';
    ctx.fillText(pick[1], x + barW / 2, y - 4);
    ctx.restore();
  }

  function renderScene(nowMs) {
    const dt = prevFrameMs ? Math.min(0.06, Math.max(0.001, (nowMs - prevFrameMs) / 1000)) : 0.016;
    prevFrameMs = nowMs;
    const appEl = $('rim-app');
    if (appEl && appEl.hidden) {
      refreshAuthClock();
      return;
    }
    stepActorKinematics(dt);
    const clockInfo = window.SwSkyTime ? window.SwSkyTime.applyDocumentSkyCss() : null;
    if (clockInfo) {
      const clk = $('rim-world-clock');
      const effW = getEffectiveWeather();
      const txt = effW.label ? `${clockInfo.clockText}・${effW.label}` : clockInfo.clockText;
      if (clk && clk.textContent !== txt) {
        clk.textContent = txt;
      }
    }
    ctx.fillStyle = (cellPx >= 5.0 && window.CddaTiles && window.CddaTiles.isReady()) ? '#2f3b28' : '#efe8d8';
    ctx.fillRect(0, 0, viewportW, viewportH);
    drawMacroBasemap();
    drawMicroLayer(nowMs);
    drawActors(nowMs, 'base');
    drawBuildingRoofsB(dt, nowMs);
    drawActors(nowMs, 'front');
    drawMountainRain(nowMs);

    if (window.SwSkyTime) {
      const lights = [];
      const effW = getEffectiveWeather();
      if (cellPx >= 5.0) {
        for (const c of cellMap.values()) {
          if (c.s !== 'campfire' && c.s !== 'workbench' && c.s !== 'lamp') continue;
          const p = microToScreen(c.x, c.y);
          if (p.x < -220 || p.x > viewportW + 220 || p.y < -220 || p.y > viewportH + 220) continue;
          const wetCamp = c.s === 'campfire' && effW.raining && !indoorCellSet.has(`${c.x},${c.y}`);
          const radius = c.s === 'lamp'
            ? cellPx * 5.6
            : cellPx * (c.s === 'campfire' ? (wetCamp ? 2.2 : 5.6) : 2.5);
          lights.push({
            x: p.x,
            y: p.y,
            radius,
            kind: c.s,
          });
        }
        for (const rm of worldRooms) {
          const rk = roomKeyOf(rm);
          const alpha = roomRoofAlpha.has(rk) ? roomRoofAlpha.get(rk) : 1.0;
          if (alpha <= 0.5) {
            const c = microToScreen((rm.x0 + rm.x1) * 0.5, (rm.y0 + rm.y1) * 0.5);
            lights.push({
              x: c.x,
              y: c.y,
              radius: cellPx * Math.max(rm.x1 - rm.x0, rm.y1 - rm.y0) * 0.68,
              kind: 'campfire',
            });
          } else if (Array.isArray(rm.doors)) {
            for (const dp of rm.doors) {
              const ox = dp[0] === rm.x0 ? -0.65 : (dp[0] === rm.x1 ? 0.65 : 0);
              const oy = dp[1] === rm.y1 ? 0.65 : (dp[1] === rm.y0 ? -0.65 : 0);
              const gp = microToScreen(dp[0] + ox, dp[1] + oy);
              lights.push({ x: gp.x, y: gp.y, radius: cellPx * 2.2, kind: 'workbench' });
            }
          }
        }
        const me = actors.find((a) => a.is_player);
        if (me) {
          const meSt = renderActors.get(me.id);
          const p = microToScreen(meSt ? meSt.x : me.px, meSt ? meSt.y : me.py);
          lights.push({ x: p.x, y: p.y, radius: cellPx * 3.4, kind: 'player' });
        }
      }
      window.SwSkyTime.drawCanvasAtmosphere(ctx, viewportW, viewportH, lights, null, nowMs);
    }

    drawScaleBar();
  }

  function frame(nowMs) {
    renderScene(nowMs);
    requestAnimationFrame(frame);
  }

  function zoomAt(sx, sy, factor) {
    const before = screenToMicro(sx, sy);
    cellPx = Math.max(MIN_CELL_PX, Math.min(MAX_CELL_PX, cellPx * factor));
    camX = before.fx - (sx - viewportW / 2) / cellPx;
    camY = before.fy + (sy - viewportH / 2) / cellPx;
    forceCellsNext = true;
  }

  async function handleTapCell(sx, sy) {
    const { mx, my, fx, fy } = screenToMicro(sx, sy);

    // 點中家具設備時須優先開啟家具（僅人在正上方才算先點中人）；空地／路面則放寬人物點選半徑
    const cell = cellMap.get(`${mx},${my}`);
    const hasFixture = Boolean(cell && ((cell.s && cell.s !== 'paving' && cell.s !== 'arcade' && cell.s !== 'wall') || cell.p || cell.b || cell.i));
    const hitActor = actors.find((a) => {
      if (a.is_player) return false;
      const st = renderActors.get(a.id);
      const ax = st ? st.x : a.px;
      const ay = st ? st.y : a.py;
      if (hasFixture) return Math.hypot(ax - fx, ay - fy) <= 0.48;
      return Math.hypot(ax - fx, ay - fy) <= 0.95 || Math.hypot(a.px - mx, a.py - my) <= 0.95;
    });
    const hasInteractive = hitActor || (cell && (cell.p || cell.s || cell.b || cell.i || cell.g === 'stream'));

    if (hasInteractive) {
      if (hitActor) {
        openCellSocket(Math.round(hitActor.px), Math.round(hitActor.py));
      } else {
        openCellSocket(mx, my);
      }
      return;
    }

    // 點擊平坦空地：單擊常速行走（1.35 m/s），雙擊疾奔（3.80 m/s）
    closeInspector();
    const now = performance.now();
    const isDoubleTap =
      now - lastTap.ms <= 340 &&
      (Math.hypot(sx - lastTap.sx, sy - lastTap.sy) <= 36 || Math.hypot(mx - lastTap.mx, my - lastTap.my) <= 2);
    lastTap = isDoubleTap
      ? { ms: 0, sx: -999, sy: -999, mx: -999, my: -999 }
      : { ms: now, sx, sy, mx, my };

    const r = await dispatchPlayerAct({ mx, my, action: 'move', run: isDoubleTap });
    if (r && !r.ok && r.msg) {
      showToast(r.msg);
    }
  }

  // 滑鼠右鍵直接開啟地面／溪流／物件現場插座卡
  canvas.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    clearLongPress();
    const rect = canvas.getBoundingClientRect();
    const { mx, my } = screenToMicro(e.clientX - rect.left, e.clientY - rect.top);
    openCellSocket(mx, my);
  });

  canvas.addEventListener('pointerdown', (e) => {
    if (e.button === 2) return; // 右鍵交由 contextmenu 處理
    try { canvas.setPointerCapture(e.pointerId); } catch (_) {}
    pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    const rect = canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;

    if (pointers.size === 2) {
      clearLongPress();
      const pts = Array.from(pointers.values());
      pinchDist0 = Math.hypot(pts[0].x - pts[1].x, pts[0].y - pts[1].y);
      pinchCellPx0 = cellPx;
      dragStart = null;
      return;
    }

    dragStart = { sx, sy, camX0: camX, camY0: camY, moved: false, longPressed: false };
    clearLongPress();
    if (cellPx >= 5.0) {
      longPressTimer = setTimeout(() => {
        if (dragStart && !dragStart.moved && pointers.size === 1) {
          dragStart.longPressed = true;
          const { mx, my } = screenToMicro(dragStart.sx, dragStart.sy);
          openCellSocket(mx, my);
        }
      }, 360);
    }
  });

  canvas.addEventListener('pointermove', (e) => {
    if (!pointers.has(e.pointerId)) return;
    pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    const rect = canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;

    if (pointers.size === 2 && pinchDist0 > 10) {
      clearLongPress();
      const pts = Array.from(pointers.values());
      const dist = Math.hypot(pts[0].x - pts[1].x, pts[0].y - pts[1].y);
      const midX = (pts[0].x + pts[1].x) / 2 - rect.left;
      const midY = (pts[0].y + pts[1].y) / 2 - rect.top;
      const targetPx = Math.max(MIN_CELL_PX, Math.min(MAX_CELL_PX, pinchCellPx0 * (dist / pinchDist0)));
      zoomAt(midX, midY, targetPx / cellPx);
      return;
    }

    if (!dragStart) return;
    const dx = sx - dragStart.sx;
    const dy = sy - dragStart.sy;
    if (Math.hypot(dx, dy) > 6) {
      dragStart.moved = true;
      clearLongPress();
    }
    if (dragStart.moved && !dragStart.longPressed) {
      followMe = false;
      $('btn-focus-me')?.classList.remove('rim-chip-active');
      camX = dragStart.camX0 - dx / cellPx;
      camY = dragStart.camY0 + dy / cellPx;
      forceCellsNext = true;
    }
  });

  function endPointer(e) {
    clearLongPress();
    pointers.delete(e.pointerId);
    const rect = canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;

    if (dragStart && !dragStart.moved && !dragStart.longPressed && pointers.size === 0) {
      handleTapCell(sx, sy);
    }
    if (pointers.size === 0) {
      dragStart = null;
    }
  }

  canvas.addEventListener('pointerup', endPointer);
  canvas.addEventListener('pointercancel', endPointer);

  canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    const rect = canvas.getBoundingClientRect();
    const factor = e.deltaY < 0 ? 1.18 : 1 / 1.18;
    zoomAt(e.clientX - rect.left, e.clientY - rect.top, factor);
  }, { passive: false });

  $('insp-close').onclick = closeInspector;

  $('btn-self-sheet').onclick = () => openModal('self');
  $('btn-gear-sheet').onclick = () => openModal('gear');
  $('rim-modal-close').onclick = closeModal;
  $('rim-modal-backdrop').addEventListener('click', (e) => {
    if (e.target === $('rim-modal-backdrop')) {
      closeModal();
    }
  });

  $('btn-focus-me').onclick = () => {
    followMe = true;
    $('btn-focus-me').classList.add('rim-chip-active');
    const me = actors.find((a) => a.is_player);
    if (me) {
      camX = me.px;
      camY = me.py;
    }
    if (cellPx < 14) cellPx = 32.0;
  };

  $('btn-zoom-toggle').onclick = () => {
    if (cellPx >= 5.0) {
      followMe = false;
      $('btn-focus-me').classList.remove('rim-chip-active');
      cellPx = 0.11; // 一鍵拉遠鳥瞰整座大甲溪～烏溪真實台灣山海母圖
    } else {
      followMe = true;
      $('btn-focus-me').classList.add('rim-chip-active');
      cellPx = 32.0;
    }
    pollState(true);
  };

  let engineStarted = false;
  function enterWorld(playerId, displayChar) {
    const authEl = $('auth-screen');
    const appEl = $('rim-app');
    if (authEl) authEl.hidden = true;
    if (appEl) appEl.hidden = false;
    if (playerId) {
      localStorage.setItem('sw_last_id', playerId);
    }
    if (displayChar) {
      localStorage.setItem('sw_display_char', displayChar);
    }
    followMe = true;
    $('btn-focus-me')?.classList.add('rim-chip-active');
    lastInspectorSig = '';
    lastModalSig = '';
    resize();
    if (!engineStarted) {
      engineStarted = true;
      if (window.CddaTiles) window.CddaTiles.onReady(() => renderScene(performance.now()));
      loadSubstrateMeta();
      setInterval(() => pollState(false), 110);
      requestAnimationFrame(frame);
    }
    pollState(true);
  }

  async function dispatchPlayerAct({ mx, my, action = 'move', target_actor, run = false }) {
    const me = actors.find((a) => a.is_player);
    const meSt = me ? renderActors.get(me.id) : null;
    const moveSeq = ++meMoveSeq;
    meCmdInFlight++;
    meMoveLockUntil = performance.now() + 450;
    meArriveHoldUntil = 0;
    if (me) me.autopilot = false;
    if (meSt) {
      meSt.autopilot = false;
      meSt.running = !!run;
      meSt.speed = run ? 3.8 : 1.35;
      const preview = findClientPath(Math.round(meSt.x), Math.round(meSt.y), mx, my);
      meSt.path = preview && preview.length > 0 ? trimPassedWaypoints(preview, meSt.x, meSt.y) : [];
    }
    try {
      const r = await sendCmd({ cmd: 'act', mx, my, action, target_actor, run });
      if (moveSeq === meMoveSeq && meSt) {
        if (!r || !r.ok) {
          meSt.path = [];
        } else if (Array.isArray(r.path)) {
          if (typeof r.px === 'number' && typeof r.py === 'number') {
            meSt.srvX = r.px;
            meSt.srvY = r.py;
          }
          meSt.srvMoving = r.path.length > 0;
          meSt.path = r.path.length > 0 ? trimPassedWaypoints(r.path, meSt.x, meSt.y) : [];
          meSt.speed = r.speed || (run ? 3.8 : 1.35);
          meSt.running = !!r.running;
          meMoveLockUntil = performance.now() + 420;
        }
      }
      return r;
    } finally {
      meCmdInFlight = Math.max(0, meCmdInFlight - 1);
    }
  }

  // ═══ 宣紙登入／創角閘門（正式帳密登入＋PostgreSQL 同源 Cookie 階段） ═══
  function setupAuthGate() {
    localStorage.removeItem('sw_rim_authed_id');
    const authEl = $('auth-screen');
    const form = $('auth-form');
    const idInput = $('auth-id');
    const pwInput = $('auth-password');

    if (!authEl || !form) {
      enterWorld('', '');
      return;
    }

    const savedId = localStorage.getItem('sw_last_id') || '';
    if (savedId && idInput && !idInput.value) {
      idInput.value = savedId;
    }
    refreshAuthClock();
    setInterval(() => {
      if (!authEl.hidden) refreshAuthClock();
    }, 1000);

    const switchBtn = $('auth-btn-switch');
    const backBtn = $('auth-btn-back');
    if (switchBtn) {
      switchBtn.onclick = () => {
        $('auth-hint').textContent = '建立新角色（ID 與密碼登入用）';
        $('auth-display-wrap').hidden = false;
        $('auth-gender-wrap').hidden = false;
        $('auth-login-actions').hidden = true;
        $('auth-create-actions').hidden = false;
        const errEl = $('auth-message');
        if (errEl) errEl.textContent = '';
      };
    }
    if (backBtn) {
      backBtn.onclick = () => {
        $('auth-hint').textContent = '請輸入 ID 與密碼登入';
        $('auth-display-wrap').hidden = true;
        $('auth-gender-wrap').hidden = true;
        $('auth-login-actions').hidden = false;
        $('auth-create-actions').hidden = true;
        const errEl = $('auth-message');
        if (errEl) errEl.textContent = '';
      };
    }

    fetch('/api/rim/me', { credentials: 'same-origin' })
      .then((res) => res.json().catch(() => null))
      .then((data) => {
        if (data && data.sky_time && window.SwSkyTime) {
          window.SwSkyTime.syncFromServer(data.sky_time);
          refreshAuthClock();
        }
        const sameTabOk = sessionStorage.getItem('sw_tab_session_ok') === '1';
        if (sameTabOk && data && data.ok && data.player_id) {
          enterWorld(data.player_id, data.glyph || '');
          return;
        }
        if (savedId && pwInput) {
          try { pwInput.focus(); } catch (_) {}
        }
      })
      .catch(() => {});

    let authBusy = false;
    form.addEventListener('submit', async (e) => {
      e.preventDefault();
      if (authBusy) return;
      const errEl = $('auth-message');
      if (errEl) errEl.textContent = '';
      const id = (idInput?.value || '').trim();
      const password = pwInput?.value || '';
      if (!id || !password) {
        if (errEl) errEl.textContent = '請填寫 ID 與密碼';
        return;
      }
      const isCreate =
        (e.submitter && e.submitter.value === 'create') ||
        !$('auth-create-actions').hidden;
      const displayChar = ($('auth-display-char')?.value || '').trim();
      const gender = form.querySelector('input[name="gender"]:checked')?.value || '男';

      authBusy = true;
      try {
        const res = await fetch('/api/rim/auth', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          credentials: 'same-origin',
          body: JSON.stringify({
            action: isCreate ? 'create' : 'login',
            player_id: id,
            password,
            display_char: isCreate ? displayChar : '',
            gender: isCreate ? gender : '',
          }),
        });
        const data = await res.json().catch(() => null);
        if (!res.ok || !data || !data.ok) {
          if (errEl) errEl.textContent = (data && data.msg) || '登入失敗，請稍後再試';
          return;
        }
        if (data.sky_time && window.SwSkyTime) {
          window.SwSkyTime.syncFromServer(data.sky_time);
        }
        sessionStorage.setItem('sw_tab_session_ok', '1');
        enterWorld(data.player_id || id, data.glyph || displayChar || id.slice(0, 1));
      } catch (_) {
        if (errEl) errEl.textContent = '連線暫時不通，請再試一次';
      } finally {
        authBusy = false;
      }
    });
  }

  window.addEventListener('resize', resize);
  setupAuthGate();
})();
