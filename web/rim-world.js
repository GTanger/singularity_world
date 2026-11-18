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

  // ─ 宣紙白描簡約俯視素材與確定性動態系統 (對齊 pawn-demo v8 標準) ─
  const PAWN_ASSET_VER = '20261118a19';
  const PAWN_ASSET_FILES = {
    hat: `assets/pawn/m1_1_hat.png?v=${PAWN_ASSET_VER}`,
    head: `assets/pawn/m1_2_head.png?v=${PAWN_ASSET_VER}`,
    pack: `assets/pawn/m1_4_pack.png?v=${PAWN_ASSET_VER}`,
    bootTop: `assets/pawn/m2_5_boot_top.png?v=${PAWN_ASSET_VER}`,
    bamboo: `assets/pawn/m1_8_bamboo.png?v=${PAWN_ASSET_VER}`,
    rock: `assets/pawn/m1_9_rock.png?v=${PAWN_ASSET_VER}`,
    campfire: `assets/pawn/obj_campfire.png?v=${PAWN_ASSET_VER}`,
    workbench: `assets/pawn/obj_workbench.png?v=${PAWN_ASSET_VER}`,
    stockpile: `assets/pawn/obj_stockpile.png?v=${PAWN_ASSET_VER}`,
    bridge: `assets/pawn/obj_bridge.png?v=${PAWN_ASSET_VER}`,
    post: `assets/pawn/obj_post.png?v=${PAWN_ASSET_VER}`,
    shoot: `assets/pawn/obj_shoot.png?v=${PAWN_ASSET_VER}`,
    itemShoot: `assets/pawn/item_shoot.png?v=${PAWN_ASSET_VER}`,
    itemBamboo: `assets/pawn/item_bamboo.png?v=${PAWN_ASSET_VER}`,
    itemMeat: `assets/pawn/item_meat.png?v=${PAWN_ASSET_VER}`,
    itemRoastShoot: `assets/pawn/item_roast_shoot.png?v=${PAWN_ASSET_VER}`,
    itemRoastMeat: `assets/pawn/item_roast_meat.png?v=${PAWN_ASSET_VER}`,
    itemBlade: `assets/pawn/item_blade.png?v=${PAWN_ASSET_VER}`,
    itemPick: `assets/pawn/item_pick.png?v=${PAWN_ASSET_VER}`,
    boar: `assets/pawn/beast_boar.png?v=${PAWN_ASSET_VER}`,
    pheasant: `assets/pawn/beast_pheasant.png?v=${PAWN_ASSET_VER}`,
  };
  const pawnImgs = {};
  let pawnLoadedCount = 0;
  let selectedSkillIdx = 0;
  const pawnTotalAssets = Object.keys(PAWN_ASSET_FILES).length;
  for (const [key, url] of Object.entries(PAWN_ASSET_FILES)) {
    const im = new Image();
    im.onload = () => {
      pawnLoadedCount++;
      if (pawnLoadedCount === pawnTotalAssets) window.__swPawnAssetsReady = true;
    };
    im.onerror = () => {
      pawnLoadedCount++;
      if (pawnLoadedCount === pawnTotalAssets) window.__swPawnAssetsReady = true;
    };
    im.src = url;
    pawnImgs[key] = im;
  }

  function isDrawableReady(img) {
    return Boolean(
      img &&
        ((img.complete && img.naturalWidth > 0) ||
          (typeof HTMLCanvasElement !== 'undefined' && img instanceof HTMLCanvasElement && img.width > 0))
    );
  }

  function makeBaimiaoSprite(drawFn) {
    const c = document.createElement('canvas');
    c.width = 128;
    c.height = 128;
    const g = c.getContext('2d');
    if (g) {
      g.save();
      g.translate(64, 64);
      drawFn(g);
      g.restore();
    }
    return c;
  }

  // 五張 128×128 離屏手繪白描淡彩植物與藥薯圖素（取代粗劣小畫家感圖塊）
  pawnImgs.wildHerb = makeBaimiaoSprite((g) => {
    g.lineJoin = 'round';
    g.lineCap = 'round';
    for (let i = 0; i < 7; i++) {
      const ang = (i * Math.PI * 2) / 7 + (i % 2 ? 0.14 : -0.1);
      g.save();
      g.rotate(ang);
      g.fillStyle = i % 2 === 0 ? '#507a64' : '#6c957b';
      g.strokeStyle = '#332620';
      g.lineWidth = 3.2;
      g.beginPath();
      g.moveTo(0, 0);
      g.quadraticCurveTo(-13, -18, -7, -34);
      g.quadraticCurveTo(0, -47, 0, -49);
      g.quadraticCurveTo(0, -47, 7, -34);
      g.quadraticCurveTo(13, -18, 0, 0);
      g.fill();
      g.stroke();
      g.strokeStyle = 'rgba(232, 226, 208, 0.55)';
      g.lineWidth = 1.6;
      g.beginPath();
      g.moveTo(0, -6);
      g.lineTo(0, -40);
      g.stroke();
      g.restore();
    }
    for (const [fx, fy, fr] of [[-11, -13, 6.2], [12, -8, 5.6], [-2, 13, 5.8], [9, 10, 4.8]]) {
      g.fillStyle = '#f2eaf5';
      g.strokeStyle = '#332620';
      g.lineWidth = 2.4;
      g.beginPath();
      g.arc(fx, fy, fr, 0, Math.PI * 2);
      g.fill();
      g.stroke();
      g.fillStyle = '#cfa048';
      g.beginPath();
      g.arc(fx, fy, fr * 0.38, 0, Math.PI * 2);
      g.fill();
    }
  });

  pawnImgs.itemHerb = makeBaimiaoSprite((g) => {
    g.lineJoin = 'round';
    g.lineCap = 'round';
    g.rotate(-0.36);
    for (const [ang, len, col] of [[-0.30, 42, '#4d7660'], [0.02, 46, '#658f74'], [0.32, 41, '#58826a']]) {
      g.save();
      g.rotate(ang);
      g.fillStyle = col;
      g.strokeStyle = '#332620';
      g.lineWidth = 3.2;
      g.beginPath();
      g.moveTo(0, 26);
      g.quadraticCurveTo(-13, 0, -7, -len + 8);
      g.quadraticCurveTo(0, -len, 7, -len + 8);
      g.quadraticCurveTo(13, 0, 0, 26);
      g.fill();
      g.stroke();
      g.restore();
    }
    // 束藥麻繩
    g.fillStyle = '#bf9c6a';
    g.strokeStyle = '#332620';
    g.lineWidth = 3;
    g.beginPath();
    g.roundRect(-14, 14, 28, 12, 4);
    g.fill();
    g.stroke();
    // 藥花
    g.fillStyle = '#f4ecf7';
    g.strokeStyle = '#332620';
    g.lineWidth = 2.4;
    for (const [fx, fy] of [[-12, -32], [6, -38], [16, -26]]) {
      g.beginPath();
      g.arc(fx, fy, 6, 0, Math.PI * 2);
      g.fill();
      g.stroke();
    }
  });

  pawnImgs.itemSalve = makeBaimiaoSprite((g) => {
    g.lineJoin = 'round';
    g.fillStyle = '#d9dfc9';
    g.strokeStyle = '#332620';
    g.lineWidth = 3.8;
    g.beginPath();
    g.arc(0, 2, 34, 0, Math.PI * 2);
    g.fill();
    g.stroke();
    // 翡翠綠藥膏泥
    g.fillStyle = '#4e785d';
    g.beginPath();
    g.arc(0, 2, 23, 0, Math.PI * 2);
    g.fill();
    g.stroke();
    g.fillStyle = 'rgba(214, 235, 212, 0.42)';
    g.beginPath();
    g.ellipse(-7, -6, 9, 4.5, -0.5, 0, Math.PI * 2);
    g.fill();
    // 硃砂藥封與木塞紐
    g.fillStyle = '#8b2020';
    g.strokeStyle = '#332620';
    g.lineWidth = 2.4;
    g.beginPath();
    g.roundRect(14, -28, 18, 22, 4);
    g.fill();
    g.stroke();
  });

  pawnImgs.itemTuber = makeBaimiaoSprite((g) => {
    g.lineJoin = 'round';
    g.rotate(-0.42);
    g.fillStyle = '#9f5747';
    g.strokeStyle = '#332620';
    g.lineWidth = 3.8;
    g.beginPath();
    g.moveTo(-40, 0);
    g.bezierCurveTo(-24, -24, 20, -26, 40, -2);
    g.bezierCurveTo(22, 25, -22, 24, -40, 0);
    g.closePath();
    g.fill();
    g.stroke();
    g.fillStyle = '#bc7361';
    g.beginPath();
    g.ellipse(-6, -8, 18, 6.5, -0.05, 0, Math.PI * 2);
    g.fill();
    g.strokeStyle = '#332620';
    g.lineWidth = 2.2;
    g.beginPath();
    g.moveTo(40, -2);
    g.quadraticCurveTo(49, -6, 52, -1);
    g.stroke();
  });

  pawnImgs.itemRoastTuber = makeBaimiaoSprite((g) => {
    g.lineJoin = 'round';
    g.rotate(-0.35);
    g.fillStyle = '#68392c';
    g.strokeStyle = '#332620';
    g.lineWidth = 3.8;
    g.beginPath();
    g.moveTo(-40, 2);
    g.bezierCurveTo(-22, -24, 20, -25, 40, 0);
    g.bezierCurveTo(22, 26, -22, 24, -40, 2);
    g.closePath();
    g.fill();
    g.stroke();
    // 裂開的金黃烤薯肉
    g.fillStyle = '#e5aa48';
    g.beginPath();
    g.ellipse(0, -1, 22, 11, 0, 0, Math.PI * 2);
    g.fill();
    g.stroke();
    g.fillStyle = '#f6d47d';
    g.beginPath();
    g.ellipse(-4, -3, 11, 4.5, 0, 0, Math.PI * 2);
    g.fill();
  });

  const PAWN_PAL = {
    ink: '#332620',
    sleeve: '#999ea2',
    skin: '#f3d9c3',
    bracer: '#8e776b',
    glove: '#70594d',
    pants: '#535879',
    wood: '#d9c0a3',
    iron: '#6e7072',
    steel: '#d6d8d9',
    bambooBody: '#ddd0ad',
    bambooChar: '#5a432b',
    bambooSpine: '#685038',
  };
  const VEST_PALETTES = [
    ['#8f705b', '#6f5443'], // 茶褐防刮皮背心
    ['#62717f', '#45525e'], // 青黛粗布背心
    ['#767f68', '#565e4b'], // 竹青艾褐短褂
    ['#946b57', '#6e4c3c'], // 赭石獵裝背心
    ['#7b7365', '#595246'], // 亞麻灰褐布衣
  ];
  const CHOP_DURATION = 0.52;
  const WORKING_ACT_RE = /^(親手採伐|採伐|親手築造|築造|拆除|打造|製作|烹製|赴火|拆招|迎擊|上前交鋒|揮鎬翻土|翻妥|播種|扦插|採收|採挖|搗藥|搗製|敷藥|赴火炙)/;

  let cellMap = new Map();
  let actors = [];
  let meDetail = null;
  let worldWeather = { code: 'clear', label: '晴風和暢', raining: false };
  let worldRooms = [];
  let indoorCellSet = new Set();
  const roomRoofAlpha = new Map();
  let bedBadgeQueue = [];
  let renderActors = new Map(); // id -> { x, y, srvX, srvY, facing, targetFacing, speed, running, path, walkPhase, moveBlend, runBlend, chopTimer, chopDidImpact, lastPop }
  let meMoveSeq = 0;
  let meMoveLockUntil = 0;
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
      gearEl.textContent = `持：${meDetail.equipped_label || '空手'}`;
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
              st.srvX = a.px;
              st.srvY = a.py;
              if (!a.is_player || performance.now() >= meMoveLockUntil) {
                st.speed = a.speed || 1.35;
                st.running = !!a.running;
                st.path = srvPath.length > 0 ? trimPassedWaypoints(srvPath, st.x, st.y) : [];
              }
              if (typeof a.facing === 'number') st.targetFacing = a.facing;
              const err = Math.hypot(a.px - st.x, a.py - st.y);
              if (err > 2.8) {
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
          await sendCmd({
            cmd: s.cmd,
            target_actor: s.target_actor,
            directive: s.directive,
          });
          lastInspectorSig = '';
          pollState(true);
        } else if (s.action) {
          if (['chop', 'build', 'craft_blade', 'craft_pick', 'craft_hat', 'craft_pack', 'cook_shoot', 'cook_meat', 'drink', 'attack_actor'].includes(s.action)) {
            const me = actors.find((x) => x.is_player);
            const meSt = me ? renderActors.get(me.id) : null;
            if (me && meSt && Math.hypot(me.px - sel.mx, me.py - sel.my) <= 1.55) {
              meSt.targetFacing = Math.atan2(sel.my - meSt.y, sel.mx - meSt.x);
              meSt.chopTimer = CHOP_DURATION;
              meSt.chopDidImpact = false;
            }
          }
          const r = await sendCmd({
            cmd: 'act',
            mx: sel.mx,
            my: sel.my,
            action: s.action === 'attack_actor' ? 'move' : s.action,
            target_actor: s.target_actor,
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

  function closeModal() {
    activeModal = null;
    lastModalSig = '';
    const bd = $('rim-modal-backdrop');
    if (bd) bd.hidden = true;
  }

  function openModal(kind) {
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
      window.SwPortrait.render(cv, {
        mode: 'full',
        width: 136,
        height: 206,
        id: d.id || 1,
        name: d.name || '自身',
        glyph: d.glyph || (d.name ? String(d.name).slice(0, 1) : '我'),
        gender: d.gender || 'M',
        soul_seed: d.soul_seed || '0',
        wearHat: d.wear_hat !== false,
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
      byokLbl.textContent = '生圖與對話金鑰（選填）';
      const byokMeta = document.createElement('div');
      byokMeta.style.cssText = 'font-size:11.5px;color:#6b5a47;line-height:1.4;';
      byokMeta.textContent = '指定 OpenRouter 繪圖模型：google/gemini-3.1-flash-image';
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
      const btnRegen = document.createElement('button');
      btnRegen.type = 'button';
      btnRegen.className = 'rim-chip-btn';
      btnRegen.style.padding = '5px 10px';
      btnRegen.textContent = '重繪人像';

      const syncByokLabel = (info) => {
        if (!info || !info.ok) {
          byokState.textContent = '目前狀態：未設定個人金鑰';
          return;
        }
        hasKeyConfigured = Boolean(info.has_key);
        byokState.textContent = hasKeyConfigured
          ? '目前狀態：已設定個人金鑰'
          : '目前狀態：未設定個人金鑰（使用預設繪製）';
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
      btnRegen.onclick = async () => {
        try {
          await fetch('/api/rim/portrait/regen', {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ reset_base: true }),
          });
          if (window.SwPortrait) window.SwPortrait.clearCache();
          showToast('已開始重新繪製人像');
        } catch (_) {
          showToast('重繪請求失敗');
        }
      };

      byokBtns.append(btnSaveKey, btnClearKey, btnRegen);
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
        const lbl = document.createElement('span');
        lbl.textContent = `對方現貨：${it.label} × ${it.count}`;
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'rim-socket-btn';
        btn.disabled = !it.can_afford;
        btn.textContent = `換取（${it.price_mg} 鎂）`;
        btn.onclick = () => doTrade(it.code, 'buy');
        row.append(lbl, btn);
        tradeCol.appendChild(row);
      }
      for (const it of sellable) {
        const row = document.createElement('div');
        row.className = 'rim-inv-row';
        const lbl = document.createElement('span');
        lbl.textContent = `我方行囊：${it.label} × ${it.count}`;
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'rim-chip-btn';
        btn.style.cssText = 'padding:4px 9px;font-size:12px;';
        btn.disabled = !it.npc_can_pay;
        btn.textContent = `讓出（得 ${it.price_mg} 鎂）`;
        btn.onclick = () => doTrade(it.code, 'sell');
        row.append(lbl, btn);
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
      const heldName = document.createElement('strong');
      heldName.textContent = `手持：${d.equipped_label || '空手'}`;
      heldRow.appendChild(heldName);

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
          const lbl = document.createElement('span');
          lbl.textContent = `${item.label} × ${item.count}`;
          row.appendChild(lbl);

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
    if (c.s === 'wall' || c.s === 'workbench' || c.s === 'campfire') return false;
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

  function resolveItemSprite(item) {
    const ch = String(item.c || '');
    const lbl = String(item.label || '');
    if (ch === '藥' || /青草藥/.test(lbl)) return pawnImgs.itemHerb;
    if (ch === '膏' || /藥膏/.test(lbl)) return pawnImgs.itemSalve;
    if (ch === '烤' || /烤甘薯/.test(lbl)) return pawnImgs.itemRoastTuber;
    if (ch === '薯' || /甘薯/.test(lbl)) return pawnImgs.itemTuber;
    if (ch === '笠' || /竹笠/.test(lbl)) return pawnImgs.hat;
    if (ch === '簍' || /背簍/.test(lbl)) return pawnImgs.pack;
    if (ch === '煨' || /煨/.test(lbl)) return pawnImgs.itemRoastShoot;
    if (ch === '炙' || /炙/.test(lbl)) return pawnImgs.itemRoastMeat;
    if (ch === '竹' || (/竹/.test(lbl) && !/刀|筍|笠|簍/.test(lbl))) return pawnImgs.itemBamboo;
    if (ch === '筍' || /筍/.test(lbl)) return pawnImgs.itemShoot;
    if (ch === '石' || /石/.test(lbl)) return pawnImgs.rock;
    if (ch === '肉' || /肉/.test(lbl)) return pawnImgs.itemMeat;
    if (ch === '刀' || ch === '刃' || /刀/.test(lbl)) return pawnImgs.itemBlade;
    if (ch === '斧' || ch === '鎬' || /斧|鎬/.test(lbl)) return pawnImgs.itemPick;
    return pawnImgs.itemBamboo;
  }

  // 地上與儲物席之宣紙白描物品圖素堆疊（去方框貼紙，僅在複數時於角邊低調標示 ×數量）
  function drawTopDownItem(item, cx, cy, sizePx) {
    const img = resolveItemSprite(item);
    const n = Math.max(1, Number(item.n) || 1);
    const ch = String(item.c || '');
    const layers = ch === '石' ? Math.min(n, 3) : Math.min(n, 2);
    const spritePx = Math.max(13, sizePx * (layers > 1 ? 0.72 : 0.80));

    ctx.save();
    ctx.translate(cx, cy);

    // 淡柔接地宣紙水影
    ctx.fillStyle = 'rgba(44, 30, 15, 0.08)';
    ctx.beginPath();
    ctx.ellipse(0, sizePx * 0.04, spritePx * 0.42, spritePx * 0.30, 0, 0, Math.PI * 2);
    ctx.fill();

    if (isDrawableReady(img)) {
      if (layers === 1) {
        const tilt = ch === '石' ? 0.25 : 0.0;
        drawPawnPart(img, 0, 0, spritePx, spritePx, 0.50, 0.50, tilt);
      } else {
        const offsets = layers === 2
          ? [[-0.08, 0.05, -0.14], [0.07, -0.05, 0.11]]
          : [[-0.12, 0.07, -0.22], [0.11, 0.05, 0.24], [0.0, -0.09, 0.05]];
        for (const [ox, oy, rot] of offsets) {
          drawPawnPart(img, ox * sizePx, oy * sizePx, spritePx * 0.92, spritePx * 0.92, 0.50, 0.50, rot);
        }
      }
    } else {
      ctx.fillStyle = '#c8b496';
      ctx.strokeStyle = PAWN_PAL.ink;
      ctx.lineWidth = 1.1;
      ctx.beginPath();
      ctx.arc(0, 0, spritePx * 0.30, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }

    if (n > 1 && sizePx >= 17) {
      const txt = `×${n}`;
      ctx.font = 'bold 10px "Noto Sans TC", sans-serif';
      ctx.textAlign = 'right';
      ctx.textBaseline = 'bottom';
      ctx.lineJoin = 'round';
      ctx.lineWidth = 3.2;
      ctx.strokeStyle = 'rgba(244, 237, 224, 0.95)';
      ctx.strokeText(txt, sizePx * 0.38, sizePx * 0.38);
      ctx.fillStyle = '#2c1e0f';
      ctx.fillText(txt, sizePx * 0.38, sizePx * 0.38);
    }
    ctx.restore();
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
    ctx.save();
    ctx.globalAlpha = alpha;

    const halfW = Math.min(40, Math.ceil(viewportW / cellPx / 2) + 2);
    const halfH = Math.min(40, Math.ceil(viewportH / cellPx / 2) + 2);
    const cx = Math.round(camX);
    const cy = Math.round(camY);

    // 鋪設宣紙白描暖底草坪基底（無格線連續地表）
    const tl = microToScreen(cx - halfW - 0.5, cy + halfH + 0.5);
    const spanW = (halfW * 2 + 1) * cellPx;
    const spanH = (halfH * 2 + 1) * cellPx;
    ctx.fillStyle = '#efe8d8';
    ctx.fillRect(tl.x, tl.y, spanW, spanH);

    // Pass 1：純地形層（草地、赭土、卵石砂灘、水系與水岸墨線），避免後畫地形覆蓋營火暖暈
    for (let my = cy - halfH; my <= cy + halfH; my++) {
      for (let mx = cx - halfW; mx <= cx + halfW; mx++) {
        const p = microToScreen(mx, my);
        const x0 = p.x - cellPx / 2;
        const y0 = p.y - cellPx / 2;
        const c = cellMap.get(`${mx},${my}`);
        const g = c ? c.g : 'grass';
        const h = hash2d(mx, my);

        const pad = 0.45;
        const rx = x0 - pad;
        const ry = y0 - pad;
        const rw = cellPx + pad * 2;
        const rh = cellPx + pad * 2;

        if (g === 'grass') {
          if (cellPx >= 16 && (h % 11) < 2 && (!c || (!c.s && !c.p))) {
            const ox = (((h >>> 4) % 9) - 4) * 0.05 * cellPx;
            const oy = (((h >>> 8) % 9) - 4) * 0.05 * cellPx;
            ctx.strokeStyle = 'rgba(60, 48, 34, 0.18)';
            ctx.lineWidth = 1.1;
            ctx.beginPath();
            ctx.moveTo(p.x + ox, p.y + oy);
            ctx.lineTo(p.x + ox - 3, p.y + oy - 3);
            ctx.moveTo(p.x + ox, p.y + oy);
            ctx.lineTo(p.x + ox + 3, p.y + oy - 2);
            ctx.stroke();
          }
        } else if (g === 'red') {
          ctx.fillStyle = '#e5d3bd';
          ctx.fillRect(rx, ry, rw, rh);
          if (cellPx >= 18 && (h % 4) === 0 && (!c || !c.s)) {
            ctx.fillStyle = 'rgba(60, 48, 34, 0.24)';
            ctx.beginPath();
            ctx.arc(p.x - cellPx * 0.18, p.y + cellPx * 0.10, 1.3, 0, Math.PI * 2);
            ctx.arc(p.x + cellPx * 0.20, p.y - cellPx * 0.14, 1.1, 0, Math.PI * 2);
            ctx.fill();
          }
        } else if (g === 'pebble') {
          ctx.fillStyle = '#f4ede0';
          ctx.fillRect(rx, ry, rw, rh);
          if (cellPx >= 18 && (h % 5) === 0 && (!c || !c.s)) {
            ctx.fillStyle = '#e6ddd0';
            ctx.strokeStyle = 'rgba(60, 48, 34, 0.24)';
            ctx.lineWidth = 0.9;
            ctx.beginPath();
            ctx.ellipse(p.x - cellPx * 0.16, p.y - cellPx * 0.08, cellPx * 0.10, cellPx * 0.07, 0.4, 0, Math.PI * 2);
            ctx.fill();
            ctx.stroke();
            ctx.beginPath();
            ctx.ellipse(p.x + cellPx * 0.18, p.y + cellPx * 0.16, cellPx * 0.07, cellPx * 0.05, -0.3, 0, Math.PI * 2);
            ctx.fill();
            ctx.stroke();
          }
        } else if (isWaterGround(g)) {
          ctx.fillStyle = g === 'spring' ? '#c2d7df' : (g === 'deep' ? '#9ebbc8' : '#d8e4e7');
          ctx.fillRect(rx, ry, rw, rh);

          // 手繪宣紙水岸（濕沙柔暈 + 淡焦墨有機邊線）
          if (!c || c.s !== 'bridge') {
            const gN = cellMap.get(`${mx},${my + 1}`)?.g || 'grass';
            const gS = cellMap.get(`${mx},${my - 1}`)?.g || 'grass';
            const gW = cellMap.get(`${mx - 1},${my}`)?.g || 'grass';
            const gE = cellMap.get(`${mx + 1},${my}`)?.g || 'grass';
            const bow = (((h % 7) - 3) * 0.022) * cellPx;
            const edges = [];
            if (!isWaterGround(gN)) edges.push([x0, y0, x0 + cellPx, y0, 0, bow]);
            if (!isWaterGround(gS)) edges.push([x0, y0 + cellPx, x0 + cellPx, y0 + cellPx, 0, -bow]);
            if (!isWaterGround(gW)) edges.push([x0, y0, x0, y0 + cellPx, bow, 0]);
            if (!isWaterGround(gE)) edges.push([x0 + cellPx, y0, x0 + cellPx, y0 + cellPx, -bow, 0]);
            if (edges.length > 0) {
              ctx.save();
              ctx.lineCap = 'round';
              ctx.strokeStyle = 'rgba(154, 138, 114, 0.28)';
              ctx.lineWidth = Math.max(2.0, cellPx * 0.085);
              ctx.beginPath();
              for (const [ax, ay, bx, by, ox, oy] of edges) {
                ctx.moveTo(ax, ay);
                ctx.quadraticCurveTo((ax + bx) * 0.5 + ox, (ay + by) * 0.5 + oy, bx, by);
              }
              ctx.stroke();
              ctx.strokeStyle = 'rgba(61, 48, 35, 0.62)';
              ctx.lineWidth = 1.05;
              ctx.stroke();
              ctx.restore();
            }
          }

          if (cellPx >= 14 && (h % 3) !== 0) {
            const wave = Math.sin(nowMs * 0.0028 + mx * 0.8 + my * 0.5) * (cellPx * 0.09);
            ctx.strokeStyle = 'rgba(60, 74, 84, 0.30)';
            ctx.lineWidth = 1.05;
            ctx.lineCap = 'round';
            ctx.beginPath();
            ctx.moveTo(p.x - cellPx * 0.24, p.y + wave * 0.35);
            ctx.quadraticCurveTo(p.x - cellPx * 0.06, p.y - cellPx * 0.06, p.x + cellPx * 0.12, p.y + wave * 0.15);
            ctx.quadraticCurveTo(p.x + cellPx * 0.20, p.y + cellPx * 0.05, p.x + cellPx * 0.28, p.y - wave * 0.30);
            ctx.stroke();
          }
        } else if (g === 'cliff') {
          ctx.fillStyle = '#c8bfb2';
          ctx.fillRect(rx, ry, rw, rh);
          if (cellPx >= 14) {
            ctx.fillStyle = 'rgba(168, 158, 144, 0.36)';
            ctx.beginPath();
            ctx.moveTo(x0 + cellPx * 0.12, y0 + cellPx * 0.32);
            ctx.lineTo(x0 + cellPx * 0.58, y0 + cellPx * 0.14);
            ctx.lineTo(x0 + cellPx * 0.84, y0 + cellPx * 0.56);
            ctx.lineTo(x0 + cellPx * 0.36, y0 + cellPx * 0.78);
            ctx.closePath();
            ctx.fill();
            ctx.strokeStyle = '#4a3e33';
            ctx.lineWidth = 1.1;
            ctx.stroke();
          }
        }
      }
    }

    bedBadgeQueue = [];

    // 單平面圍合室內：於底層土地之上鋪設溫潤竹席地板與內牆細簷陰影
    const effWeather = getEffectiveWeather();
    for (const rm of worldRooms) {
      if (!Array.isArray(rm.cells) || !rm.cells.length) continue;
      for (const pt of rm.cells) {
        const ix = pt[0];
        const iy = pt[1];
        const p = microToScreen(ix, iy);
        if (p.x < -cellPx || p.x > viewportW + cellPx || p.y < -cellPx || p.y > viewportH + cellPx) continue;
        const x0 = p.x - cellPx / 2;
        const y0 = p.y - cellPx / 2;
        ctx.fillStyle = '#f0e3ca';
        ctx.fillRect(x0 - 0.35, y0 - 0.35, cellPx + 0.7, cellPx + 0.7);
        if (cellPx >= 15) {
          ctx.strokeStyle = 'rgba(128, 96, 62, 0.14)';
          ctx.lineWidth = 0.9;
          ctx.beginPath();
          ctx.moveTo(x0 + cellPx * 0.25, y0);
          ctx.lineTo(x0 + cellPx * 0.25, y0 + cellPx);
          ctx.moveTo(x0 + cellPx * 0.75, y0);
          ctx.lineTo(x0 + cellPx * 0.75, y0 + cellPx);
          ctx.moveTo(x0, y0 + cellPx * 0.5);
          ctx.lineTo(x0 + cellPx, y0 + cellPx * 0.5);
          ctx.stroke();
        }
        // 北／西內牆內側細微簷蔭
        if (iy === rm.y1 - 1) {
          ctx.fillStyle = 'rgba(44, 30, 15, 0.08)';
          ctx.fillRect(x0, y0, cellPx, cellPx * 0.24);
        }
        if (ix === rm.x0 + 1) {
          ctx.fillStyle = 'rgba(44, 30, 15, 0.06)';
          ctx.fillRect(x0, y0, cellPx * 0.20, cellPx);
        }
      }
    }

    // Pass 2：營火暖暈（先暈染於全數地面之上，絕不被鄰格切角）、建成設施、物資籤及中景簡約物
    for (let my = cy - halfH; my <= cy + halfH; my++) {
      for (let mx = cx - halfW; mx <= cx + halfW; mx++) {
        const c = cellMap.get(`${mx},${my}`);
        if (!c || c.s !== 'campfire') continue;
        const p = microToScreen(mx, my);
        const flicker = 1 + Math.sin(nowMs * 0.012) * 0.09;
        const wetCamp = effWeather.raining && !indoorCellSet.has(`${mx},${my}`);
        const rad = cellPx * (wetCamp ? 0.92 : 1.65) * flicker;
        const grad = ctx.createRadialGradient(p.x, p.y, 2, p.x, p.y, rad);
        grad.addColorStop(0, wetCamp ? 'rgba(208, 102, 34, 0.14)' : 'rgba(218, 118, 36, 0.24)');
        grad.addColorStop(1, 'rgba(218, 118, 36, 0)');
        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.arc(p.x, p.y, rad, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    for (let my = cy - halfH; my <= cy + halfH; my++) {
      for (let mx = cx - halfW; mx <= cx + halfW; mx++) {
        const c = cellMap.get(`${mx},${my}`);
        if (!c) continue;
        const p = microToScreen(mx, my);
        const x0 = p.x - cellPx / 2;
        const y0 = p.y - cellPx / 2;

        if (c.s === 'stockpile') {
          const matImg = pawnImgs.stockpile;
          if (matImg && matImg.complete && matImg.naturalWidth) {
            ctx.save();
            ctx.globalAlpha = 0.88;
            const hMat = hash2d(mx, my);
            drawPawnPart(matImg, p.x, p.y, cellPx * 0.94, cellPx * 0.94, 0.50, 0.50, ((hMat % 4) * Math.PI) / 2);
            ctx.restore();
          } else {
            ctx.fillStyle = 'rgba(218, 198, 158, 0.34)';
            ctx.fillRect(x0 + 2, y0 + 2, cellPx - 4, cellPx - 4);
            ctx.strokeStyle = '#7e6645';
            ctx.lineWidth = 1.15;
            ctx.strokeRect(x0 + 2.5, y0 + 2.5, cellPx - 5, cellPx - 5);
          }
        } else if (c.s === 'bridge') {
          const spanNS = hasBridgeAt(mx, my + 1) || hasBridgeAt(mx, my - 1) || !(hasBridgeAt(mx - 1, my) || hasBridgeAt(mx + 1, my));
          const brImg = pawnImgs.bridge;
          ctx.save();
          ctx.translate(p.x, p.y);
          if (!spanNS) ctx.rotate(Math.PI / 2);
          ctx.fillStyle = 'rgba(44, 30, 15, 0.14)';
          ctx.fillRect(-cellPx * 0.44 + 2, -cellPx * 0.52 + 2, cellPx * 0.88, cellPx * 1.04);
          if (brImg && brImg.complete && brImg.naturalWidth) {
            drawPawnPart(brImg, 0, 0, cellPx * 0.98, cellPx * 1.05, 0.50, 0.50, 0);
          } else {
            ctx.fillStyle = '#dec6a3';
            ctx.fillRect(-cellPx * 0.45, -cellPx * 0.5, cellPx * 0.90, cellPx);
          }
          ctx.restore();
        } else if (c.s === 'wall') {
          const nConn = hasWallAt(mx, my + 1);
          const sConn = hasWallAt(mx, my - 1);
          const wConn = hasWallAt(mx - 1, my);
          const eConn = hasWallAt(mx + 1, my);
          const thick = cellPx * 0.46;
          const halfT = thick / 2;

          // 牆簷水暈陰影
          ctx.fillStyle = 'rgba(44, 30, 15, 0.14)';
          ctx.fillRect(p.x - halfT + 2.2, p.y - halfT + 2.8, thick, thick);
          if (nConn) ctx.fillRect(p.x - halfT + 2.2, y0 + 2, thick, cellPx / 2);
          if (sConn) ctx.fillRect(p.x - halfT + 2.2, p.y + 2.8, thick, cellPx / 2 - 1);
          if (wConn) ctx.fillRect(x0 + 2, p.y - halfT + 2.8, cellPx / 2, thick);
          if (eConn) ctx.fillRect(p.x + 2.2, p.y - halfT + 2.8, cellPx / 2 - 1, thick);

          // 雙層竹編夯土底脊
          ctx.fillStyle = '#cbb693';
          ctx.fillRect(p.x - halfT, p.y - halfT, thick, thick);
          if (nConn) ctx.fillRect(p.x - halfT, y0 - 0.5, thick, cellPx / 2 + 0.5);
          if (sConn) ctx.fillRect(p.x - halfT, p.y, thick, cellPx / 2 + 0.5);
          if (wConn) ctx.fillRect(x0 - 0.5, p.y - halfT, cellPx / 2 + 0.5, thick);
          if (eConn) ctx.fillRect(p.x, p.y - halfT, cellPx / 2 + 0.5, thick);

          const ridgeT = thick * 0.48;
          const halfR = ridgeT / 2;
          ctx.fillStyle = '#e2d3b5';
          ctx.fillRect(p.x - halfR, p.y - halfR, ridgeT, ridgeT);
          if (nConn) ctx.fillRect(p.x - halfR, y0 - 0.5, ridgeT, cellPx / 2 + 0.5);
          if (sConn) ctx.fillRect(p.x - halfR, p.y, ridgeT, cellPx / 2 + 0.5);
          if (wConn) ctx.fillRect(x0 - 0.5, p.y - halfR, cellPx / 2 + 0.5, ridgeT);
          if (eConn) ctx.fillRect(p.x, p.y - halfR, cellPx / 2 + 0.5, ridgeT);

          // 開放外緣焦墨輪廓（相鄰牆體無縫融合）
          ctx.strokeStyle = PAWN_PAL.ink;
          ctx.lineWidth = 1.25;
          ctx.beginPath();
          if (!nConn) {
            ctx.moveTo(p.x - halfT, p.y - halfT);
            ctx.lineTo(p.x + halfT, p.y - halfT);
          } else {
            ctx.moveTo(p.x - halfT, y0);
            ctx.lineTo(p.x - halfT, p.y - halfT);
            ctx.moveTo(p.x + halfT, y0);
            ctx.lineTo(p.x + halfT, p.y - halfT);
          }
          if (!sConn) {
            ctx.moveTo(p.x - halfT, p.y + halfT);
            ctx.lineTo(p.x + halfT, p.y + halfT);
          } else {
            ctx.moveTo(p.x - halfT, p.y + halfT);
            ctx.lineTo(p.x - halfT, y0 + cellPx);
            ctx.moveTo(p.x + halfT, p.y + halfT);
            ctx.lineTo(p.x + halfT, y0 + cellPx);
          }
          if (!wConn) {
            ctx.moveTo(p.x - halfT, p.y - halfT);
            ctx.lineTo(p.x - halfT, p.y + halfT);
          } else {
            ctx.moveTo(x0, p.y - halfT);
            ctx.lineTo(p.x - halfT, p.y - halfT);
            ctx.moveTo(x0, p.y + halfT);
            ctx.lineTo(p.x - halfT, p.y + halfT);
          }
          if (!eConn) {
            ctx.moveTo(p.x + halfT, p.y - halfT);
            ctx.lineTo(p.x + halfT, p.y + halfT);
          } else {
            ctx.moveTo(p.x + halfT, p.y - halfT);
            ctx.lineTo(x0 + cellPx, p.y - halfT);
            ctx.moveTo(p.x + halfT, p.y + halfT);
            ctx.lineTo(x0 + cellPx, p.y + halfT);
          }
          ctx.stroke();

          // 剖竹內縫線、雙道藤篾縛節 ＋ 轉角原木立柱（obj_post.png）
          if (cellPx >= 15) {
            ctx.strokeStyle = 'rgba(51, 38, 32, 0.45)';
            ctx.lineWidth = 0.8;
            ctx.beginPath();
            if (nConn) {
              ctx.moveTo(p.x - halfR, y0);
              ctx.lineTo(p.x - halfR, p.y - halfR);
              ctx.moveTo(p.x + halfR, y0);
              ctx.lineTo(p.x + halfR, p.y - halfR);
            }
            if (sConn) {
              ctx.moveTo(p.x - halfR, p.y + halfR);
              ctx.lineTo(p.x - halfR, y0 + cellPx);
              ctx.moveTo(p.x + halfR, p.y + halfR);
              ctx.lineTo(p.x + halfR, y0 + cellPx);
            }
            if (wConn) {
              ctx.moveTo(x0, p.y - halfR);
              ctx.lineTo(p.x - halfR, p.y - halfR);
              ctx.moveTo(x0, p.y + halfR);
              ctx.lineTo(p.x - halfR, p.y + halfR);
            }
            if (eConn) {
              ctx.moveTo(p.x + halfR, p.y - halfR);
              ctx.lineTo(x0 + cellPx, p.y - halfR);
              ctx.moveTo(p.x + halfR, p.y + halfR);
              ctx.lineTo(x0 + cellPx, p.y + halfR);
            }
            ctx.stroke();
          }

          if (cellPx >= 16) {
            ctx.strokeStyle = 'rgba(92, 68, 46, 0.58)';
            ctx.lineWidth = 1.05;
            ctx.beginPath();
            for (const f of [0.24, 0.76]) {
              if ((nConn && f < 0.5) || (sConn && f > 0.5)) {
                const ny = y0 + cellPx * f;
                for (const d of [-1.8, 1.8]) {
                  ctx.moveTo(p.x - halfT + 1.2, ny + d);
                  ctx.lineTo(p.x + halfT - 1.2, ny + d);
                }
              }
              if ((wConn && f < 0.5) || (eConn && f > 0.5)) {
                const nx = x0 + cellPx * f;
                for (const d of [-1.8, 1.8]) {
                  ctx.moveTo(nx + d, p.y - halfT + 1.2);
                  ctx.lineTo(nx + d, p.y + halfT - 1.2);
                }
              }
            }
            ctx.stroke();

            const connCount = Number(nConn) + Number(sConn) + Number(wConn) + Number(eConn);
            const isCornerOrEnd = ((nConn || sConn) && (wConn || eConn)) || connCount <= 1;
            if (isCornerOrEnd) {
              const postSz = Math.max(8, thick * 1.18);
              const postImg = pawnImgs.post;
              if (postImg && postImg.complete && postImg.naturalWidth) {
                drawPawnPart(postImg, p.x, p.y, postSz, postSz, 0.50, 0.50, (hash2d(mx, my) % 628) / 100);
              } else {
                ctx.fillStyle = '#8f705b';
                ctx.strokeStyle = PAWN_PAL.ink;
                ctx.lineWidth = 1.05;
                ctx.beginPath();
                ctx.arc(p.x, p.y, Math.max(2.6, thick * 0.34), 0, Math.PI * 2);
                ctx.fill();
                ctx.stroke();
              }
            }
          }
        } else if (c.s === 'door') {
          const vertDoor = hasWallAt(mx, my + 1) || hasWallAt(mx, my - 1);
          const isOpen = isActorNearCell(mx, my, 1.05);
          ctx.save();
          ctx.translate(p.x, p.y);
          if (vertDoor) ctx.rotate(Math.PI / 2);

          // 兩端先鋪與相鄰竹牆同寬（cellPx * 0.46）之牆頭短榫，徹底消除縮頸感
          const wallThick = cellPx * 0.46;
          const halfWT = wallThick / 2;
          const ridgeWT = wallThick * 0.48;
          const stubW = cellPx * 0.16;
          for (const sx of [-1, 1]) {
            const xLeft = sx < 0 ? -cellPx / 2 : cellPx / 2 - stubW;
            ctx.fillStyle = '#cbb693';
            ctx.fillRect(xLeft, -halfWT, stubW, wallThick);
            ctx.fillStyle = '#e2d3b5';
            ctx.fillRect(xLeft, -ridgeWT / 2, stubW, ridgeWT);
            ctx.strokeStyle = PAWN_PAL.ink;
            ctx.lineWidth = 1.25;
            ctx.beginPath();
            ctx.moveTo(xLeft, -halfWT);
            ctx.lineTo(xLeft + stubW, -halfWT);
            ctx.moveTo(xLeft, halfWT);
            ctx.lineTo(xLeft + stubW, halfWT);
            ctx.stroke();
          }

          // 門檻暖石／木踏板
          ctx.fillStyle = 'rgba(218, 198, 166, 0.62)';
          ctx.strokeStyle = 'rgba(92, 68, 46, 0.38)';
          ctx.lineWidth = 0.9;
          ctx.fillRect(-cellPx * 0.44, -cellPx * 0.16, cellPx * 0.88, cellPx * 0.32);
          ctx.strokeRect(-cellPx * 0.44, -cellPx * 0.16, cellPx * 0.88, cellPx * 0.32);

          // 左右兩側原木麻繩門柱（寬度齊平牆脊）
          const postSz = Math.max(8, cellPx * 0.48);
          const postImg = pawnImgs.post;
          for (const sx of [-1, 1]) {
            const jx = sx * (cellPx * 0.5 - postSz * 0.34);
            if (postImg && postImg.complete && postImg.naturalWidth) {
              drawPawnPart(postImg, jx, 0, postSz, postSz, 0.50, 0.50, sx * 0.6);
            } else {
              ctx.fillStyle = '#836348';
              ctx.strokeStyle = PAWN_PAL.ink;
              ctx.lineWidth = 1.1;
              ctx.beginPath();
              ctx.arc(jx, 0, postSz * 0.42, 0, Math.PI * 2);
              ctx.fill();
              ctx.stroke();
            }
          }

          const span = cellPx * 0.68;
          const hingeX = -span * 0.5;
          const leafW = span;
          const leafThick = Math.max(4.5, cellPx * 0.17);
          if (isOpen) {
            ctx.strokeStyle = 'rgba(51, 38, 32, 0.24)';
            ctx.lineWidth = 1;
            ctx.setLineDash([2, 2]);
            ctx.beginPath();
            ctx.arc(hingeX, 0, leafW, -1.25, 0);
            ctx.stroke();
            ctx.setLineDash([]);
          }

          // 竹編三束柴扉門扇
          ctx.save();
          ctx.translate(hingeX, 0);
          ctx.rotate(isOpen ? -1.22 : 0);
          ctx.fillStyle = '#e2ccb0';
          ctx.strokeStyle = PAWN_PAL.ink;
          ctx.lineWidth = 1.15;
          ctx.fillRect(0, -leafThick / 2, leafW, leafThick);
          ctx.strokeRect(0, -leafThick / 2, leafW, leafThick);
          ctx.fillStyle = '#a07e54';
          for (const bx of [0.22, 0.52, 0.80]) {
            ctx.fillRect(leafW * bx - 1.5, -leafThick * 0.55, 3, leafThick * 1.1);
            ctx.strokeRect(leafW * bx - 1.5, -leafThick * 0.55, 3, leafThick * 1.1);
          }
          ctx.restore();
          ctx.restore();
        } else if (c.s === 'campfire') {
          const cfImg = pawnImgs.campfire;
          const flicker = 1 + Math.sin(nowMs * 0.012) * 0.09;
          const wetCamp = effWeather.raining && !indoorCellSet.has(`${mx},${my}`);
          if (cfImg && cfImg.complete && cfImg.naturalWidth) {
            const hCf = hash2d(mx, my);
            drawPawnPart(cfImg, p.x, p.y, cellPx * 1.08, cellPx * 1.08, 0.50, 0.50, ((hCf % 4) * Math.PI) / 2);
            if (wetCamp) {
              ctx.save();
              ctx.fillStyle = 'rgba(62, 54, 48, 0.42)';
              ctx.beginPath();
              ctx.arc(p.x, p.y, cellPx * 0.20, 0, Math.PI * 2);
              ctx.fill();
              ctx.fillStyle = 'rgba(218, 88, 34, 0.46)';
              ctx.beginPath();
              ctx.arc(p.x, p.y, cellPx * 0.08, 0, Math.PI * 2);
              ctx.fill();
              const tSec = nowMs * 0.001;
              for (let sm = 0; sm < 3; sm++) {
                const ph = (tSec * 0.55 + sm * 0.33 + (mx % 5) * 0.1) % 1.0;
                ctx.fillStyle = `rgba(235, 232, 224, ${(1 - ph) * 0.40})`;
                ctx.beginPath();
                ctx.arc(p.x + Math.sin(tSec * 2 + sm) * 3.5, p.y - ph * cellPx * 0.68, cellPx * (0.08 + ph * 0.13), 0, Math.PI * 2);
                ctx.fill();
              }
              ctx.restore();
            } else {
              // 疊加呼吸動態火芯微光（令白描營火自然閃動而不遮掩手繪筆觸）
              ctx.save();
              ctx.globalAlpha = 0.24 + Math.sin(nowMs * 0.014) * 0.08;
              ctx.fillStyle = '#ff9c2a';
              ctx.beginPath();
              ctx.arc(p.x, p.y, cellPx * 0.14 * flicker, 0, Math.PI * 2);
              ctx.fill();
              ctx.globalAlpha = 0.34 + Math.cos(nowMs * 0.019) * 0.10;
              ctx.fillStyle = '#ffe27a';
              ctx.beginPath();
              ctx.arc(p.x, p.y, cellPx * 0.07 * flicker, 0, Math.PI * 2);
              ctx.fill();
              if (cellPx >= 20) {
                const spk1 = Math.sin(nowMs * 0.008 + mx);
                const spk2 = Math.cos(nowMs * 0.011 + my);
                ctx.globalAlpha = 0.55;
                ctx.fillStyle = '#ffbc42';
                ctx.beginPath();
                ctx.arc(p.x + spk1 * cellPx * 0.10, p.y - Math.abs(spk2) * cellPx * 0.18, 1.4, 0, Math.PI * 2);
                ctx.fill();
              }
              ctx.restore();
            }
          } else {
            ctx.fillStyle = wetCamp ? '#9c4b22' : '#c85a1e';
            ctx.beginPath();
            ctx.arc(p.x, p.y, Math.max(3.5, cellPx * (wetCamp ? 0.12 : 0.20) * flicker), 0, Math.PI * 2);
            ctx.fill();
          }
        } else if (c.s === 'workbench') {
          const wbImg = pawnImgs.workbench;
          if (wbImg && wbImg.complete && wbImg.naturalWidth) {
            ctx.fillStyle = 'rgba(44, 30, 15, 0.14)';
            ctx.fillRect(p.x - cellPx * 0.45 + 2, p.y - cellPx * 0.32 + 2.5, cellPx * 0.90, cellPx * 0.64);
            drawPawnPart(wbImg, p.x, p.y, cellPx * 1.08, cellPx * 0.86, 0.50, 0.50, 0);
          } else {
            ctx.fillStyle = '#d9c0a3';
            ctx.strokeStyle = PAWN_PAL.ink;
            ctx.fillRect(p.x - cellPx * 0.42, p.y - cellPx * 0.34, cellPx * 0.84, cellPx * 0.68);
            ctx.strokeRect(p.x - cellPx * 0.42, p.y - cellPx * 0.34, cellPx * 0.84, cellPx * 0.68);
          }
        } else if (c.s === 'bed') {
          drawBambooBed(mx, my, p.x, p.y, cellPx, findBedSleeper(mx, my));
        } else if (c.s === 'ridge' || c.s === 'herb_seed' || c.s === 'herb_plot' || c.s === 'crop_seed' || c.s === 'crop_plot') {
          drawGardenPlot(c.s, mx, my, p.x, p.y, cellPx, nowMs, effWeather.raining);
        }

        // 中景尺度（5 <= cellPx < 12）自然物繪製簡潔白描圓叢點；近景（>= 12）則移入 Y-Sort 佇列
        if (cellPx < 12.0) {
          if (c.p === 'bamboo') {
            const r = Math.max(2.2, cellPx * 0.28);
            ctx.fillStyle = '#6c8c62';
            ctx.strokeStyle = PAWN_PAL.ink;
            ctx.lineWidth = 1.0;
            ctx.beginPath();
            ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
            ctx.fill();
            ctx.stroke();
          } else if (c.p === 'shoot') {
            ctx.fillStyle = '#8fa868';
            ctx.strokeStyle = PAWN_PAL.ink;
            ctx.lineWidth = 0.9;
            ctx.beginPath();
            ctx.arc(p.x, p.y, Math.max(1.8, cellPx * 0.16), 0, Math.PI * 2);
            ctx.fill();
            ctx.stroke();
          } else if (c.p === 'herb') {
            ctx.fillStyle = '#527c65';
            ctx.strokeStyle = PAWN_PAL.ink;
            ctx.lineWidth = 0.9;
            ctx.beginPath();
            ctx.arc(p.x, p.y, Math.max(1.9, cellPx * 0.18), 0, Math.PI * 2);
            ctx.fill();
            ctx.stroke();
          } else if (c.p === 'stone') {
            ctx.fillStyle = '#9e988e';
            ctx.strokeStyle = PAWN_PAL.ink;
            ctx.lineWidth = 0.9;
            ctx.beginPath();
            ctx.arc(p.x, p.y, Math.max(2.0, cellPx * 0.22), 0, Math.PI * 2);
            ctx.fill();
            ctx.stroke();
          }
          if (c.b) {
            const isChop = c.b.k === 'chop' || c.b.k === 'decon';
            ctx.strokeStyle = isChop ? '#8b2020' : '#2a4a6b';
            ctx.lineWidth = 1.4;
            ctx.setLineDash([3, 3]);
            ctx.strokeRect(x0 + 2, y0 + 2, cellPx - 4, cellPx - 4);
            ctx.setLineDash([]);
          }
        }

        // 地上散落或儲物席白描物資圖標
        if (c.i && cellPx >= 13) {
          drawTopDownItem(c.i, p.x, p.y, cellPx);
        }
      }
    }

    // 接觸插座焦點柔圓標記（不畫棋盤硬框）
    if (selectedCell) {
      const sp = microToScreen(selectedCell.mx, selectedCell.my);
      ctx.strokeStyle = '#8b2020';
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

      if (st.path && st.path.length > 0) {
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
        const errD = Math.hypot(st.srvX - st.x, st.srvY - st.y);
        const drift = errD > 0.85 ? Math.min(1.0, 2.8 * dt) : Math.min(1.0, 0.14 * dt);
        st.x += (st.srvX - st.x) * drift;
        st.y += (st.srvY - st.y) * drift;
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

  // ═══ 宣紙白描簡約全俯視角色與自然物繪製器官（與 pawn-demo v8 同源） ═══

  function drawPawnPart(img, x, y, w, h, ax, ay, rotRad = 0) {
    if (!isDrawableReady(img)) return;
    ctx.save();
    ctx.translate(x, y);
    if (rotRad) ctx.rotate(rotRad);
    ctx.drawImage(img, -w * ax, -h * ay, w, h);
    ctx.restore();
  }

  // 無縫外公切線漸縮膠囊體：d <= |r0-r1| 時自動退化為單圓，徹底根絕斷肢凸起
  function drawTaperedLimb(x0, y0, r0, x1, y1, r1, fill, ink, lw) {
    const dx = x1 - x0;
    const dy = y1 - y0;
    const d = Math.hypot(dx, dy);
    ctx.save();
    ctx.fillStyle = fill;
    ctx.strokeStyle = ink;
    ctx.lineWidth = lw;
    ctx.lineJoin = 'round';
    ctx.lineCap = 'round';
    ctx.beginPath();
    if (d <= Math.abs(r0 - r1) + 0.05) {
      const cx = r0 >= r1 ? x0 : x1;
      const cy = r0 >= r1 ? y0 : y1;
      ctx.arc(cx, cy, Math.max(r0, r1), 0, Math.PI * 2);
    } else {
      const theta = Math.atan2(dy, dx);
      const cosA = Math.max(-1, Math.min(1, (r0 - r1) / d));
      const alpha = Math.acos(cosA);
      ctx.arc(x0, y0, r0, theta + alpha, theta + Math.PI * 2 - alpha, false);
      ctx.arc(x1, y1, r1, theta - alpha, theta + alpha, false);
      ctx.closePath();
    }
    ctx.fill();
    ctx.stroke();
    ctx.restore();
  }

  function drawVestTorso(S, lw, vestFill, vestTrim) {
    ctx.save();
    ctx.lineWidth = lw;
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.fillStyle = vestFill;

    ctx.beginPath();
    ctx.moveTo(0, -0.23 * S);
    ctx.bezierCurveTo(0.24 * S, -0.24 * S, 0.43 * S, -0.15 * S, 0.43 * S, 0.0);
    ctx.bezierCurveTo(0.43 * S, 0.14 * S, 0.28 * S, 0.22 * S, 0, 0.22 * S);
    ctx.bezierCurveTo(-0.28 * S, 0.22 * S, -0.43 * S, 0.14 * S, -0.43 * S, 0.0);
    ctx.bezierCurveTo(-0.43 * S, -0.15 * S, -0.24 * S, -0.24 * S, 0, -0.23 * S);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();

    // 淺灰內衫領口
    ctx.fillStyle = PAWN_PAL.sleeve;
    ctx.beginPath();
    ctx.ellipse(0, -0.17 * S, 0.16 * S, 0.07 * S, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();

    // 背心前襟滾邊縫線
    ctx.strokeStyle = vestTrim;
    ctx.lineWidth = lw * 0.72;
    ctx.beginPath();
    ctx.moveTo(-0.07 * S, -0.22 * S);
    ctx.lineTo(0, -0.11 * S);
    ctx.lineTo(0.07 * S, -0.22 * S);
    ctx.stroke();
    ctx.restore();
  }

  function drawArticulatedArm(shX, shY, elX, elY, haX, haY, S, lw, isRight) {
    const midX = lerp(elX, haX, 0.42);
    const midY = lerp(elY, haY, 0.42);
    drawTaperedLimb(elX, elY, 0.072 * S, midX, midY, 0.070 * S, PAWN_PAL.skin, PAWN_PAL.ink, lw * 0.9);
    drawTaperedLimb(midX, midY, 0.072 * S, haX, haY, 0.064 * S, PAWN_PAL.bracer, PAWN_PAL.ink, lw * 0.9);

    const fAng = Math.atan2(haY - elY, haX - elX);
    ctx.save();
    ctx.translate(lerp(midX, haX, 0.5), lerp(midY, haY, 0.5));
    ctx.rotate(fAng);
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw * 0.6;
    ctx.beginPath();
    ctx.moveTo(-0.04 * S, -0.04 * S);
    ctx.lineTo(0.04 * S, 0.04 * S);
    ctx.moveTo(-0.04 * S, 0.04 * S);
    ctx.lineTo(0.04 * S, -0.04 * S);
    ctx.stroke();
    ctx.restore();

    ctx.save();
    ctx.translate(haX, haY);
    ctx.rotate(fAng + Math.PI / 2);
    ctx.fillStyle = PAWN_PAL.glove;
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw * 0.9;
    ctx.beginPath();
    ctx.ellipse(0, 0, 0.068 * S, 0.076 * S, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    const thumbX = isRight ? -0.048 * S : 0.048 * S;
    ctx.fillStyle = PAWN_PAL.skin;
    ctx.beginPath();
    ctx.arc(thumbX, -0.032 * S, 0.022 * S, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.restore();

    const slX = lerp(shX, elX, 0.88);
    const slY = lerp(shY, elY, 0.88);
    drawTaperedLimb(shX, shY, 0.105 * S, slX, slY, 0.085 * S, PAWN_PAL.sleeve, PAWN_PAL.ink, lw);
  }

  function drawRollAxe(handX, handY, axeYaw, axeRoll, S, lw) {
    const L = (0.64 + 0.16 * axeRoll) * S;
    ctx.save();
    ctx.translate(handX, handY);
    ctx.rotate(axeYaw);

    drawTaperedLimb(0, 0.085 * L, 0.024 * S, 0, -0.50 * L, 0.028 * S, PAWN_PAL.wood, PAWN_PAL.ink, lw * 0.82);

    const eyeY = -0.47 * L;
    const pollW = (0.042 + 0.015 * axeRoll) * S;
    const bladeOut = (0.026 + 0.215 * axeRoll) * S;
    const halfH = (0.070 + 0.045 * axeRoll) * S;

    ctx.fillStyle = PAWN_PAL.iron;
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw * 0.85;
    ctx.lineJoin = 'round';
    ctx.beginPath();
    ctx.moveTo(pollW, eyeY - halfH * 0.45);
    ctx.lineTo(pollW, eyeY + halfH * 0.45);
    ctx.lineTo(-bladeOut * 0.60, eyeY + halfH * 0.72);
    ctx.quadraticCurveTo(-bladeOut * 1.08, eyeY, -bladeOut * 0.75, eyeY - halfH);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();

    ctx.fillStyle = PAWN_PAL.steel;
    ctx.beginPath();
    ctx.moveTo(-bladeOut * 0.55, eyeY + halfH * 0.62);
    ctx.quadraticCurveTo(-bladeOut * 1.02, eyeY, -bladeOut * 0.70, eyeY - halfH * 0.90);
    ctx.quadraticCurveTo(-bladeOut * 0.76, eyeY, -bladeOut * 0.38, eyeY + halfH * 0.50);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();

    ctx.fillStyle = PAWN_PAL.wood;
    ctx.beginPath();
    ctx.ellipse(0, eyeY, 0.022 * S, 0.028 * S, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.restore();
  }

  // 動態翻轉烤刃竹刀（bladeRoll=0.12 僅見窄脊；揮砍轉腕展開淺竹青刀面與焦褐烤刃）
  function drawRollBambooBlade(handX, handY, bladeYaw, bladeRoll, S, lw) {
    const L = (0.68 + 0.16 * bladeRoll) * S;
    ctx.save();
    ctx.translate(handX, handY);
    ctx.rotate(bladeYaw);

    // 短竹把與細木護格
    drawTaperedLimb(0, 0.08 * L, 0.022 * S, 0, -0.07 * L, 0.024 * S, PAWN_PAL.wood, PAWN_PAL.ink, lw * 0.80);
    ctx.fillStyle = PAWN_PAL.wood;
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw * 0.80;
    ctx.beginPath();
    ctx.roundRect(-0.045 * S, -0.095 * L, 0.090 * S, 0.026 * S, 2);
    ctx.fill();
    ctx.stroke();

    const baseY = -0.10 * L;
    const tipY = -0.68 * L;
    const spineW = (0.018 + 0.012 * bladeRoll) * S;
    const edgeOut = (0.018 + 0.128 * bladeRoll) * S;

    ctx.fillStyle = bladeRoll > 0.35 ? PAWN_PAL.bambooBody : PAWN_PAL.bambooSpine;
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw * 0.85;
    ctx.lineJoin = 'round';
    ctx.beginPath();
    ctx.moveTo(spineW, baseY);
    ctx.lineTo(spineW * 0.72, tipY + 0.04 * L);
    ctx.quadraticCurveTo(-edgeOut * 0.25, tipY - 0.03 * L, -edgeOut * 0.82, tipY + 0.10 * L);
    ctx.quadraticCurveTo(-edgeOut * 1.05, lerp(baseY, tipY, 0.45), -edgeOut * 0.68, baseY);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();

    // 火烤硬化深褐刃線（於翻轉展開時顯現）
    if (bladeRoll > 0.30) {
      ctx.fillStyle = PAWN_PAL.bambooChar;
      ctx.beginPath();
      ctx.moveTo(-edgeOut * 0.62, baseY);
      ctx.quadraticCurveTo(-edgeOut * 1.03, lerp(baseY, tipY, 0.45), -edgeOut * 0.78, tipY + 0.10 * L);
      ctx.quadraticCurveTo(-edgeOut * 0.68, lerp(baseY, tipY, 0.45), -edgeOut * 0.38, baseY);
      ctx.closePath();
      ctx.fill();
      ctx.stroke();
    }
    ctx.restore();
  }

  function drawBaimiaoHuman(a, pos, sp, nowMs) {
    const S = Math.max(15.5, cellPx * 0.84);
    const lw = Math.max(1.15, 0.040 * S);
    const mb = pos.moveBlend || 0;
    const rb = pos.runBlend || 0;
    const s = Math.sin(pos.walkPhase || 0);
    const c = Math.cos(pos.walkPhase || 0);

    const h = hashStr(`${a.id}|${a.soul_seed || ''}`);
    const [vestFill, vestTrim] = VEST_PALETTES[h % VEST_PALETTES.length];
    const showHat = a.is_player
      ? (meDetail ? Boolean(meDetail.equipped_hat) : a.wear_hat !== false)
      : (typeof a.wear_hat === 'boolean' ? a.wear_hat : Number(a.id) % 2 === 1);
    const showPack =
      Boolean(a.carry) ||
      Boolean(a.show_pack) ||
      Boolean(a.is_player && meDetail && meDetail.equipped_pack);

    const actStr = String(a.act || '');
    const isSitting = /歇息|盤坐|圍爐|調息/.test(actStr) && mb < 0.15;
    const isGathering = /拾取|採筍|採摘|搬回/.test(actStr) && mb < 0.25 && pos.chopTimer <= 0;
    const isCrafting = /^(打造|製作|烹製)/.test(actStr) && pos.chopTimer > 0;

    let torsoYaw = -s * (0.09 + 0.05 * rb) * mb;

    const idleElL = [-0.41 * S, 0.03 * S];
    const idleHaL = [-0.42 * S, -0.09 * S];
    const moveElL = [
      -0.41 * S,
      0.03 * S + Math.max(0, s) * (0.10 + 0.08 * rb) * S + Math.min(0, s) * (0.06 * rb) * S,
    ];
    const moveHaL = [
      lerp(-0.42, -0.40, rb) * S,
      lerp(-0.09, -0.18, rb) * S + s * (0.18 + 0.08 * rb) * S,
    ];
    let elL = lerpPt(idleElL, moveElL, mb);
    let haL = lerpPt(idleHaL, moveHaL, mb);

    const idleElR = [0.41 * S, 0.03 * S];
    const idleHaR = [0.42 * S, -0.11 * S];
    const moveElR = [
      0.41 * S,
      0.03 * S + Math.max(0, -s) * (0.08 + 0.06 * rb) * S + Math.min(0, -s) * (0.05 * rb) * S,
    ];
    const moveHaR = [
      lerp(0.42, 0.40, rb) * S,
      lerp(-0.11, -0.20, rb) * S - s * (0.14 + 0.06 * rb) * S,
    ];
    let elR = lerpPt(idleElR, moveElR, mb);
    let haR = lerpPt(idleHaR, moveHaR, mb);

    let axeYaw = -0.04 - s * 0.10 * mb;
    let axeRoll = 0.12;
    let slashPhase = 0;

    if (isSitting) {
      const breath = Math.sin((nowMs || 0) * 0.0022 + Number(a.id || 0)) * 0.015 * S;
      elL = [-0.36 * S, -0.02 * S];
      haL = [-0.21 * S, -0.21 * S + breath];
      elR = [0.36 * S, -0.02 * S];
      haR = [0.21 * S, -0.21 * S + breath];
      torsoYaw = Math.sin((nowMs || 0) * 0.0014 + Number(a.id || 0)) * 0.04;
      axeYaw = -0.35;
      axeRoll = 0.10;
    } else if (isGathering) {
      const gp = 0.5 - 0.5 * Math.cos((nowMs || 0) * 0.008 + Number(a.id || 0));
      torsoYaw = 0.18 * gp;
      elL = [-0.39 * S, 0.04 * S];
      haL = [-0.34 * S, -0.12 * S];
      elR = [0.34 * S, -0.12 * S];
      haR = [0.12 * S, (-0.30 - 0.14 * gp) * S];
      axeYaw = -0.18;
    } else if (isCrafting) {
      const p = 1 - pos.chopTimer / CHOP_DURATION;
      const tap = Math.max(0, Math.sin(p * Math.PI * 2));
      torsoYaw = -0.10 + 0.14 * tap;
      elL = [-0.32 * S, -0.08 * S];
      haL = [-0.12 * S, -0.34 * S];
      elR = [0.36 * S, -0.12 * S];
      haR = [0.16 * S, (-0.28 - 0.14 * tap) * S];
      axeYaw = lerp(-0.12, -0.52, tap);
      axeRoll = 0.52;
    } else if (pos.chopTimer > 0) {
      const p = 1 - pos.chopTimer / CHOP_DURATION;
      const baseElL = elL, baseHaL = haL, baseElR = elR, baseHaR = haR, baseYaw = axeYaw;
      const wElL = [-0.41 * S, -0.04 * S], wHaL = [-0.42 * S, -0.22 * S];
      const wElR = [0.46 * S, 0.10 * S], wHaR = [0.55 * S, 0.02 * S];
      const sElL = [-0.44 * S, 0.12 * S], sHaL = [-0.45 * S, 0.02 * S];
      const sElR = [0.34 * S, -0.22 * S], sHaR = [0.16 * S, -0.48 * S];
      const fElL = [-0.44 * S, 0.14 * S], fHaL = [-0.44 * S, 0.04 * S];
      const fElR = [0.24 * S, -0.24 * S], fHaR = [0.02 * S, -0.46 * S];

      if (p < 0.28) {
        const k = smoothstep(p / 0.28);
        torsoYaw += k * 0.48;
        elL = lerpPt(baseElL, wElL, k);
        haL = lerpPt(baseHaL, wHaL, k);
        elR = lerpPt(baseElR, wElR, k);
        haR = lerpPt(baseHaR, wHaR, k);
        axeYaw = lerp(baseYaw, 0.78, k);
        axeRoll = lerp(0.12, 0.86, k);
      } else if (p < 0.54) {
        const k = smoothstep((p - 0.28) / 0.26);
        torsoYaw += lerp(0.48, -0.44, k);
        elL = lerpPt(wElL, sElL, k);
        haL = lerpPt(wHaL, sHaL, k);
        elR = lerpPt(wElR, sElR, k);
        haR = lerpPt(wHaR, sHaR, k);
        axeYaw = lerp(0.78, -0.78, k);
        axeRoll = 0.88;
        if (p > 0.34) slashPhase = (p - 0.34) / 0.20;
      } else if (p < 0.74) {
        const k = smoothstep((p - 0.54) / 0.20);
        torsoYaw += lerp(-0.44, -0.50, k);
        elL = lerpPt(sElL, fElL, k);
        haL = lerpPt(sHaL, fHaL, k);
        elR = lerpPt(sElR, fElR, k);
        haR = lerpPt(sHaR, fHaR, k);
        axeYaw = lerp(-0.78, -0.96, k);
        axeRoll = 0.82;
      } else {
        const k = smoothstep((p - 0.74) / 0.26);
        torsoYaw += lerp(-0.50, 0, k);
        elL = lerpPt(fElL, baseElL, k);
        haL = lerpPt(fHaL, baseHaL, k);
        elR = lerpPt(fElR, baseElR, k);
        haR = lerpPt(fHaR, baseHaR, k);
        axeYaw = lerp(-0.96, baseYaw, k);
        axeRoll = lerp(0.82, 0.12, k);
      }
    }

    ctx.save();
    ctx.translate(sp.x, sp.y);

    // 雙擊疾奔身後水墨殘影（與世界座標方向對齊）
    if ((pos.running || a.running) && pos.path && pos.path.length > 0 && typeof pos.facing === 'number') {
      for (const k of [1, 2]) {
        const bx = -Math.cos(pos.facing) * (k * 0.26 * S);
        const by = Math.sin(pos.facing) * (k * 0.26 * S);
        ctx.strokeStyle = k === 1 ? 'rgba(139, 32, 32, 0.30)' : 'rgba(139, 32, 32, 0.14)';
        ctx.lineWidth = Math.max(1.2, lw * 0.9);
        ctx.beginPath();
        ctx.arc(bx, by, (0.44 - k * 0.04) * S, 0, Math.PI * 2);
        ctx.stroke();
      }
    }

    // 柔和接地水暈陰影 & 本尊硃砂氣韻細圈
    ctx.fillStyle = 'rgba(44, 30, 15, 0.09)';
    ctx.beginPath();
    ctx.ellipse(0, 2, S * (isSitting ? 0.52 : 0.48), S * (isSitting ? 0.44 : 0.40), 0, 0, Math.PI * 2);
    ctx.fill();

    if (a.is_player) {
      ctx.strokeStyle = 'rgba(139, 32, 32, 0.52)';
      ctx.lineWidth = 1.6;
      ctx.beginPath();
      ctx.arc(0, 0, S * 0.50, 0, Math.PI * 2);
      ctx.stroke();
    }

    const isDowned = a.act === '力竭倒地';
    if (isDowned) {
      ctx.save();
      ctx.setLineDash([4, 3]);
      ctx.strokeStyle = 'rgba(139, 32, 32, 0.85)';
      ctx.lineWidth = 2.0;
      ctx.beginPath();
      ctx.arc(0, 0, S * 0.64, 0, Math.PI * 2);
      ctx.stroke();
      ctx.restore();
    } else if (typeof a.hp_ratio === 'number' && a.hp_ratio < 0.99) {
      ctx.strokeStyle = '#8b2020';
      ctx.lineWidth = 2.2;
      ctx.beginPath();
      ctx.arc(0, 0, S * 0.56, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * a.hp_ratio);
      ctx.stroke();
    }

    // ★ 世界座標 +my 為北（螢幕上方 -Y）；局部骨架前方向為 -Y，因此旋轉必須為 (Math.PI / 2 - pos.facing) ★
    ctx.rotate(Math.PI / 2 - pos.facing + (isDowned ? 1.35 : 0));

    // ─ 骨盆與雙腿層 ─
    const pelvisRot = isSitting ? 0 : s * (0.09 + 0.04 * rb) * mb;
    const stepHalfW = (0.115 - 0.02 * rb) * S;
    const frontAmp = lerp(0.32, 0.45, rb) * S;
    const rearAmp = lerp(0.18, 0.29, rb) * S;

    ctx.save();
    ctx.rotate(pelvisRot);
    if (isSitting) {
      // 俯視盤坐：雙膝朝前外張，小腿與靴履交叉收回腿下
      for (const side of [-1, 1]) {
        drawTaperedLimb(side * 0.14 * S, 0.02 * S, 0.11 * S, side * 0.34 * S, -0.16 * S, 0.095 * S, PAWN_PAL.pants, PAWN_PAL.ink, lw * 0.9);
        drawTaperedLimb(side * 0.34 * S, -0.16 * S, 0.09 * S, -side * 0.05 * S, -0.24 * S, 0.072 * S, PAWN_PAL.pants, PAWN_PAL.ink, lw * 0.9);
        drawPawnPart(pawnImgs.bootTop, -side * 0.11 * S, -0.24 * S, 0.15 * S, 0.24 * S, 0.50, 0.55, side * 0.52);
      }
    } else {
      const legs = [
        { side: -1, phi: pos.walkPhase || 0, idleY: -0.09 * S },
        { side: 1, phi: (pos.walkPhase || 0) + Math.PI, idleY: -0.01 * S },
      ];
      const legComputed = legs.map((lg) => {
        const spf = Math.sin(lg.phi);
        const moveY = spf >= 0 ? -spf * frontAmp : -spf * rearAmp;
        const footY = lerp(lg.idleY, moveY, mb);
        const heelShrink = spf < 0 ? 1 - 0.18 * (-spf) * mb * (0.4 + 0.6 * rb) : 1.0;
        return { side: lg.side, hipX: lg.side * stepHalfW, footY, bootH: 0.35 * S * heelShrink };
      });
      legComputed.sort((u, v) => v.footY - u.footY);

      for (const st of legComputed) {
        drawPawnPart(pawnImgs.bootTop, st.hipX, st.footY, 0.175 * S, st.bootH, 0.50, 0.55, st.side * 0.02);
        if (st.footY < -0.14 * S) {
          drawTaperedLimb(st.hipX, -0.02 * S, 0.105 * S, st.hipX, st.footY + 0.02 * S, 0.088 * S, PAWN_PAL.pants, PAWN_PAL.ink, lw * 0.9);
        }
      }
    }

    ctx.fillStyle = PAWN_PAL.pants;
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw;
    ctx.beginPath();
    ctx.ellipse(0, 0.01 * S, 0.28 * S, 0.18 * S, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.restore();

    // ─ 胸肩雙臂與右手器械層 ─
    ctx.save();
    ctx.rotate(torsoYaw);

    if (a.tool && !isSitting) {
      if (/刀|劍/.test(a.tool)) {
        drawRollBambooBlade(haR[0], haR[1], axeYaw, axeRoll, S, lw);
      } else {
        drawRollAxe(haR[0], haR[1], axeYaw, axeRoll, S, lw);
      }
    }

    if (slashPhase > 0) {
      ctx.save();
      ctx.strokeStyle = a.tool ? 'rgba(44, 30, 15, 0.64)' : 'rgba(44, 30, 15, 0.34)';
      ctx.lineWidth = lw * 1.1;
      ctx.beginPath();
      ctx.arc(0.03 * S, -0.14 * S, (a.tool ? 0.78 : 0.54) * S, -Math.PI * 0.84, -Math.PI * 0.16);
      ctx.stroke();
      if (a.tool) {
        ctx.strokeStyle = 'rgba(44, 30, 15, 0.28)';
        ctx.lineWidth = lw * 0.7;
        ctx.beginPath();
        ctx.arc(0.03 * S, -0.14 * S, 0.68 * S, -Math.PI * 0.78, -Math.PI * 0.22);
        ctx.stroke();
      }
      ctx.restore();
    }

    drawArticulatedArm(-0.36 * S, 0, elL[0], elL[1], haL[0], haL[1], S, lw, false);
    drawArticulatedArm(0.36 * S, 0, elR[0], elR[1], haR[0], haR[1], S, lw, true);
    drawVestTorso(S, lw, vestFill, vestTrim);

    if (showPack) {
      drawPawnPart(pawnImgs.pack, 0, 0.27 * S + Math.abs(c) * 0.018 * S * mb, 0.78 * S, 0.72 * S, 0.50, 0.48, -torsoYaw * 0.28);
    }
    ctx.restore();

    // ─ 頭部頂層 ─
    const headBobScale = 1 + Math.abs(c) * 0.020 * mb;
    if (showHat) {
      drawPawnPart(pawnImgs.hat, 0, -0.03 * S, 0.88 * S * headBobScale, 0.88 * S * headBobScale, 0.50, 0.50, s * 0.025 * mb);
    } else {
      drawPawnPart(pawnImgs.head, 0, -0.03 * S, 0.44 * S * headBobScale, 0.56 * S * headBobScale, 0.50, 0.55, s * 0.03 * mb);
    }

    ctx.restore();
  }

  function drawBaimiaoBeast(a, pos, sp) {
    const S = Math.max(15.5, cellPx * 0.84);
    const mb = pos.moveBlend || 0;
    const s = Math.sin(pos.walkPhase || 0);
    const isPheasant = a.glyph === '雉' || /雞|雉|禽/.test(a.name || a.role || '');

    ctx.save();
    ctx.translate(sp.x, sp.y);

    ctx.fillStyle = 'rgba(44, 30, 15, 0.09)';
    ctx.beginPath();
    ctx.ellipse(0, 2, S * (isPheasant ? 0.34 : 0.46), S * (isPheasant ? 0.28 : 0.36), 0, 0, Math.PI * 2);
    ctx.fill();

    if (typeof a.hp_ratio === 'number' && a.hp_ratio < 0.99) {
      ctx.strokeStyle = '#8b2020';
      ctx.lineWidth = 2.0;
      ctx.beginPath();
      ctx.arc(0, 0, S * 0.52, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * a.hp_ratio);
      ctx.stroke();
    }

    ctx.rotate(Math.PI / 2 - pos.facing);

    const gaitSway = s * 0.055 * mb;
    const bobScale = 1 + Math.abs(s) * 0.025 * mb;
    const img = isPheasant ? pawnImgs.pheasant : pawnImgs.boar;

    if (img && img.complete && img.naturalWidth) {
      if (mb > 0.08 && !isPheasant) {
        const diagA = s * 0.11 * S * mb;
        const diagB = -s * 0.11 * S * mb;
        ctx.fillStyle = '#3a2c24';
        for (const [lx, ly] of [
          [-0.20 * S, -0.18 * S - diagA],
          [0.20 * S, 0.18 * S + diagA],
          [0.20 * S, -0.18 * S - diagB],
          [-0.20 * S, 0.18 * S + diagB],
        ]) {
          ctx.beginPath();
          ctx.ellipse(lx, ly, 0.042 * S, 0.062 * S, 0, 0, Math.PI * 2);
          ctx.fill();
        }
      }
      const w = (isPheasant ? 0.96 : 1.04) * S * bobScale;
      const h = (isPheasant ? 1.08 : 1.18) * S * bobScale;
      drawPawnPart(img, 0, 0, w, h, 0.50, 0.50, gaitSway);
    } else {
      ctx.fillStyle = isPheasant ? '#6f685f' : '#78523d';
      ctx.strokeStyle = PAWN_PAL.ink;
      ctx.lineWidth = Math.max(1.15, 0.038 * S);
      ctx.beginPath();
      ctx.ellipse(0, 0, S * 0.24, S * 0.34, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }

    ctx.restore();
  }

  function drawGardenPlot(kind, mx, my, cx, cy, cellPx, nowMs, isRaining) {
    const w = cellPx * 0.90;
    const h = cellPx * 0.86;
    const lw = Math.max(0.95, cellPx * 0.025);
    const sway = Math.sin(nowMs * 0.0028 + mx * 1.4 + my * 0.9) * (cellPx * 0.024);

    ctx.save();
    ctx.translate(cx, cy);

    // 1. 宣紙白描沃土畦底（雨天呈深潤土色）
    ctx.fillStyle = isRaining ? '#846b50' : '#9c8367';
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw;
    ctx.beginPath();
    ctx.roundRect(-w / 2, -h / 2, w, h, Math.max(3, cellPx * 0.10));
    ctx.fill();
    ctx.stroke();

    // 2. 三道立體翻鬆土壟條
    for (const ry of [-h * 0.27, 0, h * 0.27]) {
      const rw = w * 0.82;
      const rh = h * 0.20;
      ctx.fillStyle = isRaining ? '#94785b' : '#b79c7c';
      ctx.beginPath();
      ctx.roundRect(-rw / 2, ry - rh / 2, rw, rh, Math.max(2, rh * 0.42));
      ctx.fill();
      ctx.strokeStyle = 'rgba(51, 38, 32, 0.36)';
      ctx.lineWidth = Math.max(0.75, lw * 0.72);
      ctx.beginPath();
      ctx.moveTo(-rw * 0.42, ry + rh * 0.48);
      ctx.lineTo(rw * 0.42, ry + rh * 0.48);
      ctx.stroke();
    }

    if (cellPx < 13) {
      if (kind === 'herb_seed' || kind === 'herb_plot') {
        ctx.fillStyle = kind === 'herb_plot' ? '#4d7761' : '#80a67d';
        ctx.beginPath();
        ctx.arc(0, 0, Math.max(2, cellPx * (kind === 'herb_plot' ? 0.26 : 0.16)), 0, Math.PI * 2);
        ctx.fill();
      } else if (kind === 'crop_seed' || kind === 'crop_plot') {
        ctx.fillStyle = kind === 'crop_plot' ? '#587a45' : '#86a766';
        ctx.beginPath();
        ctx.arc(0, 0, Math.max(2, cellPx * (kind === 'crop_plot' ? 0.26 : 0.16)), 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.restore();
      return;
    }

    if (kind === 'herb_seed') {
      const spots = [[-0.22, -0.26], [0.16, -0.24], [-0.04, 0.0], [-0.20, 0.26], [0.21, 0.25]];
      for (const [sx, sy] of spots) {
        ctx.save();
        ctx.translate(sx * cellPx + sway * 0.4, sy * cellPx);
        ctx.fillStyle = '#7ea67b';
        ctx.strokeStyle = PAWN_PAL.ink;
        ctx.lineWidth = lw * 0.8;
        for (const a of [-0.65, 0.65]) {
          ctx.save();
          ctx.rotate(a);
          ctx.beginPath();
          ctx.ellipse(0, -cellPx * 0.06, cellPx * 0.036, cellPx * 0.072, 0, 0, Math.PI * 2);
          ctx.fill();
          ctx.stroke();
          ctx.restore();
        }
        ctx.restore();
      }
    } else if (kind === 'herb_plot') {
      const clusters = [[-0.22, -0.24], [0.18, -0.22], [-0.02, 0.02], [-0.21, 0.24], [0.20, 0.23]];
      for (let i = 0; i < clusters.length; i++) {
        const [bx, by] = clusters[i];
        ctx.save();
        ctx.translate(bx * cellPx + sway * 0.65, by * cellPx);
        for (let k = 0; k < 5; k++) {
          const ang = (k * Math.PI * 2) / 5 + (i * 0.4);
          ctx.save();
          ctx.rotate(ang);
          ctx.fillStyle = k % 2 === 0 ? '#4d7660' : '#6c937a';
          ctx.strokeStyle = PAWN_PAL.ink;
          ctx.lineWidth = lw * 0.78;
          ctx.beginPath();
          ctx.ellipse(0, -cellPx * 0.085, cellPx * 0.042, cellPx * 0.095, 0, 0, Math.PI * 2);
          ctx.fill();
          ctx.stroke();
          ctx.restore();
        }
        ctx.fillStyle = '#f3ebf6';
        ctx.strokeStyle = PAWN_PAL.ink;
        ctx.lineWidth = lw * 0.72;
        ctx.beginPath();
        ctx.arc(0, 0, cellPx * 0.046, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = '#cfa048';
        ctx.beginPath();
        ctx.arc(0, 0, cellPx * 0.020, 0, Math.PI * 2);
        ctx.fill();
        ctx.restore();
      }
    } else if (kind === 'crop_seed') {
      ctx.strokeStyle = '#5a7b45';
      ctx.lineWidth = lw * 1.1;
      ctx.beginPath();
      ctx.moveTo(-w * 0.34, -h * 0.22);
      ctx.quadraticCurveTo(0, -h * 0.04, w * 0.32, h * 0.20);
      ctx.stroke();
      for (const [lx, ly, rot] of [[-0.24, -0.22, -0.4], [0.14, -0.18, 0.5], [-0.08, 0.04, -0.6], [0.22, 0.22, 0.3]]) {
        ctx.save();
        ctx.translate(lx * cellPx + sway * 0.3, ly * cellPx);
        ctx.rotate(rot);
        ctx.fillStyle = '#85a664';
        ctx.strokeStyle = PAWN_PAL.ink;
        ctx.lineWidth = lw * 0.78;
        ctx.beginPath();
        ctx.ellipse(0, 0, cellPx * 0.072, cellPx * 0.052, 0, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
        ctx.restore();
      }
    } else if (kind === 'crop_plot') {
      // 甘薯心形茂葉叢＋紫赭紅塊根露頭
      ctx.strokeStyle = '#537341';
      ctx.lineWidth = lw * 1.15;
      ctx.beginPath();
      ctx.moveTo(-w * 0.34, -h * 0.26);
      ctx.bezierCurveTo(-w * 0.1, h * 0.12, w * 0.14, -h * 0.15, w * 0.34, h * 0.24);
      ctx.stroke();

      for (const [lx, ly, rot, sc] of [
        [-0.24, -0.24, -0.5, 1.0], [0.06, -0.25, 0.4, 0.92], [0.24, -0.16, 1.1, 0.96],
        [-0.22, 0.04, -1.2, 1.05], [0.04, 0.02, 0.2, 1.08], [0.25, 0.14, 0.8, 0.95],
        [-0.14, 0.25, -0.3, 1.0], [0.12, 0.26, 0.6, 0.92],
      ]) {
        const s = cellPx * 0.088 * sc;
        ctx.save();
        ctx.translate(lx * cellPx + sway * 0.55, ly * cellPx);
        ctx.rotate(rot);
        ctx.fillStyle = sc > 1.0 ? '#587a45' : '#769859';
        ctx.strokeStyle = PAWN_PAL.ink;
        ctx.lineWidth = lw * 0.78;
        ctx.beginPath();
        ctx.moveTo(0, -s);
        ctx.bezierCurveTo(s * 1.15, -s * 0.65, s * 0.95, s * 0.95, 0, s * 0.75);
        ctx.bezierCurveTo(-s * 0.95, s * 0.95, -s * 1.15, -s * 0.65, 0, -s);
        ctx.closePath();
        ctx.fill();
        ctx.stroke();
        ctx.restore();
      }
      // 露土甘薯塊根頭
      for (const [tx, ty, trot] of [[-0.12, -0.11, -0.4], [0.14, 0.06, 0.5], [-0.04, 0.22, 0.15]]) {
        ctx.save();
        ctx.translate(tx * cellPx, ty * cellPx);
        ctx.rotate(trot);
        ctx.fillStyle = '#a15847';
        ctx.strokeStyle = PAWN_PAL.ink;
        ctx.lineWidth = lw * 0.85;
        ctx.beginPath();
        ctx.ellipse(0, 0, cellPx * 0.11, cellPx * 0.066, 0, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
        ctx.restore();
      }
    }

    ctx.restore();
  }

  function drawBambooBed(mx, my, cx, cy, cellPx, sleeper) {
    const w = cellPx * 1.32;
    const h = cellPx * 0.84;
    const lw = Math.max(1.05, cellPx * 0.028);
    ctx.save();
    ctx.translate(cx, cy);

    ctx.fillStyle = 'rgba(44, 30, 15, 0.14)';
    ctx.fillRect(-w / 2 + 2.2, -h / 2 + 2.6, w, h);

    ctx.fillStyle = '#d4bd97';
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw;
    ctx.beginPath();
    ctx.roundRect(-w / 2, -h / 2, w, h, 4);
    ctx.fill();
    ctx.stroke();

    ctx.fillStyle = '#eadcc2';
    ctx.fillRect(-w * 0.44, -h * 0.40, w * 0.88, h * 0.80);
    ctx.strokeStyle = 'rgba(92, 68, 46, 0.24)';
    ctx.lineWidth = 0.8;
    ctx.beginPath();
    for (let i = -3; i <= 3; i++) {
      const sx = (w * 0.42 * i) / 4;
      ctx.moveTo(sx, -h * 0.39);
      ctx.lineTo(sx, h * 0.39);
    }
    ctx.stroke();

    const postR = cellPx * 0.072;
    for (const [px, py] of [
      [-w * 0.47, -h * 0.46], [w * 0.47, -h * 0.46],
      [-w * 0.47, h * 0.46], [w * 0.47, h * 0.46],
    ]) {
      ctx.fillStyle = '#9c7c57';
      ctx.strokeStyle = PAWN_PAL.ink;
      ctx.lineWidth = lw * 0.9;
      ctx.beginPath();
      ctx.arc(px, py, postR, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }

    // 東側竹枕
    ctx.fillStyle = '#f5efe2';
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw * 0.9;
    ctx.beginPath();
    ctx.roundRect(w * 0.20, -h * 0.31, w * 0.21, h * 0.62, 4);
    ctx.fill();
    ctx.stroke();

    if (sleeper) {
      const S = cellPx * 0.72;
      const sh = hashStr(`${sleeper.id}|${sleeper.soul_seed || ''}`);
      const [vestFill] = VEST_PALETTES[sh % VEST_PALETTES.length];
      ctx.save();
      ctx.translate(w * 0.16, 0);
      ctx.rotate(-Math.PI / 2);
      ctx.fillStyle = vestFill;
      ctx.strokeStyle = PAWN_PAL.ink;
      ctx.lineWidth = lw;
      ctx.beginPath();
      ctx.ellipse(0, 0.16 * S, 0.36 * S, 0.18 * S, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
      drawPawnPart(pawnImgs.head, 0, -0.06 * S, 0.44 * S, 0.54 * S, 0.5, 0.5, 0);
      ctx.restore();
    }

    // 礦物靛藍被褥＋米白翻領緣
    const qLeft = -w * 0.43;
    const qW = sleeper ? w * 0.55 : w * 0.48;
    ctx.fillStyle = '#62717f';
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw;
    ctx.beginPath();
    ctx.roundRect(qLeft, -h * 0.37, qW, h * 0.74, 3);
    ctx.fill();
    ctx.stroke();

    ctx.strokeStyle = 'rgba(34, 43, 52, 0.42)';
    ctx.lineWidth = 0.9;
    ctx.beginPath();
    ctx.moveTo(qLeft + qW * 0.22, -h * 0.26);
    ctx.lineTo(qLeft + qW * 0.75, -h * 0.10);
    ctx.moveTo(qLeft + qW * 0.18, h * 0.16);
    ctx.lineTo(qLeft + qW * 0.72, h * 0.22);
    ctx.stroke();

    ctx.fillStyle = '#efe7d6';
    ctx.strokeStyle = PAWN_PAL.ink;
    ctx.lineWidth = lw * 0.85;
    ctx.fillRect(qLeft + qW - w * 0.08, -h * 0.37, w * 0.08, h * 0.74);
    ctx.strokeRect(qLeft + qW - w * 0.08, -h * 0.37, w * 0.08, h * 0.74);
    ctx.restore();

    if (sleeper) {
      bedBadgeQueue.push({
        mx,
        my,
        cx,
        by: cy - h * 0.5 - 11,
        txt: `${sleeper.name || sleeper.glyph || ''}・榻上安歇`,
        isPlayer: !!sleeper.is_player,
      });
    }
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
      const nearDoor = me && doors.some((dp) => Math.hypot(meX - dp[0], meY - dp[1]) <= 1.25);
      const selInside = selectedCell && selectedCell.mx >= rm.x0 && selectedCell.mx <= rm.x1 && selectedCell.my >= rm.y0 && selectedCell.my <= rm.y1;
      const targetAlpha = (meInside || nearDoor || selInside) ? 0.0 : 1.0;

      let alpha = roomRoofAlpha.has(rk) ? roomRoofAlpha.get(rk) : targetAlpha;
      alpha += (targetAlpha - alpha) * (1.0 - Math.exp(-9.0 * dt));
      if (Math.abs(alpha - targetAlpha) < 0.01) alpha = targetAlpha;
      roomRoofAlpha.set(rk, alpha);

      const pTL = microToScreen(rm.x0 - 0.84, rm.y1 + 0.86);
      const pBR = microToScreen(rm.x1 + 0.84, rm.y0 - 0.86);
      const rx0 = pTL.x;
      const ry0 = pTL.y;
      const rw = pBR.x - pTL.x;
      const rh = pBR.y - pTL.y;

      // 掀頂後仍保留極淡滴水簷虛線，讓屋簷保護界線可察覺
      ctx.save();
      ctx.setLineDash([4, 4]);
      ctx.strokeStyle = 'rgba(92, 68, 46, 0.22)';
      ctx.lineWidth = 1.05;
      ctx.strokeRect(rx0, ry0, rw, rh);
      ctx.restore();

      if (alpha <= 0.02 || rw <= 12 || rh <= 12) continue;

      const midY = ry0 + rh * 0.5;
      const lw = Math.max(1.15, cellPx * 0.030);
      ctx.save();
      ctx.globalAlpha = alpha * Math.min(1.0, (cellPx - 5.0) / 5.0);

      // 屋簷垂影
      ctx.fillStyle = 'rgba(34, 24, 14, 0.20)';
      ctx.fillRect(rx0 + cellPx * 0.14, ry0 + cellPx * 0.16, rw, rh);

      // 南北雙坡底色
      ctx.fillStyle = '#ddd0b7';
      ctx.fillRect(rx0, ry0, rw, rh * 0.5);
      ctx.fillStyle = '#ccc0a4';
      ctx.fillRect(rx0, midY, rw, rh * 0.5);

      // 手繪茅竹層疊脈絡
      if (cellPx >= 12) {
        ctx.lineCap = 'round';
        const layersPerSlope = 5;
        for (const isSouth of [false, true]) {
          const dir = isSouth ? 1 : -1;
          for (let layer = 0; layer < layersPerSlope; layer++) {
            const frac = (layer + 0.65) / layersPerSlope;
            const bandY = isSouth ? midY + frac * (rh * 0.48) : midY - frac * (rh * 0.48);
            ctx.strokeStyle = 'rgba(92, 68, 46, 0.28)';
            ctx.lineWidth = lw * 0.72;
            ctx.beginPath();
            ctx.moveTo(rx0 + cellPx * 0.18, bandY);
            ctx.lineTo(rx1Or(rx0, rw) - cellPx * 0.18, bandY);
            ctx.stroke();

            ctx.strokeStyle = 'rgba(51, 38, 32, 0.34)';
            ctx.lineWidth = Math.max(0.75, cellPx * 0.017);
            ctx.beginPath();
            const step = Math.max(5.5, cellPx * 0.15);
            let idx = 0;
            for (let sx = rx0 + cellPx * 0.22; sx < rx0 + rw - cellPx * 0.22; sx += step) {
              const hh = hash2d(idx + layer * 41 + rm.x0, isSouth ? 91 : 37);
              const jx = ((hh % 5) - 2) * 0.6;
              const len = ((rh * 0.45) / layersPerSlope) * (0.72 + ((hh >>> 4) % 30) / 100);
              ctx.moveTo(sx + jx, bandY - dir * len * 0.85);
              ctx.lineTo(sx - jx * 0.5, bandY + dir * 1.3);
              idx++;
            }
            ctx.stroke();
          }
        }
      }

      ctx.strokeStyle = PAWN_PAL.ink;
      ctx.lineWidth = lw * 1.15;
      ctx.strokeRect(rx0, ry0, rw, rh);

      // 東西山牆壓竹封邊條
      for (const gx of [rx0, rx0 + rw - cellPx * 0.20]) {
        ctx.fillStyle = '#c5b08a';
        ctx.fillRect(gx, ry0, cellPx * 0.20, rh);
        ctx.strokeRect(gx, ry0, cellPx * 0.20, rh);
      }

      // 各院門出簷門亭小罩棚
      for (const dp of doors) {
        const dx = dp[0];
        const dy = dp[1];
        const dc = microToScreen(dx, dy);
        ctx.fillStyle = '#d3c2a1';
        ctx.strokeStyle = PAWN_PAL.ink;
        ctx.lineWidth = lw;
        if (dx === rm.x0) {
          ctx.fillRect(rx0 - cellPx * 0.26, dc.y - cellPx * 0.44, cellPx * 0.28, cellPx * 0.88);
          ctx.strokeRect(rx0 - cellPx * 0.26, dc.y - cellPx * 0.44, cellPx * 0.28, cellPx * 0.88);
        } else if (dx === rm.x1) {
          ctx.fillRect(rx0 + rw - 2, dc.y - cellPx * 0.44, cellPx * 0.28, cellPx * 0.88);
          ctx.strokeRect(rx0 + rw - 2, dc.y - cellPx * 0.44, cellPx * 0.28, cellPx * 0.88);
        } else if (dy === rm.y1) {
          ctx.fillRect(dc.x - cellPx * 0.44, ry0 - cellPx * 0.26, cellPx * 0.88, cellPx * 0.28);
          ctx.strokeRect(dc.x - cellPx * 0.44, ry0 - cellPx * 0.26, cellPx * 0.88, cellPx * 0.28);
        } else if (dy === rm.y0) {
          ctx.fillRect(dc.x - cellPx * 0.44, ry0 + rh - 2, cellPx * 0.88, cellPx * 0.28);
          ctx.strokeRect(dc.x - cellPx * 0.44, ry0 + rh - 2, cellPx * 0.88, cellPx * 0.28);
        }
      }

      // 中央竹脊大樑＋藤篾縛結＋南北茅草鬚邊
      const ridgeH = cellPx * 0.22;
      ctx.fillStyle = '#e7d9bd';
      ctx.fillRect(rx0 - 2, midY - ridgeH / 2, rw + 4, ridgeH);
      ctx.strokeRect(rx0 - 2, midY - ridgeH / 2, rw + 4, ridgeH);

      if (cellPx >= 14) {
        ctx.strokeStyle = 'rgba(51, 38, 32, 0.48)';
        ctx.lineWidth = lw * 0.65;
        ctx.beginPath();
        ctx.moveTo(rx0, midY);
        ctx.lineTo(rx0 + rw, midY);
        ctx.stroke();

        ctx.strokeStyle = PAWN_PAL.ink;
        ctx.lineWidth = Math.max(0.8, cellPx * 0.017);
        ctx.beginPath();
        const fStep = Math.max(6, cellPx * 0.16);
        let fi = 0;
        for (let sx = rx0 + cellPx * 0.25; sx < rx0 + rw - cellPx * 0.25; sx += fStep) {
          const hh = hash2d(fi + rm.x0, 503);
          const tuft = cellPx * (0.045 + (hh % 4) * 0.015);
          ctx.moveTo(sx, ry0);
          ctx.lineTo(sx + ((hh % 3) - 1), ry0 - tuft);
          ctx.moveTo(sx, ry0 + rh);
          ctx.lineTo(sx + (((hh >>> 2) % 3) - 1), ry0 + rh + tuft);
          fi++;
        }
        ctx.stroke();

        const ties = Math.max(4, Math.round((rm.x1 - rm.x0) * 1.2));
        for (let i = 0; i <= ties; i++) {
          const tx = rx0 + (rw * (i + 0.35)) / (ties + 0.7);
          ctx.fillStyle = '#9c7850';
          ctx.strokeStyle = PAWN_PAL.ink;
          ctx.lineWidth = 0.9;
          ctx.fillRect(tx - cellPx * 0.05, midY - ridgeH * 0.56, cellPx * 0.10, ridgeH * 1.12);
          ctx.strokeRect(tx - cellPx * 0.05, midY - ridgeH * 0.56, cellPx * 0.10, ridgeH * 1.12);
        }
      }
      ctx.restore();
    }

    // 繪製掀起屋頂室內的睡榻標籤
    if (cellPx >= 12) {
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
    void nowMs;
  }

  function rx1Or(x0, w) {
    return x0 + w;
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

  function drawActors(nowMs) {
    // 繪製本尊尋路預覽線（行走為淡墨細虛線，雙擊疾奔為鮮明硃砂快步線）
    const me = actors.find((a) => a.is_player);
    const meSt = me ? renderActors.get(me.id) : null;
    const isMeRunning = !!(meSt ? meSt.running : (me && me.running));
    const drawPath = (meSt && meSt.path && meSt.path.length > 0) ? meSt.path : (me && me.path ? me.path : []);
    if (me && drawPath.length > 0) {
      const cur = meSt || { x: me.px, y: me.py };
      ctx.save();
      ctx.strokeStyle = isMeRunning ? 'rgba(139, 32, 32, 0.82)' : 'rgba(107, 90, 71, 0.55)';
      ctx.lineWidth = isMeRunning ? 2.6 : 1.8;
      ctx.setLineDash(isMeRunning ? [7, 3] : [4, 5]);
      ctx.beginPath();
      const s0 = microToScreen(cur.x, cur.y);
      ctx.moveTo(s0.x, s0.y);
      for (const [px, py] of drawPath) {
        const sp = microToScreen(px, py);
        ctx.lineTo(sp.x, sp.y);
      }
      ctx.stroke();
      ctx.restore();
    }

    const isMacroView = cellPx < 5.0;
    const isNearView = cellPx >= 12.0;

    if (isNearView) {
      const renderItems = [];
      const halfW = Math.min(40, Math.ceil(viewportW / cellPx / 2) + 2);
      const halfH = Math.min(40, Math.ceil(viewportH / cellPx / 2) + 2);
      const cx = Math.round(camX);
      const cy = Math.round(camY);

      for (let my = cy - halfH; my <= cy + halfH; my++) {
        for (let mx = cx - halfW; mx <= cx + halfW; mx++) {
          const c = cellMap.get(`${mx},${my}`);
          if (!c || !c.p) continue;
          const sp = microToScreen(mx, my);
          if (sp.x < -70 || sp.x > viewportW + 70 || sp.y < -70 || sp.y > viewportH + 70) continue;
          renderItems.push({ kind: 'prop', sortY: sp.y, rank: 0, mx, my, pType: c.p, sp });
        }
      }

      const labelDrawList = [];
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
        if (sp.x < -60 || sp.x > viewportW + 60 || sp.y < -60 || sp.y > viewportH + 60) continue;
        if (isActorSleepingInBed(a, pos)) continue;
        const rank = a.is_player ? 2 : 1;
        renderItems.push({ kind: 'actor', sortY: sp.y, rank, a, pos, sp });
        const badge = a.pop || a.carry;
        if (badge && getRoomRoofAlphaAt(pos.x, pos.y) <= 0.5) {
          labelDrawList.push({ a, sp, badge });
        }
      }

      renderItems.sort((u, v) => (u.sortY - v.sortY) || (u.rank - v.rank));

      for (const item of renderItems) {
        if (item.kind === 'prop') {
          const h = hash2d(item.mx, item.my);
          const sp = item.sp;
          if (item.pType === 'bamboo') {
            const baseRot = ((h % 628) / 100);
            const remShake = bambooShakeMap.get(`${item.mx},${item.my}`) || 0;
            const shakeAng = remShake > 0 ? Math.sin(nowMs * 0.06) * 0.11 * (remShake / 0.28) : 0;
            drawPawnPart(pawnImgs.bamboo, sp.x, sp.y, cellPx * 1.54, cellPx * 1.62, 0.50, 0.50, baseRot + shakeAng);
          } else if (item.pType === 'stone') {
            const scale = 0.84 + ((h % 20) / 100);
            ctx.fillStyle = 'rgba(44, 30, 15, 0.08)';
            ctx.beginPath();
            ctx.arc(sp.x + 2, sp.y + 3, cellPx * 0.34 * scale, 0, Math.PI * 2);
            ctx.fill();
            drawPawnPart(pawnImgs.rock, sp.x, sp.y, cellPx * 0.88 * scale, cellPx * 0.94 * scale, 0.50, 0.50, ((h >>> 5) % 628) / 100);
          } else if (item.pType === 'shoot') {
            ctx.fillStyle = 'rgba(44, 30, 15, 0.08)';
            ctx.beginPath();
            ctx.ellipse(sp.x, sp.y + cellPx * 0.09, cellPx * 0.24, cellPx * 0.14, 0, 0, Math.PI * 2);
            ctx.fill();
            const shImg = pawnImgs.shoot;
            if (isDrawableReady(shImg)) {
              drawPawnPart(shImg, sp.x, sp.y - cellPx * 0.04, cellPx * 0.84, cellPx * 0.84, 0.50, 0.50, ((h % 9) - 4) * 0.06);
            } else {
              ctx.fillStyle = '#9ab07a';
              ctx.beginPath();
              ctx.arc(sp.x, sp.y, Math.max(3, cellPx * 0.18), 0, Math.PI * 2);
              ctx.fill();
            }
          } else if (item.pType === 'herb') {
            ctx.fillStyle = 'rgba(44, 30, 15, 0.08)';
            ctx.beginPath();
            ctx.ellipse(sp.x, sp.y + cellPx * 0.08, cellPx * 0.26, cellPx * 0.15, 0, 0, Math.PI * 2);
            ctx.fill();
            drawPawnPart(pawnImgs.wildHerb, sp.x, sp.y, cellPx * 0.90, cellPx * 0.90, 0.50, 0.50, ((h % 11) - 5) * 0.14);
          }
        } else {
          if (item.a.faction === 'wild_beast') {
            drawBaimiaoBeast(item.a, item.pos, item.sp);
          } else {
            drawBaimiaoHuman(item.a, item.pos, item.sp, nowMs);
          }
        }
      }

      // 飄落竹葉（白描嫩葉）
      for (const lp of leafParticles) {
        const lpt = microToScreen(lp.x, lp.y);
        ctx.save();
        ctx.globalAlpha = Math.max(0, lp.life / 0.62);
        ctx.translate(lpt.x, lpt.y);
        ctx.rotate(lp.rot);
        ctx.fillStyle = '#c5d3b5';
        ctx.strokeStyle = PAWN_PAL.ink;
        ctx.lineWidth = 0.8;
        ctx.beginPath();
        ctx.ellipse(0, 0, 4.8, 1.9, 0, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
        ctx.restore();
      }

      // 近景施工與採伐標記置於竹叢之後顯示（附宣紙底角章，避免被茂密竹葉遮擋）
      for (let my = cy - halfH; my <= cy + halfH; my++) {
        for (let mx = cx - halfW; mx <= cx + halfW; mx++) {
          const c = cellMap.get(`${mx},${my}`);
          if (!c || !c.b) continue;
          const sp = microToScreen(mx, my);
          if (sp.x < -50 || sp.x > viewportW + 50 || sp.y < -50 || sp.y > viewportH + 50) continue;
          const isChop = c.b.k === 'chop' || c.b.k === 'decon';
          const color = isChop ? '#8b2020' : '#2a4a6b';
          const x0 = sp.x - cellPx / 2;
          const y0 = sp.y - cellPx / 2;
          ctx.save();
          ctx.strokeStyle = color;
          ctx.lineWidth = 1.6;
          ctx.setLineDash([4, 3]);
          ctx.strokeRect(x0 + 2, y0 + 2, cellPx - 4, cellPx - 4);
          ctx.setLineDash([]);
          if (cellPx >= 16) {
            const tag = c.b.k === 'chop' ? '伐' : (c.b.k === 'decon' ? '拆' : '築');
            ctx.fillStyle = 'rgba(247, 240, 227, 0.92)';
            ctx.strokeStyle = color;
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.roundRect(sp.x - 8, sp.y - 8, 16, 16, 3);
            ctx.fill();
            ctx.stroke();
            ctx.fillStyle = color;
            ctx.font = 'bold 10px "Noto Sans TC", sans-serif';
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText(tag, sp.x, sp.y + 0.5);
          }
          ctx.restore();
        }
      }

      // 所有角色頭頂浮籤永遠最後繪製，防止被竹叢與人物遮擋
      const S = Math.max(15.5, cellPx * 0.84);
      for (const lbl of labelDrawList) {
        ctx.font = '600 11px "Noto Sans TC", sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        const tw = ctx.measureText(lbl.badge).width + 10;
        const by = lbl.sp.y - S * 0.58 - 10;
        ctx.fillStyle = 'rgba(247, 240, 227, 0.94)';
        ctx.strokeStyle = lbl.a.pop ? '#8b2020' : '#a89880';
        ctx.lineWidth = 1;
        ctx.fillRect(lbl.sp.x - tw / 2, by - 8, tw, 16);
        ctx.strokeRect(lbl.sp.x - tw / 2, by - 8, tw, 16);
        ctx.fillStyle = lbl.a.pop ? '#8b2020' : '#2c1e0f';
        ctx.fillText(lbl.badge, lbl.sp.x, by + 0.5);
      }
      return;
    }

    // 中景過渡（5 <= cellPx < 12）與遠景鳥瞰（cellPx < 5）：乾淨無字宣紙圓章與方位定位標
    const sortedActors = [...actors].sort((a, b) => {
      const rank = (x) => (x.is_player ? 3 : (x.faction === 'wild_beast' ? 1 : 2));
      return rank(a) - rank(b);
    });

    for (const a of sortedActors) {
      const pos = renderActors.get(a.id) || { x: a.px, y: a.py, facing: a.facing || 0 };
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
    ctx.fillStyle = '#efe8d8';
    ctx.fillRect(0, 0, viewportW, viewportH);
    drawMacroBasemap();
    drawMicroLayer(nowMs);
    drawActors(nowMs);
    drawBuildingRoofsB(dt, nowMs);
    drawMountainRain(nowMs);

    if (window.SwSkyTime) {
      const lights = [];
      const effW = getEffectiveWeather();
      if (cellPx >= 5.0) {
        for (const c of cellMap.values()) {
          if (c.s !== 'campfire' && c.s !== 'workbench') continue;
          const p = microToScreen(c.x, c.y);
          if (p.x < -220 || p.x > viewportW + 220 || p.y < -220 || p.y > viewportH + 220) continue;
          const wetCamp = c.s === 'campfire' && effW.raining && !indoorCellSet.has(`${c.x},${c.y}`);
          lights.push({
            x: p.x,
            y: p.y,
            radius: cellPx * (c.s === 'campfire' ? (wetCamp ? 2.2 : 5.6) : 2.5),
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

  function handleTapCell(sx, sy) {
    const { mx, my, fx, fy } = screenToMicro(sx, sy);

    // 點中人物／異獸（同時比對渲染平滑位置）／自然物／設施／地面物品／溪流時，直接展開接觸物件插座卡
    const hitActor = actors.find((a) => {
      if (a.is_player) return false;
      const st = renderActors.get(a.id);
      const ax = st ? st.x : a.px;
      const ay = st ? st.y : a.py;
      return Math.hypot(ax - fx, ay - fy) <= 0.95 || Math.hypot(a.px - mx, a.py - my) <= 1.05;
    });
    const cell = cellMap.get(`${mx},${my}`);
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

    const me = actors.find((a) => a.is_player);
    const meSt = me ? renderActors.get(me.id) : null;
    const moveSeq = ++meMoveSeq;
    meMoveLockUntil = performance.now() + 320;
    if (meSt) {
      meSt.running = isDoubleTap;
      meSt.speed = isDoubleTap ? 3.8 : 1.35;
      const preview = findClientPath(Math.round(meSt.x), Math.round(meSt.y), mx, my);
      if (preview && preview.length > 0) {
        meSt.path = trimPassedWaypoints(preview, meSt.x, meSt.y);
      }
    }

    sendCmd({ cmd: 'act', mx, my, action: 'move', run: isDoubleTap }).then((r) => {
      if (moveSeq !== meMoveSeq) return;
      if (r && !r.ok) {
        if (meSt) meSt.path = [];
        if (r.msg) showToast(r.msg);
      } else if (r && r.ok && Array.isArray(r.path) && meSt) {
        if (typeof r.px === 'number' && typeof r.py === 'number') {
          meSt.srvX = r.px;
          meSt.srvY = r.py;
        }
        meSt.path = trimPassedWaypoints(r.path, meSt.x, meSt.y);
        meSt.speed = r.speed || (isDoubleTap ? 3.8 : 1.35);
        meSt.running = !!r.running;
        meMoveLockUntil = performance.now() + 180;
      }
    });
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
      loadSubstrateMeta();
      setInterval(() => pollState(false), 110);
      requestAnimationFrame(frame);
    }
    pollState(true);
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
