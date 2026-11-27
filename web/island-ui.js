/* 《奇點世界・銀鷗外島》三區鎖定前端控制器與三尺度彩色輿圖渲染器 */
(function () {
  'use strict';

  var appState = {
    authed: false,
    authMode: 'login',
    playerId: '',
    view: null,
    mapData: null,
    themeOverride: null, // null = follow server clock, 'day' | 'night'
    selectedRace: '人類',
    selectedOrigin: 'smith_apprentice',
    activeSheet: null, // 'talk' | 'self' | 'bag' | 'recap' | null
    talkNpc: null,
    talkHistory: {}, // npcId -> [{ kind: 'npc'|'me'|'overheard', text }]
    mapOpen: false,
    mapScaleMode: 'local', // 'local' | 'island'
    asideMode: 'local', // 'local' | 'island'
    mobileMapCollapsed: false,
    moveInFlight: false,
    lastCellId: null,
    lastNodeId: null,
    crowdExpanded: false,
    bootEpoch: 0,
    persistSeq: 0,
    serverOffsetSec: 0,
    pulseInFlight: false,
    lastAppliedSlot: -1,
    lastAppliedIsDark: null,
    lastAppliedExposure: '',
    cam: { cx: 634, cy: 413, pxPerTile: 13.0, followPlayer: true, dragging: false, lastX: 0, lastY: 0, downX: 0, downY: 0, hitTargets: [] },
    asideHitTargets: [],
    dngCam: { cx: null, cy: null, zoom: 1.0 },
    walkAnim: {
      active: false,
      inFlight: false,
      planInFlight: false,
      walkSeq: 0,
      displayPos: null,
      fullPath: [],
      segIdx: 0,
      srvIndex: 0,
      srvDone: true,
      pendingIndoor: null,
      lastFrameTs: 0,
      lastSyncTs: 0,
      rafId: 0
    },
    terrainDigest: '',
    terrainRefreshInFlight: false
  };

  function byId(id) {
    return document.getElementById(id);
  }

  function setTextIfChanged(el, txt) {
    if (el && el.textContent !== txt) el.textContent = txt;
  }

  function setHtmlIfChanged(el, html) {
    if (el && el.innerHTML !== html) el.innerHTML = html;
  }

  function pad2(n) {
    return n < 10 ? ('0' + n) : String(n);
  }

  var BELL_12_NAMES = [
    '子夜沉鐘', '殘夜二鐘', '破曉初鐘', '晨鐘二響', '晨鐘三響', '日高四響',
    '正午鐘響', '午後初鐘', '申時二鐘', '黃昏鐘響', '暮色二鐘', '入夜初鐘'
  ];

  var TIDE_48_SHORT_NAMES = [
    '夜央', '深宵', '寒更', '沉夜', '更深', '蒼暝', '夜闌', '曉星', '殘夜', '將曉', '拂曉', '破曉',
    '晨微', '朝露', '晨光', '朝日', '早潮', '朝晴', '晝前', '晴嵐', '隅中', '向午', '日高', '照融',
    '正晝', '日盛', '日中', '日斜', '午後', '日昳', '晚晴', '斜陽', '落照', '夕照', '夕景', '黃昏',
    '日暮', '暮色', '夕霧', '薄暮', '初夜', '夜幕', '星垂', '沉冥', '夜靜', '寒夜', '夜半', '深更'
  ];

  var SKY_TIDE_48_PALETTE = [
    { top: [13, 17, 26],    mid: [18, 24, 34],    bot: [25, 33, 45] },
    { top: [14, 18, 27],    mid: [19, 25, 35],    bot: [26, 34, 46] },
    { top: [14, 19, 28],    mid: [20, 26, 36],    bot: [27, 35, 47] },
    { top: [15, 20, 30],    mid: [21, 28, 38],    bot: [28, 36, 49] },
    { top: [16, 21, 32],    mid: [22, 29, 40],    bot: [30, 38, 51] },
    { top: [17, 23, 35],    mid: [24, 31, 43],    bot: [32, 41, 54] },
    { top: [19, 25, 38],    mid: [26, 34, 47],    bot: [35, 44, 58] },
    { top: [22, 29, 43],    mid: [30, 38, 52],    bot: [40, 50, 64] },
    { top: [26, 34, 50],    mid: [35, 44, 60],    bot: [47, 57, 72] },
    { top: [32, 42, 60],    mid: [43, 53, 70],    bot: [58, 68, 84] },
    { top: [40, 52, 72],    mid: [54, 66, 84],    bot: [76, 84, 98] },
    { top: [192, 208, 218], mid: [210, 220, 224], bot: [228, 224, 214] },
    { top: [202, 216, 224], mid: [218, 228, 232], bot: [234, 236, 232] },
    { top: [210, 222, 228], mid: [224, 232, 235], bot: [238, 241, 239] },
    { top: [216, 226, 228], mid: [229, 236, 236], bot: [241, 243, 240] },
    { top: [220, 228, 225], mid: [232, 237, 233], bot: [243, 244, 238] },
    { top: [224, 229, 222], mid: [235, 238, 230], bot: [245, 244, 236] },
    { top: [227, 230, 219], mid: [237, 238, 228], bot: [246, 244, 235] },
    { top: [229, 229, 216], mid: [239, 238, 226], bot: [247, 245, 235] },
    { top: [231, 228, 213], mid: [240, 237, 224], bot: [248, 245, 235] },
    { top: [232, 227, 210], mid: [241, 237, 223], bot: [248, 245, 235] },
    { top: [233, 226, 207], mid: [242, 236, 222], bot: [249, 245, 236] },
    { top: [234, 225, 204], mid: [243, 236, 221], bot: [249, 246, 236] },
    { top: [235, 224, 202], mid: [244, 236, 220], bot: [250, 246, 236] },
    { top: [235, 224, 200], mid: [244, 235, 218], bot: [250, 245, 235] },
    { top: [235, 223, 198], mid: [244, 234, 216], bot: [250, 244, 233] },
    { top: [234, 221, 196], mid: [243, 233, 214], bot: [249, 243, 231] },
    { top: [233, 219, 193], mid: [242, 231, 212], bot: [249, 242, 229] },
    { top: [232, 217, 190], mid: [241, 229, 209], bot: [248, 240, 226] },
    { top: [231, 214, 187], mid: [240, 227, 206], bot: [247, 238, 223] },
    { top: [230, 211, 183], mid: [239, 225, 203], bot: [247, 237, 220] },
    { top: [229, 208, 180], mid: [238, 222, 199], bot: [246, 235, 217] },
    { top: [227, 203, 174], mid: [236, 218, 194], bot: [245, 232, 212] },
    { top: [224, 196, 166], mid: [234, 212, 186], bot: [243, 227, 205] },
    { top: [220, 188, 157], mid: [231, 204, 177], bot: [241, 221, 197] },
    { top: [215, 179, 148], mid: [227, 196, 167], bot: [238, 214, 188] },
    { top: [206, 168, 140], mid: [220, 186, 158], bot: [233, 204, 178] },
    { top: [194, 156, 134], mid: [210, 174, 148], bot: [225, 192, 166] },
    { top: [52, 54, 76],    mid: [66, 66, 86],    bot: [88, 76, 90] },
    { top: [42, 48, 68],    mid: [54, 58, 78],    bot: [72, 70, 84] },
    { top: [34, 42, 60],    mid: [45, 52, 69],    bot: [60, 63, 76] },
    { top: [28, 36, 52],    mid: [38, 45, 61],    bot: [51, 56, 70] },
    { top: [23, 30, 45],    mid: [32, 39, 53],    bot: [44, 50, 63] },
    { top: [20, 26, 39],    mid: [27, 34, 47],    bot: [38, 45, 57] },
    { top: [17, 23, 34],    mid: [24, 30, 42],    bot: [33, 40, 52] },
    { top: [15, 20, 31],    mid: [21, 27, 38],    bot: [29, 37, 49] },
    { top: [14, 18, 28],    mid: [19, 25, 35],    bot: [27, 35, 46] },
    { top: [13, 17, 26],    mid: [18, 24, 34],    bot: [25, 33, 45] }
  ];

  function lerpRgb(c0, c1, t) {
    return [
      Math.round(c0[0] + (c1[0] - c0[0]) * t),
      Math.round(c0[1] + (c1[1] - c0[1]) * t),
      Math.round(c0[2] + (c1[2] - c0[2]) * t)
    ];
  }

  function rgbStr(c) {
    return 'rgb(' + c[0] + ', ' + c[1] + ', ' + c[2] + ')';
  }

  function formatSolarCalendar(days) {
    var d = Math.max(0, Math.floor(days));
    var rem = ((d % 364) + 364) % 364;
    var season = '初春';
    var dayIn = rem + 1;
    if (rem >= 91 && rem < 182) {
      season = '仲夏';
      dayIn = rem - 90;
    } else if (rem >= 182 && rem < 273) {
      season = '金秋';
      dayIn = rem - 181;
    } else if (rem >= 273) {
      season = '寒冬';
      dayIn = rem - 272;
    }
    var yr = Math.floor(d / 364) + 312;
    return '太陽曆 ' + yr + ' 年・' + season + '・' + dayIn + ' 日';
  }

  function esc(str) {
    return String(str == null ? '' : str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function syncServerClockOffset(unixSec, unixMs) {
    var srvSec = (typeof unixMs === 'number' && unixMs > 0)
      ? (unixMs / 1000)
      : (typeof unixSec === 'number' && unixSec > 0 ? unixSec : 0);
    if (!srvSec) return;
    var cand = srvSec - (Date.now() / 1000);
    if (appState.serverOffsetSec === 0 || Math.abs(cand - appState.serverOffsetSec) > 1.8) {
      appState.serverOffsetSec = cand;
    } else if (cand > appState.serverOffsetSec) {
      appState.serverOffsetSec += (cand - appState.serverOffsetSec) * 0.25;
    }
  }

  var toastTimer = null;
  function showToast(msg) {
    var t = byId('oi-toast');
    if (!t || !msg) return;
    t.textContent = msg;
    t.hidden = false;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function () {
      t.hidden = true;
    }, 2800);
  }

  function apiJson(url, method, bodyObj) {
    var opts = {
      method: method || 'GET',
      credentials: 'same-origin',
      headers: {}
    };
    if (bodyObj) {
      opts.headers['Content-Type'] = 'application/json';
      opts.body = JSON.stringify(bodyObj);
    }
    return fetch(url, opts).then(function (r) {
      return r.json().catch(function () {
        return { ok: false, msg: '海風吹散了回音，請再試一次。' };
      });
    });
  }

  function applySkyTidePalette(hourFloat, exposure) {
    var effHour = hourFloat;
    var effExp = exposure || 'outdoor';
    if (appState.themeOverride === 'night') {
      effHour = 22.0;
      effExp = 'outdoor';
    } else if (appState.themeOverride === 'day') {
      effHour = 12.0;
      effExp = 'outdoor';
    }
    var slotIdx = ((Math.floor(effHour * 2) % 48) + 48) % 48;
    if (
      slotIdx === appState.lastAppliedSlot &&
      effExp === appState.lastAppliedExposure &&
      appState.lastAppliedIsDark !== null
    ) {
      return;
    }

    var root = document.documentElement;
    var isDark = true;
    var bgPage, bgSurf, bgElev, bgChip, bgChipHover, inkMain, inkSub, inkMuted, borderCol;
    var accentMain, accentMainInk, accentGold, accentJade;

    if (effExp === 'sealed') {
      isDark = true;
      bgPage = '#15120f';
      bgSurf = '#1e1915';
      bgElev = '#27211c';
      bgChip = '#322a22';
      bgChipHover = '#3f352b';
      inkMain = '#ebe2d3';
      inkSub = '#c2b5a5';
      inkMuted = '#aa9d8e';
      borderCol = '#3d342a';
      accentMain = '#f09c76';
      accentMainInk = '#1a130e';
      accentGold = '#efc268';
      accentJade = '#76cfa8';
    } else {
      var entry = SKY_TIDE_48_PALETTE[slotIdx] || SKY_TIDE_48_PALETTE[24];
      var mid = entry.mid;
      var top = entry.top;
      var bot = entry.bot;
      if (effExp === 'windowed') {
        var winLum = (0.2126 * mid[0] + 0.7152 * mid[1] + 0.0722 * mid[2]) / 255;
        if (winLum < 0.50) {
          mid = lerpRgb(mid, [29, 23, 18], 0.55);
          top = lerpRgb(top, [23, 18, 14], 0.55);
          bot = lerpRgb(bot, [38, 30, 24], 0.55);
        } else {
          mid = lerpRgb(mid, [242, 235, 221], 0.45);
          top = lerpRgb(top, [234, 226, 210], 0.45);
          bot = lerpRgb(bot, [248, 243, 232], 0.45);
        }
      }
      var lum = (0.2126 * mid[0] + 0.7152 * mid[1] + 0.0722 * mid[2]) / 255;
      isDark = lum < 0.50;
      var warmBias = (slotIdx >= 30 && slotIdx <= 37) ? Math.min(1, (slotIdx - 30) / 7) : 0;
      var nightDepth = Math.max(0, Math.min(1, (0.42 - lum) / 0.30));

      bgPage = rgbStr(mid);
      bgSurf = rgbStr(lerpRgb(mid, isDark ? [34, 29, 24] : [252, 249, 242], isDark ? 0.36 : 0.48));
      bgElev = rgbStr(lerpRgb(bot, isDark ? [44, 38, 32] : [255, 253, 248], 0.46));
      bgChip = rgbStr(lerpRgb(top, isDark ? [54, 46, 38] : [224, 214, 198], 0.44));
      bgChipHover = rgbStr(lerpRgb(top, isDark ? [68, 58, 48] : [210, 198, 180], 0.52));

      if (isDark) {
        inkMain = rgbStr(lerpRgb([244, 238, 226], [230, 223, 210], nightDepth));
        inkSub = rgbStr(lerpRgb([216, 212, 206], [184, 190, 198], nightDepth));
        inkMuted = rgbStr(lerpRgb([206, 202, 196], [164, 172, 182], nightDepth));
        borderCol = rgbStr(lerpRgb(bot, [92, 84, 76], 0.45));
        accentMain = '#f4a27e';
        accentMainInk = '#1a130e';
        accentGold = '#f0c46c';
        accentJade = '#7cd4b8';
      } else {
        inkMain = rgbStr(lerpRgb([32, 26, 18], [34, 18, 10], warmBias));
        inkSub = rgbStr(lerpRgb([70, 60, 48], [54, 30, 16], warmBias));
        inkMuted = rgbStr(lerpRgb([88, 76, 62], [58, 32, 18], warmBias));
        borderCol = rgbStr(lerpRgb(top, [158, 142, 118], 0.45));
        accentMain = '#70240e';
        accentMainInk = '#fdfaf4';
        accentGold = '#593806';
        accentJade = '#174d3c';
      }
    }

    var crossedPolarity = appState.lastAppliedIsDark !== null && appState.lastAppliedIsDark !== isDark;
    if (crossedPolarity) {
      document.body.classList.remove('oi-sky-glide');
    } else if (appState.lastAppliedIsDark !== null) {
      document.body.classList.add('oi-sky-glide');
    }

    root.style.setProperty('--bg-page', bgPage);
    root.style.setProperty('--bg-surface', bgSurf);
    root.style.setProperty('--bg-elevated', bgElev);
    root.style.setProperty('--bg-chip', bgChip);
    root.style.setProperty('--bg-chip-hover', bgChipHover);
    root.style.setProperty('--ink-main', inkMain);
    root.style.setProperty('--ink-sub', inkSub);
    root.style.setProperty('--ink-muted', inkMuted);
    root.style.setProperty('--border-line', borderCol);
    root.style.setProperty('--accent-main', accentMain);
    root.style.setProperty('--accent-main-ink', accentMainInk);
    root.style.setProperty('--accent-gold', accentGold);
    root.style.setProperty('--accent-jade', accentJade);
    root.classList.toggle('sw-night', isDark);
    document.body.classList.toggle('sw-night', isDark);

    var themeMeta = document.querySelector('meta[name="theme-color"]');
    if (themeMeta) themeMeta.setAttribute('content', bgPage);

    var prevDark = appState.lastAppliedIsDark;
    appState.lastAppliedSlot = slotIdx;
    appState.lastAppliedExposure = effExp;
    appState.lastAppliedIsDark = isDark;

    if (crossedPolarity) {
      requestAnimationFrame(function () {
        requestAnimationFrame(function () {
          document.body.classList.add('oi-sky-glide');
        });
      });
      renderDesktopMiniMap();
      if (appState.mapOpen) renderFullscreenMap();
    } else if (prevDark === null) {
      requestAnimationFrame(function () {
        document.body.classList.add('oi-sky-glide');
      });
    }
  }

  function tickLocalIslandClock() {
    var view = appState.view;
    if (!view || !view.clock) return;
    var clk = view.clock;
    var hh = typeof clk.hour === 'number' ? clk.hour : 8;
    var mm = typeof clk.minute === 'number' ? clk.minute : 0;
    var cal = clk.calendar || '';
    var bell = clk.bell || '';
    var slot = typeof clk.tide_slot === 'number' ? clk.tide_slot : (Math.floor(hh * 2) % 48);
    var hourFloat = hh + mm / 60;

    if (typeof clk.epoch_unix === 'number' && clk.scale && clk.scale > 0) {
      var nowServerSec = (Date.now() / 1000) + appState.serverOffsetSec;
      if (nowServerSec >= clk.epoch_unix) {
        var secTotal = (nowServerSec - clk.epoch_unix) * clk.scale;
        var days = Math.floor(secTotal / 86400);
        var secDay = ((secTotal % 86400) + 86400) % 86400;
        hh = Math.floor(secDay / 3600) % 24;
        mm = Math.floor((secDay % 3600) / 60) % 60;
        hourFloat = secDay / 3600;
        slot = Math.floor(secDay / 1800) % 48;
        cal = formatSolarCalendar(days);
        bell = BELL_12_NAMES[Math.floor(hh / 2) % 12];
      }
    }

    var clockLine = (window.innerWidth < 460)
      ? (cal.replace(/^太陽曆\s*/, '').replace(/\s+/g, '') + ' ｜ ' + bell.slice(0, 2) + ' ' + pad2(hh) + ':' + pad2(mm))
      : (cal + ' ｜ ' + bell + ' ' + pad2(hh) + ':' + pad2(mm));
    setTextIfChanged(byId('oi-clock-text'), clockLine);
    var badgeEl = byId('oi-tide-weather');
    if (badgeEl) {
      var exp = clk.exposure || 'outdoor';
      var tName = TIDE_48_SHORT_NAMES[slot] || clk.tide_name || '早潮';
      var badgeTxt = exp === 'sealed'
        ? '坑道幽暗'
        : (tName + '・' + (clk.weather || '晴朗'));
      setTextIfChanged(badgeEl, badgeTxt);
    }
    applySkyTidePalette(hourFloat, clk.exposure || 'outdoor');
  }

  function syncTheme() {
    tickLocalIslandClock();
  }

  function pollIslandPulse() {
    var wa = appState.walkAnim;
    if (!appState.authed || document.visibilityState === 'hidden' || appState.pulseInFlight || wa.active || wa.inFlight || wa.planInFlight) {
      return;
    }
    if (!appState.view || !appState.view.setup_done) {
      return;
    }
    var pulseSeq = wa.walkSeq;
    appState.pulseInFlight = true;
    apiJson('/api/island/pulse', 'GET').then(function (res) {
      if (!res || !res.ok) {
        appState.pulseInFlight = false;
        return;
      }
      syncServerClockOffset(res.server_unix, res.server_unix_ms);
      var epochChanged = res.boot_epoch && appState.bootEpoch && res.boot_epoch !== appState.bootEpoch;
      var seqAdvanced = typeof res.persist_seq === 'number' && res.persist_seq > appState.persistSeq;
      if (epochChanged || seqAdvanced) {
        return apiJson('/api/island/state', 'GET').then(function (st) {
          var waNow = appState.walkAnim;
          if (
            st && st.ok &&
            pulseSeq === waNow.walkSeq &&
            !waNow.active && !waNow.inFlight && !waNow.planInFlight
          ) {
            applyServerView(st, true);
          }
          appState.pulseInFlight = false;
        });
      }
      appState.pulseInFlight = false;
    }).catch(function () {
      appState.pulseInFlight = false;
    });
  }

  /* ═══ 認證與初抵白鷗港登岸冊 ═══ */
  function checkSessionOnBoot() {
    apiJson('/api/rim/me', 'GET').then(function (res) {
      if (res && res.ok) {
        appState.authed = true;
        appState.playerId = res.player_id || '';
        loadWorldAndMap(true);
      } else {
        showAuthScreen();
      }
    });
  }

  function showAuthScreen() {
    byId('oi-auth-screen').hidden = false;
    byId('oi-origin-screen').hidden = true;
    byId('oi-shell').hidden = true;
  }

  function bindAuthEvents() {
    var form = byId('oi-auth-form');
    var toggleBtn = byId('oi-auth-toggle');
    var submitBtn = byId('oi-auth-submit');
    var hintEl = byId('oi-auth-sub');
    var msgEl = byId('oi-auth-msg');

    toggleBtn.addEventListener('click', function () {
      if (appState.authMode === 'login') {
        appState.authMode = 'create';
        submitBtn.textContent = '登記新身世';
        toggleBtn.textContent = '已有帳號？返回登入';
        hintEl.textContent = '建立你在銀鷗外島的旅人通行帳號';
      } else {
        appState.authMode = 'login';
        submitBtn.textContent = '踏入海港';
        toggleBtn.textContent = '初次登島？登記帳號';
        hintEl.textContent = '輸入旅人帳號與通行密語，重返白鷗港';
      }
      msgEl.textContent = '';
    });

    form.addEventListener('submit', function (ev) {
      ev.preventDefault();
      var pid = (byId('oi-auth-id').value || '').trim();
      var pw = byId('oi-auth-pw').value || '';
      if (pid.length < 2 || pw.length < 6) {
        msgEl.textContent = '帳號需至少 2 字，通行密語至少 6 字。';
        return;
      }
      msgEl.textContent = '正在核對港務通行冊……';
      apiJson('/api/rim/auth', 'POST', {
        action: appState.authMode === 'create' ? 'create' : 'login',
        player_id: pid,
        password: pw
      }).then(function (res) {
        if (!res || !res.ok) {
          msgEl.textContent = (res && res.msg) || '無法通過查驗，請確認帳號與密語。';
          return;
        }
        appState.authed = true;
        appState.playerId = res.player_id || pid;
        msgEl.textContent = '';
        loadWorldAndMap(true);
      });
    });

    // 初抵白鷗港登岸冊
    var raceContainer = byId('oi-race-grid');
    raceContainer.addEventListener('click', function (ev) {
      var card = ev.target.closest('[data-race]');
      if (!card) return;
      appState.selectedRace = card.getAttribute('data-race');
      Array.prototype.forEach.call(raceContainer.querySelectorAll('[data-race]'), function (el) {
        el.classList.toggle('active', el === card);
      });
    });

    var originContainer = byId('oi-pack-grid');
    originContainer.addEventListener('click', function (ev) {
      var card = ev.target.closest('[data-pack]');
      if (!card) return;
      appState.selectedOrigin = card.getAttribute('data-pack');
      Array.prototype.forEach.call(originContainer.querySelectorAll('[data-pack]'), function (el) {
        el.classList.toggle('active', el === card);
      });
    });

    byId('oi-origin-form').addEventListener('submit', function (ev) {
      ev.preventDefault();
      var dname = (byId('oi-origin-name').value || '').trim() || appState.playerId;
      var app = (byId('oi-origin-app').value || '').trim();
      var errEl = byId('oi-origin-msg');
      errEl.textContent = '';
      apiJson('/api/island/setup', 'POST', {
        display_name: dname,
        race: appState.selectedRace,
        appearance: app,
        origin_pack: appState.selectedOrigin
      }).then(function (res) {
        if (!res || !res.ok) {
          errEl.textContent = (res && res.msg) || '登記未成，請檢查名號字數。';
          return;
        }
        if (res.view) {
          applyServerView(res.view);
        }
        refreshMapData(true);
        showToast(res.msg || '已踏上白鷗港石棧橋');
      });
    });
  }

  function loadWorldAndMap(centerCamera) {
    apiJson('/api/island/state', 'GET').then(function (res) {
      if (!res || !res.ok) {
        showAuthScreen();
        return;
      }
      applyServerView(res);
      refreshMapData(centerCamera);
    });
  }

  function fitIslandCameraBounds(m, boxW, boxH) {
    var sub = m && m.substrate;
    var b = (m && m.camera_bounds) || [1360, 628, 1506, 792];
    if (!sub) return { cx: 430, cy: 465, pxPerTile: 0.7 };
    var gx0 = (b[0] - sub.wx0) * sub.scale;
    var gy0 = (b[1] - sub.wy0) * sub.scale;
    var gx1 = (b[2] - sub.wx0) * sub.scale;
    var gy1 = (b[3] - sub.wy0) * sub.scale;
    var spanW = Math.max(40, gx1 - gx0);
    var spanH = Math.max(40, gy1 - gy0);
    var cw = Math.max(220, boxW || 380);
    var ch = Math.max(220, boxH || 380);
    var fitPx = Math.max(0.38, Math.min(3.4, Math.min(cw / spanW, ch / spanH) * 0.90));
    return {
      cx: Math.max(0, Math.min(sub.grid_w, (gx0 + gx1) * 0.5)),
      cy: Math.max(0, Math.min(sub.grid_h, (gy0 + gy1) * 0.5)),
      pxPerTile: fitPx
    };
  }

  function pushUniquePathPt(pts, pt) {
    if (!pt) return;
    var last = pts.length ? pts[pts.length - 1] : null;
    if (!last || Math.hypot(pt[0] - last[0], pt[1] - last[1]) > 0.04) {
      pts.push([pt[0], pt[1]]);
    }
  }

  function currentWalkProgress(wa) {
    if (!wa.displayPos || !wa.fullPath || wa.segIdx + 1 >= wa.fullPath.length) {
      return wa.segIdx;
    }
    var p0 = wa.fullPath[wa.segIdx];
    var p1 = wa.fullPath[wa.segIdx + 1];
    var segLen = Math.hypot(p1[0] - p0[0], p1[1] - p0[1]);
    var frac = segLen > 1e-4
      ? Math.min(1.0, Math.hypot(wa.displayPos[0] - p0[0], wa.displayPos[1] - p0[1]) / segLen)
      : 0.0;
    return wa.segIdx + frac;
  }

  function applyWalkMapSync(sync, expectedSeq, isStepResponse) {
    if (!sync || !appState.mapData || !appState.mapData.substrate) return;
    var wa = appState.walkAnim;
    if (typeof expectedSeq === 'number' && expectedSeq !== wa.walkSeq) return;
    var m = appState.mapData;
    var sub = m.substrate;
    var prevInDungeon = !!m.in_dungeon;
    if (typeof sync.current_cell_id === 'number') m.current_cell_id = sync.current_cell_id;
    if (sync.current_node_id) m.current_node_id = sync.current_node_id;
    if (sync.current_node_name != null) m.current_node_name = sync.current_node_name;
    m.in_dungeon = !!sync.in_dungeon;
    if (Array.isArray(sync.nearby_actors)) sub.nearby_actors = sync.nearby_actors;
    if (Array.isArray(sync.proximity_capsules)) sub.proximity_capsules = sync.proximity_capsules;
    if (Array.isArray(sync.pois)) sub.pois = sync.pois;
    if (Array.isArray(sync.rumor_pins)) sub.rumor_pins = sync.rumor_pins;
    if (Array.isArray(sync.building_hits)) sub.building_hits = sync.building_hits;
    if (sync.terrain_digest) appState.terrainDigest = sync.terrain_digest;

    var rawWps = sync.walk_waypoints || [];
    sub.walk_waypoints = rawWps;
    var mappedWps = rawWps.map(function (pt) { return [pt[0] + 0.5, pt[1] + 0.5]; });
    var srvPos = sync.player_pos;
    var largeNonStepJump = !isStepResponse && mappedWps.length === 0 && srvPos && wa.displayPos &&
      Math.hypot(srvPos[0] - wa.displayPos[0], srvPos[1] - wa.displayPos[1]) > 12.0;

    if (!wa.displayPos || m.in_dungeon !== prevInDungeon || largeNonStepJump) {
      if (srvPos) wa.displayPos = [srvPos[0], srvPos[1]];
      wa.fullPath = [];
      wa.segIdx = 0;
      wa.srvIndex = 0;
      wa.srvDone = true;
      wa.pendingIndoor = null;
      wa.active = false;
    } else if (!isStepResponse) {
      var startPt = [wa.displayPos[0], wa.displayPos[1]];
      var pts = [startPt];
      var skipSrvHook = false;
      if (srvPos && mappedWps.length > 0 && Math.hypot(srvPos[0] - startPt[0], srvPos[1] - startPt[1]) <= 2.2) {
        var v1x = srvPos[0] - startPt[0], v1y = srvPos[1] - startPt[1];
        var v2x = mappedWps[0][0] - srvPos[0], v2y = mappedWps[0][1] - srvPos[1];
        if (v1x * v2x + v1y * v2y < -0.01) {
          skipSrvHook = true;
        }
      }
      if (!skipSrvHook && srvPos) pushUniquePathPt(pts, srvPos);
      wa.srvIndex = pts.length - 1;
      for (var mi = 0; mi < mappedWps.length; mi++) {
        pushUniquePathPt(pts, mappedWps[mi]);
      }
      wa.fullPath = pts;
      wa.segIdx = 0;
      wa.srvDone = mappedWps.length === 0;
    } else {
      var matchIdx = -1;
      if (srvPos) {
        for (var k = Math.max(0, wa.srvIndex); k < wa.fullPath.length; k++) {
          if (Math.hypot(wa.fullPath[k][0] - srvPos[0], wa.fullPath[k][1] - srvPos[1]) < 0.15) {
            matchIdx = k;
            break;
          }
        }
      }
      var ptsStep;
      if (matchIdx >= 0) {
        ptsStep = wa.fullPath.slice(0, matchIdx + 1);
        wa.srvIndex = matchIdx;
      } else {
        var keepEnd = Math.max(wa.segIdx + 1, wa.srvIndex);
        ptsStep = wa.fullPath.slice(0, Math.min(wa.fullPath.length, keepEnd + 1));
        if (!ptsStep.length && wa.displayPos) ptsStep = [[wa.displayPos[0], wa.displayPos[1]]];
        if (srvPos) pushUniquePathPt(ptsStep, srvPos);
        wa.srvIndex = Math.max(0, ptsStep.length - 1);
      }
      for (var mj = 0; mj < mappedWps.length; mj++) {
        pushUniquePathPt(ptsStep, mappedWps[mj]);
      }
      wa.fullPath = ptsStep;
      wa.srvDone = mappedWps.length === 0;
    }

    var hasRemainPath = wa.segIdx + 1 < wa.fullPath.length && !m.in_dungeon;
    if (sync.in_indoor && !m.in_dungeon && hasRemainPath) {
      wa.pendingIndoor = {
        in_indoor: true,
        indoor_node_id: sync.indoor_node_id || null,
        indoor_label: sync.indoor_label || null
      };
      sub.in_indoor = false;
      sub.indoor_node_id = null;
      sub.indoor_label = null;
    } else {
      wa.pendingIndoor = null;
      sub.in_indoor = !!sync.in_indoor;
      sub.indoor_node_id = sync.indoor_node_id || null;
      sub.indoor_label = sync.indoor_label || null;
    }

    sub.player_pos = wa.displayPos ? [wa.displayPos[0], wa.displayPos[1]] : srvPos;
    sub.render_waypoints = hasRemainPath ? wa.fullPath.slice(wa.segIdx + 1) : [];
    wa.active = sub.render_waypoints.length > 0;
    if (wa.active) ensureWalkAnimationLoop();

    if (m.in_dungeon !== prevInDungeon || sync.terrain_dirty) {
      if (!appState.terrainRefreshInFlight) {
        refreshMapData(false);
      }
    }
    renderDesktopMiniMap();
    if (appState.mapOpen) renderFullscreenMap();
  }

  function syncWalkStateFromMap(m, centerCamera) {
    var sub = m && m.substrate;
    var wa = appState.walkAnim;
    if (sub && sub.terrain_digest) {
      appState.terrainDigest = sub.terrain_digest;
    }
    if (!sub || !sub.player_pos) {
      wa.active = false;
      wa.fullPath = [];
      wa.segIdx = 0;
      wa.srvIndex = 0;
      wa.srvDone = true;
      wa.pendingIndoor = null;
      return;
    }
    var srvPos = sub.player_pos;
    var mappedWps = (sub.walk_waypoints || []).map(function (pt) { return [pt[0] + 0.5, pt[1] + 0.5]; });
    if (
      !wa.displayPos ||
      centerCamera ||
      m.in_dungeon ||
      (mappedWps.length === 0 && Math.hypot(wa.displayPos[0] - srvPos[0], wa.displayPos[1] - srvPos[1]) > 12.0)
    ) {
      wa.displayPos = [srvPos[0], srvPos[1]];
      if (centerCamera) appState.cam.followPlayer = true;
      var pts = [[wa.displayPos[0], wa.displayPos[1]]];
      wa.srvIndex = 0;
      for (var i = 0; i < mappedWps.length; i++) pushUniquePathPt(pts, mappedWps[i]);
      wa.fullPath = pts;
      wa.segIdx = 0;
      wa.srvDone = mappedWps.length === 0;
    } else {
      var pts2 = [[wa.displayPos[0], wa.displayPos[1]]];
      pushUniquePathPt(pts2, srvPos);
      wa.srvIndex = pts2.length - 1;
      for (var j = 0; j < mappedWps.length; j++) pushUniquePathPt(pts2, mappedWps[j]);
      wa.fullPath = pts2;
      wa.segIdx = 0;
      wa.srvDone = mappedWps.length === 0;
      sub.player_pos = [wa.displayPos[0], wa.displayPos[1]];
    }
    wa.pendingIndoor = null;
    sub.render_waypoints = (wa.segIdx + 1 < wa.fullPath.length && !m.in_dungeon)
      ? wa.fullPath.slice(wa.segIdx + 1)
      : [];
    wa.active = sub.render_waypoints.length > 0;
    if (wa.active) ensureWalkAnimationLoop();
  }

  function stepWalkAnimation(now) {
    var wa = appState.walkAnim;
    wa.rafId = 0;
    if ((!wa.active && !wa.inFlight && !wa.planInFlight) || !appState.mapData || !appState.mapData.substrate) return;
    var sub = appState.mapData.substrate;
    var dt = wa.lastFrameTs > 0 ? Math.min(0.050, Math.max(0.004, (now - wa.lastFrameTs) / 1000)) : 0.016;
    wa.lastFrameTs = now;

    if (wa.active && wa.displayPos && wa.segIdx + 1 < wa.fullPath.length) {
      var feProg = currentWalkProgress(wa);
      var lead = feProg - wa.srvIndex;
      var baseSpeed = 6.0;
      var speed = baseSpeed;
      if (!wa.srvDone && lead > 1.6) {
        speed = baseSpeed * Math.max(0.0, (2.6 - lead) / 1.0);
      } else if (lead < -1.2) {
        speed = baseSpeed + Math.min(5.0, (-lead - 1.2) * 2.2);
      }

      var remain = speed * dt;
      while (remain > 0 && wa.segIdx + 1 < wa.fullPath.length) {
        var target = wa.fullPath[wa.segIdx + 1];
        var dx = target[0] - wa.displayPos[0];
        var dy = target[1] - wa.displayPos[1];
        var dist = Math.hypot(dx, dy);
        if (dist <= remain) {
          wa.displayPos[0] = target[0];
          wa.displayPos[1] = target[1];
          wa.segIdx++;
          remain -= dist;
        } else {
          wa.displayPos[0] += (dx / dist) * remain;
          wa.displayPos[1] += (dy / dist) * remain;
          remain = 0;
        }
      }
      sub.player_pos = [wa.displayPos[0], wa.displayPos[1]];
      sub.render_waypoints = wa.fullPath.slice(wa.segIdx + 1);

      if (appState.mapOpen && appState.mapScaleMode !== 'island' && !appState.cam.dragging) {
        if (appState.cam.followPlayer) {
          var camAlpha = 1 - Math.exp(-10.0 * dt);
          appState.cam.cx += (wa.displayPos[0] - appState.cam.cx) * camAlpha;
          appState.cam.cy += (wa.displayPos[1] - appState.cam.cy) * camAlpha;
        } else {
          var mapWrap = byId('oi-map-canvas');
          if (mapWrap) {
            var z = appState.cam.pxPerTile || 13.0;
            var halfW = (mapWrap.clientWidth * 0.36) / z;
            var halfH = (mapWrap.clientHeight * 0.36) / z;
            var offX = wa.displayPos[0] - appState.cam.cx;
            var offY = wa.displayPos[1] - appState.cam.cy;
            var edgeAlpha = 1 - Math.exp(-6.0 * dt);
            if (Math.abs(offX) > halfW) {
              var tgtCx = wa.displayPos[0] - Math.sign(offX) * halfW;
              appState.cam.cx += (tgtCx - appState.cam.cx) * edgeAlpha;
            }
            if (Math.abs(offY) > halfH) {
              var tgtCy = wa.displayPos[1] - Math.sign(offY) * halfH;
              appState.cam.cy += (tgtCy - appState.cam.cy) * edgeAlpha;
            }
          }
        }
      }

      if (wa.segIdx + 1 >= wa.fullPath.length && wa.srvDone) {
        if (wa.pendingIndoor) {
          sub.in_indoor = !!wa.pendingIndoor.in_indoor;
          sub.indoor_node_id = wa.pendingIndoor.indoor_node_id;
          sub.indoor_label = wa.pendingIndoor.indoor_label;
          wa.pendingIndoor = null;
        }
        sub.render_waypoints = [];
        wa.active = false;
      }

      renderDesktopMiniMap();
      if (appState.mapOpen) renderFullscreenMap();
    } else if (wa.segIdx + 1 >= wa.fullPath.length && wa.srvDone) {
      if (wa.pendingIndoor) {
        sub.in_indoor = !!wa.pendingIndoor.in_indoor;
        sub.indoor_node_id = wa.pendingIndoor.indoor_node_id;
        sub.indoor_label = wa.pendingIndoor.indoor_label;
        wa.pendingIndoor = null;
      }
      sub.render_waypoints = [];
      wa.active = false;
      renderDesktopMiniMap();
      if (appState.mapOpen) renderFullscreenMap();
    }

    if (!wa.srvDone && !wa.inFlight && !wa.planInFlight && now - wa.lastSyncTs >= 220) {
      var postProg = currentWalkProgress(wa);
      var need = Math.ceil(postProg + 1.0) - wa.srvIndex;
      if (need >= 1) {
        var reqSteps = Math.max(1, Math.min(4, need));
        var mySeq = wa.walkSeq;
        wa.inFlight = true;
        wa.lastSyncTs = now;
        apiJson('/api/island/move', 'POST', {
          to: '',
          mode: 'step',
          steps: reqSteps,
          prev_digest: appState.terrainDigest || undefined
        }).then(function (res) {
          if (mySeq !== wa.walkSeq) return;
          wa.inFlight = false;
          if (res && res.view) applyServerView(res.view, true);
          if (res && res.msg && (!res.ok || /停步|沖斷|疫氣|變故|力竭|阻/.test(res.msg))) {
            showToast(res.msg);
          }
          if (res && res.map_sync) {
            applyWalkMapSync(res.map_sync, mySeq, true);
          } else {
            refreshMapData(false);
          }
        }).catch(function () {
          if (mySeq === wa.walkSeq) wa.inFlight = false;
        });
      }
    }

    if (wa.active || wa.inFlight || wa.planInFlight) {
      wa.rafId = requestAnimationFrame(stepWalkAnimation);
    }
  }

  function ensureWalkAnimationLoop() {
    var wa = appState.walkAnim;
    if ((wa.active || wa.inFlight || wa.planInFlight) && !wa.rafId) {
      wa.lastFrameTs = performance.now();
      wa.rafId = requestAnimationFrame(stepWalkAnimation);
    }
  }

  function moveNodeTo(targetNid) {
    if (!targetNid || appState.moveInFlight) return;
    if (appState.view && appState.view.scene && appState.view.scene.node_id === targetNid) return;
    appState.moveInFlight = true;
    apiJson('/api/island/move', 'POST', {
      to: targetNid,
      mode: 'node'
    }).then(function (res) {
      appState.moveInFlight = false;
      if (res && res.view) {
        applyServerView(res.view, false);
      }
      if (res && res.msg && (!res.ok || /停步|沖斷|疫氣|變故|力竭|攔住|阻|海浪|無法|隔開|對峙/.test(res.msg))) {
        showToast(res.msg);
      }
    }).catch(function () {
      appState.moveInFlight = false;
      showToast('腳步略頓，請再點一次。');
    });
  }

  function startGridWalk(opts) {
    if (opts && opts.to) {
      moveNodeTo(opts.to);
    }
  }

  function refreshMapData() {
    if (appState.terrainRefreshInFlight) return;
    appState.terrainRefreshInFlight = true;
    apiJson('/api/island/map', 'GET').then(function (m) {
      appState.terrainRefreshInFlight = false;
      if (!m || !m.ok) return;
      appState.lastCellId = m.current_cell_id;
      appState.lastMapNodeId = m.current_node_id;
      appState.mapData = m;
      renderDesktopMiniMap();
      if (appState.mapOpen) {
        renderFullscreenMap();
      }
    }).catch(function () {
      appState.terrainRefreshInFlight = false;
    });
  }

  function applyServerView(view, isBackgroundSync) {
    if (
      isBackgroundSync &&
      view &&
      view.sync &&
      appState.bootEpoch &&
      view.sync.boot_epoch === appState.bootEpoch &&
      typeof view.sync.persist_seq === 'number' &&
      view.sync.persist_seq < appState.persistSeq
    ) {
      return;
    }
    var prevNid = appState.lastNodeId;
    var nextNid = view && view.scene && view.scene.node_id;
    if (nextNid && nextNid !== prevNid) {
      appState.lastNodeId = nextNid;
      appState.crowdExpanded = false;
    }
    if (view && view.sync) {
      if (view.sync.boot_epoch) appState.bootEpoch = view.sync.boot_epoch;
      if (typeof view.sync.persist_seq === 'number') appState.persistSeq = view.sync.persist_seq;
    }
    if (view && view.clock) {
      syncServerClockOffset(view.clock.server_unix, view.clock.server_unix_ms);
    }
    appState.view = view;
    syncTheme();

    if (!view.setup_done) {
      byId('oi-auth-screen').hidden = true;
      byId('oi-shell').hidden = true;
      byId('oi-origin-screen').hidden = false;
      var nameInp = byId('oi-origin-name');
      if (!nameInp.value) {
        nameInp.value = (view.player && view.player.name) || appState.playerId;
      }
      return;
    }

    byId('oi-auth-screen').hidden = true;
    byId('oi-origin-screen').hidden = true;
    byId('oi-shell').hidden = false;

    renderTopbar(view);
    renderSceneStage(view);
    renderDesktopMiniMap();
    renderPresenceDoors(view);
    renderExitsStrip(view);
    if (appState.mapOpen) {
      renderFullscreenMap();
    }
    if (!isBackgroundSync && appState.activeSheet && appState.activeSheet !== 'talk') {
      renderActiveSheet();
    }
  }

  /* ═══ 三區主畫面渲染 ═══ */
  function renderTopbar(view) {
    if (!view) return;
    tickLocalIslandClock();
  }

  function renderSceneStage(view) {
    var sc = view.scene || {};
    var p = view.player || {};
    setTextIfChanged(byId('oi-scene-title'), sc.name || '');
    setTextIfChanged(byId('oi-copper-pill'), (p.copper || 0) + ' 枚銅幣');

    var cuesEl = byId('oi-body-cues');
    var cues = p.body_cues || [];
    if (cues.length > 0 && !view.skirmish && !view.transit) {
      cuesEl.hidden = false;
      setHtmlIfChanged(
        cuesEl,
        cues.map(function (c) {
          var rowCls = 'oi-body-cue-row' + (c.kind === 'wound' ? ' is-wound' : '');
          var btnHtml = c.action
            ? ('<button type="button" class="oi-body-cue-btn" data-act="' + esc(c.action.id) + '" data-param="' + esc(c.action.param || '') + '">' + esc(c.action.label) + '</button>')
            : '';
          return '<div class="' + rowCls + '"><span>' + esc(c.text) + '</span>' + btnHtml + '</div>';
        }).join('')
      );
    } else {
      cuesEl.hidden = true;
      setHtmlIfChanged(cuesEl, '');
    }

    setTextIfChanged(byId('oi-scene-desc'), sc.desc || '');

    // 短兵相接或車船行進狀態卡（零回合編號、零量化進度條、零連點催行勞作）
    var skWrap = byId('oi-skirmish-wrap');
    if (view.skirmish) {
      var sk = view.skirmish;
      var allMoves = sk.moves || [];
      var mainMovesHtml = allMoves.filter(function (mv) { return mv.id !== 'skirmish_retreat'; }).map(function (mv) {
        return '<button type="button" class="oi-deed-btn" data-act="' + esc(mv.id) + '">' +
          '<strong>' + esc(mv.label) + '</strong>' +
          '<small>' + esc(mv.hint || '') + '</small></button>';
      }).join('');
      var retreatMv = allMoves.find(function (mv) { return mv.id === 'skirmish_retreat'; });
      var retreatHtml = retreatMv
        ? ('<div class="oi-retreat-row"><button type="button" class="oi-deed-btn" data-act="skirmish_retreat"><strong>' + esc(retreatMv.label) + '</strong><small>' + esc(retreatMv.hint || '') + '</small></button></div>')
        : '';
      skWrap.hidden = false;
      setHtmlIfChanged(
        skWrap,
        '<div class="oi-skirmish-card">' +
        '<h3 class="oi-skirmish-title">短兵相接：' + esc(sk.foe_name) + '</h3>' +
        '<div style="font-size:0.9rem;color:var(--ink-sub);">掌中器物：' + esc(p.weapon && p.weapon.name) +
        (sk.staggered ? ' ｜ 對方重心已晃，正可搶攻！' : ' ｜ 留意對方前撲手腕與呼吸空檔') + '</div>' +
        '<div class="oi-skirmish-grid">' + mainMovesHtml + '</div>' +
        retreatHtml +
        '</div>'
      );
    } else if (view.transit) {
      var tr = view.transit;
      var nextStop = tr.next_stop_name || '前方站口';
      var transitCue = tr.alight_next_stop
        ? ('已向駕手示意：抵達前方【' + esc(nextStop) + '】即停靠落腳')
        : ('車船穩步前行，正朝前方【' + esc(nextStop) + '】駛去');
      skWrap.hidden = false;
      setHtmlIfChanged(
        skWrap,
        '<div class="oi-skirmish-card">' +
        '<h3 class="oi-skirmish-title">' + esc(tr.line_name) + '</h3>' +
        '<div style="font-size:0.9rem;color:var(--ink-sub);">' + transitCue + '</div>' +
        '<div class="oi-skirmish-grid">' +
        '<button type="button" class="oi-deed-btn" data-act="transit_alight_next"><strong>下一站停靠下車</strong><small>抵前方站口落腳</small></button>' +
        '<button type="button" class="oi-deed-btn" data-act="carriage_alight"><strong>招呼就地停靠</strong><small>當場改為步行</small></button>' +
        '</div></div>'
      );
    } else {
      skWrap.hidden = true;
      setHtmlIfChanged(skWrap, '');
    }

    // 眼前所見之人（無分類標題，當值對口獨立優先席，人群收合零數字）
    var npcs = sc.npcs || [];
    var overflow = sc.crowd_overflow || [];
    var npcSec = byId('oi-presence-people');
    function renderPersonCard(n) {
      var phaseTag = (n.peer && n.peer.phase_badge) ? ('・成相【' + esc(n.peer.phase_badge) + '】') : '';
      var isStrangerLabel = String(n.name || '').indexOf('陌生的') === 0;
      var rolePart = isStrangerLabel ? esc(n.race) : (esc(n.race) + '・' + esc(n.role));
      return '<button type="button" class="oi-person-btn" data-npc-id="' + esc(n.id) + '" data-npc-name="' + esc(n.name) + '" data-npc-role="' + esc(n.race + '・' + n.role) + '" data-is-player="' + (n.is_player ? '1' : '0') + '">' +
        '<span class="oi-person-top">' + esc(n.name) + '</span>' +
        '<span class="oi-person-sub">' + rolePart + phaseTag + '・' + esc(n.cue) + '</span>' +
        '</button>';
    }
    if ((npcs.length > 0 || overflow.length > 0) && !view.skirmish && !view.transit) {
      npcSec.hidden = false;
      var htmlParts = npcs.map(renderPersonCard);
      if (overflow.length > 0) {
        htmlParts.push(
          '<button type="button" class="oi-person-btn" data-toggle-crowd="1" style="border-style:dashed;">' +
          '<span class="oi-person-top">' +
          (appState.crowdExpanded ? '將視線收回近處 ▴' : '人聲漸雜，後頭還有幾道身影 ▾') +
          '</span>' +
          '<span class="oi-person-sub">' +
          (appState.crowdExpanded ? '專注眼前近處人物' : '走近打量後排身影') +
          '</span></button>'
        );
        if (appState.crowdExpanded) {
          for (var oi = 0; oi < overflow.length; oi++) {
            htmlParts.push(renderPersonCard(overflow[oi]));
          }
        }
      }
      setHtmlIfChanged(npcSec, htmlParts.join(''));
    } else {
      npcSec.hidden = true;
      setHtmlIfChanged(npcSec, '');
    }

    // 眼前所見之具名實物與停靠車船（無分類標題、零動詞任務句）
    var objs = sc.objects || [];
    var objSec = byId('oi-presence-objects');
    if (objs.length > 0 && !view.skirmish && !view.transit) {
      objSec.hidden = false;
      setHtmlIfChanged(objSec, objs.map(function (ob) {
        return '<button type="button" class="oi-object-btn" data-object-id="' + esc(ob.object_id) + '">' +
          '<span class="oi-person-top">' + esc(ob.name) + '</span>' +
          '<span class="oi-person-sub">' + esc(ob.cue || '') + '</span>' +
          '</button>';
      }).join(''));
    } else {
      objSec.hidden = true;
      setHtmlIfChanged(objSec, '');
    }

    // 當下呼吸紀錄（去重正規化；手機主畫面精選最新 2 筆防底部切字，完整紀錄於底欄「近事」查閱）
    var rawBreaths = sc.breaths || [];
    var cleanBreaths = [];
    for (var bi = 0; bi < rawBreaths.length; bi++) {
      var bl = String(rawBreaths[bi] || '').trim();
      if (!bl) continue;
      if (cleanBreaths.length === 0 || cleanBreaths[cleanBreaths.length - 1] !== bl) {
        cleanBreaths.push(bl);
      }
    }
    var maxMainBreaths = (window.innerWidth < 700 || window.innerHeight < 920) ? 2 : 3;
    var recent = cleanBreaths.slice(-maxMainBreaths);
    setHtmlIfChanged(byId('oi-breaths-box'), recent.map(function (line, idx) {
      var cls = idx === recent.length - 1 ? 'oi-breath-item latest' : 'oi-breath-item';
      return '<p class="' + cls + '">' + esc(line) + '</p>';
    }).join(''));
  }

  function cleanExitPillLabel(ex) {
    var shortDest = String(ex.to_name || '').split('・').pop() || ex.to_name || '';
    var cleanDir = String(ex.dir || '').replace(/（[^）]*）/g, '').trim();
    // 消除「訪赤爐莉亞家 · 赤爐莉亞家」「進一號房 · 一號客房」這類重覆前綴
    if (!cleanDir || cleanDir === shortDest || cleanDir.indexOf(shortDest) >= 0 || shortDest.indexOf(cleanDir) >= 0) {
      return shortDest;
    }
    if (/^(訪|進)/.test(cleanDir) && (ex.to_kind === 'house' || ex.to_kind === 'house_vacant' || ex.to_kind === 'inn_room')) {
      return shortDest;
    }
    return cleanDir + ' · ' + shortDest;
  }

  function renderPresenceDoors(view) {
    var doorsEl = byId('oi-presence-doors');
    if (!doorsEl) return;
    if (!view || !view.scene || view.skirmish || view.transit) {
      doorsEl.hidden = true;
      setHtmlIfChanged(doorsEl, '');
      return;
    }
    var mapped = appState.mappedExitIds || {};
    var exits = view.scene.exits || [];
    var unmapped = exits.filter(function (ex) {
      return ex && ex.to && !mapped[ex.to];
    });
    if (!unmapped.length) {
      doorsEl.hidden = true;
      setHtmlIfChanged(doorsEl, '');
      return;
    }
    doorsEl.hidden = false;
    var headLabel = unmapped.some(function (e) { return e.to_kind === 'inn_room'; }) ? '樓上客房' : '此處門戶';
    setHtmlIfChanged(
      doorsEl,
      '<div class="oi-doors-head">' + headLabel + '</div>' +
      '<div class="oi-doors-grid">' +
      unmapped.map(function (ex) {
        var lbl = cleanExitPillLabel(ex);
        return '<button type="button" class="oi-door-chip" data-move-to="' + esc(ex.to) + '" title="' + esc(ex.ambient_cue || ex.to_name) + '">' +
          esc(lbl) + '</button>';
      }).join('') +
      '</div>'
    );
  }

  function renderExitsStrip(view) {
    var strip = byId('oi-exits-strip');
    if (!strip) return;
    if (view.skirmish) {
      strip.hidden = false;
      setHtmlIfChanged(strip, '<span style="font-size:0.86rem;color:var(--accent-main);padding:4px 4px;">短兵相接中，請於上方選擇招式或虛晃退回</span>');
      return;
    }
    if (view.transit) {
      strip.hidden = false;
      var tName = (view.transit && view.transit.line_name) ? ('【' + esc(view.transit.line_name) + '】') : '車船';
      setHtmlIfChanged(strip, '<span style="font-size:0.86rem;color:var(--ink-sub);padding:4px 4px;">正在' + tName + '座艙內，抵達或下一站下車後即可自由邁步</span>');
      return;
    }
    // 靜態手繪圖展開時，不重複顯示底部出口列（把高度讓給多文字區）；僅手機收合地圖或看全島總覽時才顯示備用出口列
    var mapHiddenOrOverview = (appState.mobileMapCollapsed && window.innerWidth < 960) || appState.asideMode === 'island';
    if (!mapHiddenOrOverview) {
      strip.hidden = true;
      setHtmlIfChanged(strip, '');
      return;
    }
    strip.hidden = false;
    var exits = (view.scene && view.scene.exits) || [];
    setHtmlIfChanged(strip, exits.map(function (ex) {
      var label = cleanExitPillLabel(ex);
      if (ex.milestone_zh) {
        label += '〔' + ex.milestone_zh + '〕';
      }
      var cls = ex.kind === 'secret' ? 'oi-exit-pill secret' : 'oi-exit-pill';
      return '<button type="button" class="' + cls + '" data-move-to="' + esc(ex.to) + '" title="' + esc(ex.ambient_cue || ex.to_name) + '">' +
        esc(label) +
        '</button>';
    }).join(''));
  }

  /* ═══ 抽屜面板（當面交談／自身／行囊木箱／近事） ═══ */
  function openTalkSheet(npcId, npcName, npcRole, isPlayer) {
    appState.activeSheet = 'talk';
    appState.talkNpc = { id: npcId, name: npcName, role: npcRole, isPlayer: !!isPlayer };
    byId('oi-sheet-backdrop').hidden = false;
    renderActiveSheet();
    if (!isPlayer && (!appState.talkHistory[npcId] || !appState.talkHistory[npcId].length)) {
      sendTalkRequest('greet', '');
    }
  }

  function openObjectSheet(objectId) {
    appState.activeSheet = 'object';
    appState.activeObjectId = objectId;
    byId('oi-sheet-backdrop').hidden = false;
    renderActiveSheet();
  }

  function openInfoSheet(kind) {
    appState.activeSheet = kind;
    byId('oi-sheet-backdrop').hidden = false;
    renderActiveSheet();
  }

  function closeSheet() {
    appState.activeSheet = null;
    byId('oi-sheet-backdrop').hidden = true;
  }

  function renderActiveSheet() {
    var titleEl = byId('oi-sheet-title');
    var bodyEl = byId('oi-sheet-body');
    var view = appState.view;
    if (!view || !appState.activeSheet) return;
    var p = view.player || {};

    if (appState.activeSheet === 'talk' && appState.talkNpc) {
      var npc = appState.talkNpc;
      titleEl.textContent = npc.name + '（' + npc.role + '）';
      var hist = appState.talkHistory[npc.id] || [];
      var bubbles = hist.map(function (item) {
        var cls = 'oi-talk-bubble ' + (item.kind || 'npc');
        return '<div class="' + cls + '">' + esc(item.text) + '</div>';
      }).join('');
      if (!bubbles) {
        bubbles = npc.isPlayer
          ? '<div class="oi-talk-bubble">對方是同在此處的旅人，可直接開口傳話（加前綴〔矮人語〕／〔精靈語〕可母語密談）、亮物談價或講定點到為止切磋。</div>'
          : '<div class="oi-talk-bubble">正在上前搭話……</div>';
      }
      var allScenePeople = ((view.scene && view.scene.npcs) || []).concat((view.scene && view.scene.crowd_overflow) || []);
      var liveEntry = null;
      for (var pi = 0; pi < allScenePeople.length; pi++) {
        if (allScenePeople[pi].id === npc.id) { liveEntry = allScenePeople[pi]; break; }
      }
      var peer = (liveEntry && liveEntry.peer) || {};
      var peerBtns = [];
      if (peer.in_skirmish) {
        peerBtns.push('<button type="button" class="oi-topic-chip" data-peer-act="peer_assist" data-peer-param="' + esc(npc.id) + '">拔刀相助迎擊' + esc(peer.in_skirmish) + '</button>');
      }
      if (peer.incoming_offer) {
        peerBtns.push('<button type="button" class="oi-topic-chip" data-peer-act="peer_accept_offer" data-peer-param="">點頭付 ' + esc(peer.incoming_offer.price_copper) + ' 銅接手【' + esc(peer.incoming_offer.item_name) + '】</button>');
      }
      var pacts = peer.open_pacts || [];
      for (var ci = 0; ci < pacts.length; ci++) {
        var pc = pacts[ci];
        if (pc.can_accept) {
          peerBtns.push('<button type="button" class="oi-topic-chip" data-peer-act="accept_pact" data-peer-param="' + esc(pc.contract_id) + '">接下託付（保付 ' + esc(pc.reward_copper) + ' 銅）</button>');
        } else if (pc.can_fulfill) {
          peerBtns.push('<button type="button" class="oi-topic-chip" data-peer-act="fulfill_pact" data-peer-param="' + esc(pc.contract_id) + '">當面交差領賞（' + esc(pc.reward_copper) + ' 銅）</button>');
        }
      }
      var goods = peer.tradable_goods || [];
      for (var gi = 0; gi < goods.length; gi++) {
        var gd = goods[gi];
        peerBtns.push('<button type="button" class="oi-topic-chip" data-peer-act="peer_buy_item" data-peer-param="' + esc(npc.id + '|' + gd.slug) + '">以 ' + esc(gd.ask_copper) + ' 銅購【' + esc(gd.name) + '】</button>');
      }
      peerBtns.push('<button type="button" class="oi-topic-chip" data-peer-act="propose_courier_pact" data-peer-param="' + esc(npc.id) + '">託人送家書予他（保付5銅）</button>');
      peerBtns.push('<button type="button" class="oi-topic-chip" data-peer-act="propose_escort_pact" data-peer-param="">徵求同行護衛（保付6銅）</button>');
      var sparLabel = peer.incoming_spar ? '拱手應戰切磋' : (npc.isPlayer ? '拱手邀約切磋' : '點到為止切磋');
      peerBtns.push('<button type="button" class="oi-topic-chip" data-peer-act="peer_spar" data-peer-param="' + esc(npc.id) + '">' + sparLabel + '</button>');

      var offerables = (p.backpack || []).filter(function (i) { return !i.is_voucher && i.count > 0; });
      var offerBarHtml = '';
      if (offerables.length > 0) {
        var optsHtml = offerables.map(function (it) {
          var fc = it.fair_copper || (it.is_vessel ? 14 : 5);
          return '<option value="' + esc(it.slug) + '" data-fair="' + esc(fc) + '">' + esc(it.name) + '（×' + esc(it.count) + '）</option>';
        }).join('');
        var defFair = offerables[0].fair_copper || (offerables[0].is_vessel ? 14 : 5);
        offerBarHtml =
          '<div style="display:flex;gap:6px;align-items:center;margin:6px 0;flex-wrap:wrap;">' +
          '<select id="oi-offer-slug" class="oi-talk-input" style="flex:1 1 150px;padding:6px 8px;">' + optsHtml + '</select>' +
          '<input type="number" id="oi-offer-price" class="oi-talk-input" min="1" max="500" value="' + esc(defFair) + '" style="width:76px;flex:0 0 auto;padding:6px 8px;" aria-label="報價銅幣">' +
          '<button type="button" class="oi-topic-chip" data-submit-offer="' + esc(npc.id) + '">亮物報價</button>' +
          '</div>';
      }

      var hasRubbing = (p.backpack || []).some(function (i) { return i.slug === 'stele_rubbing'; });
      var rubbingBtn = (!npc.isPlayer && hasRubbing)
        ? '<button type="button" class="oi-topic-chip" data-topic="show_rubbing">出示古碑拓片</button>'
        : '';
      var npcTopicRow = npc.isPlayer ? '' : (
        '<button type="button" class="oi-topic-chip" data-topic="work">打聽活計與價目</button>' +
        '<button type="button" class="oi-topic-chip" data-topic="rumor">探聽北林與渡口風聲</button>' +
        '<button type="button" class="oi-topic-chip" data-topic="learn_lang">請教幾句日常母語</button>' +
        rubbingBtn
      );

      var liveDeeds = (liveEntry && liveEntry.deeds) || [];
      var counterDeedsHtml = liveDeeds.length > 0
        ? ('<div class="oi-counter-deeds">' + liveDeeds.map(function (d) {
            return '<button type="button" class="oi-deed-btn" data-sheet-act="' + esc(d.id) + '" data-sheet-param="' + esc(d.param || '') + '">' +
              '<strong>' + esc(d.label) + '</strong>' +
              '<small>' + esc(d.sub || '') + '</small></button>';
          }).join('') + '</div>')
        : '';

      bodyEl.innerHTML =
        counterDeedsHtml +
        '<div class="oi-talk-log" id="oi-talk-log">' + bubbles + '</div>' +
        '<div class="oi-talk-topics">' +
        npcTopicRow +
        peerBtns.join('') +
        '</div>' +
        offerBarHtml +
        '<form class="oi-talk-form" id="oi-talk-form">' +
        '<input type="text" class="oi-talk-input" id="oi-talk-input" maxlength="80" autocomplete="off" placeholder="直接輸入你想對他說的話……">' +
        '<button type="submit" class="oi-btn oi-btn-primary" style="flex:0 0 auto;">開口</button>' +
        '</form>';

      var logEl = byId('oi-talk-log');
      if (logEl) logEl.scrollTop = logEl.scrollHeight;
      return;
    }

    if (appState.activeSheet === 'object') {
      var sceneObjs = (view.scene && view.scene.objects) || [];
      var curObj = null;
      for (var obi = 0; obi < sceneObjs.length; obi++) {
        if (sceneObjs[obi].object_id === appState.activeObjectId) {
          curObj = sceneObjs[obi];
          break;
        }
      }
      if (!curObj) {
        closeSheet();
        return;
      }
      titleEl.textContent = curObj.name;
      var objOpts = curObj.options || [];
      var optsListHtml = objOpts.map(function (op) {
        return '<button type="button" class="oi-deed-btn" data-sheet-act="' + esc(op.id) + '" data-sheet-param="' + esc(op.param || '') + '" data-close-on-ok="1" style="width:100%;">' +
          '<strong>' + esc(op.label) + '</strong>' +
          '<small>' + esc(op.sub || '') + '</small></button>';
      }).join('');
      bodyEl.innerHTML =
        '<div class="oi-kv-list">' +
        '<div class="oi-kv-card"><div style="line-height:1.65;color:var(--ink-main);">' + esc(curObj.desc || curObj.cue || '') + '</div></div>' +
        optsListHtml +
        '</div>';
      return;
    }

    if (appState.activeSheet === 'self') {
      titleEl.textContent = p.name + '・' + p.race;
      var w = p.weapon || {};
      var langsHtml = (p.languages || []).map(function (l) {
        return '<div class="oi-item-row"><span>' + esc(l.name) + '</span><span style="color:var(--ink-sub);">' + esc(l.tier) + '</span></div>';
      }).join('');
      bodyEl.innerHTML =
        '<div class="oi-kv-list">' +
        '<div class="oi-kv-card"><strong>外貌與體態</strong><div style="margin-top:4px;color:var(--ink-sub);">' + esc(p.appearance) + ' ｜ ' + esc(p.vibe) + '</div>' +
        (p.temper_cue ? '<div style="margin-top:4px;font-size:0.86rem;color:var(--ink-sub);">' + esc(p.temper_cue) + '</div>' : '') +
        (p.wound ? '<div style="margin-top:6px;color:var(--danger-ink);">傷勢：' + esc(p.wound) + '</div>' : '') +
        '</div>' +
        '<div class="oi-kv-card"><strong>隨身兵刃：' + esc(w.name) + '</strong>' +
        '<div style="margin-top:4px;font-size:0.88rem;color:var(--ink-sub);">' + esc(w.provenance) + '</div>' +
        '<div style="margin-top:6px;font-size:0.88rem;">器況：' + esc(w.wear) + ' ｜ 手感：' + esc(w.grip) +
        (w.rune ? ' ｜ 已鑲【' + esc(w.rune) + '】' : '') + '</div>' +
        (p.mastery ? '<div style="margin-top:4px;font-size:0.86rem;color:var(--accent-jade);">歷練成相：' + esc(p.mastery) + '</div>' : '') +
        '</div>' +
        '<div class="oi-kv-card"><strong>住宿與契約</strong><div style="margin-top:4px;font-size:0.88rem;color:var(--ink-sub);">' +
        '二樓宿房剩餘：' + esc(p.inn_nights_left || 0) + ' 宿 ｜ 逾時搜救契：' + (p.rescue_pact ? '已登記生效' : '未登記') +
        (p.owned_house_node ? ' ｜ 已置辦自有屋舍' : '') + '</div></div>' +
        '<div class="oi-kv-card"><strong>略通的各族言語</strong>' + langsHtml + '</div>' +
        '<button type="button" class="oi-btn" id="oi-btn-logout" style="width:100%;margin-top:4px;">暫別海港（登出帳號）</button>' +
        '</div>';
      return;
    }

    if (appState.activeSheet === 'bag') {
      titleEl.textContent = '隨身行囊與宿房木箱';
      var sc = view.scene || {};
      var canUseLocker = (sc.kind === 'inn_room' && (p.inn_nights_left || 0) > 0) || (p.owned_house_node && p.owned_house_node === sc.node_id);
      var bagItems = p.backpack || [];
      var bagHtml = bagItems.length
        ? bagItems.map(function (it) {
            var eatBtn = it.slug === 'rye_bread'
              ? '<button type="button" class="oi-icon-btn" data-sheet-act="eat_ration" data-sheet-param="" style="margin-right:4px;">取食</button>'
              : '';
            var dropCrateBtn = it.slug === 'dock_cargo_crate'
              ? '<button type="button" class="oi-icon-btn" data-sheet-act="cancel_activity" data-sheet-param="" style="margin-right:4px;">卸在腳邊</button>'
              : '';
            var equipBtn = it.is_vessel
              ? '<button type="button" class="oi-icon-btn" data-equip-slug="' + esc(it.slug) + '" style="margin-right:4px;">換握上手</button>'
              : '';
            var lockBtn = (canUseLocker && !it.is_voucher)
              ? '<button type="button" class="oi-icon-btn" data-store-slug="' + esc(it.slug) + '">鎖入木箱</button>'
              : '';
            var tag = it.is_voucher ? '（隨身憑據）' : (it.is_unclaimed ? '（拾遺未報備）' : (it.is_vessel ? '（落款器物）' : ''));
            return '<div class="oi-item-row"><span>' + esc(it.name) + ' × ' + esc(it.count) + ' <small style="color:var(--ink-muted);">' + esc(tag) + '</small></span><span>' + eatBtn + dropCrateBtn + equipBtn + lockBtn + '</span></div>';
          }).join('')
        : '<div style="color:var(--ink-muted);font-size:0.9rem;padding:6px 0;">背囊裡目前空空如也。</div>';

      var lockerItems = p.inn_locker || [];
      var lockerHtml = lockerItems.length
        ? lockerItems.map(function (it) {
            var takeBtn = canUseLocker
              ? '<button type="button" class="oi-icon-btn" data-take-slug="' + esc(it.slug) + '">取出隨身</button>'
              : '';
            return '<div class="oi-item-row"><span>' + esc(it.name) + ' × ' + esc(it.count) + '</span>' + takeBtn + '</div>';
          }).join('')
        : '<div style="color:var(--ink-muted);font-size:0.9rem;padding:6px 0;">床頭厚木箱目前沒有存放貨品。</div>';

      bodyEl.innerHTML =
        '<div class="oi-kv-list">' +
        '<div class="oi-kv-card"><strong>懷中錢袋：' + esc(p.copper || 0) + ' 枚銅幣</strong>' +
        '<div style="font-size:0.83rem;color:var(--ink-sub);margin-top:4px;">重要採集貨品可在進入深山前鎖入二樓宿房厚木箱保管。</div></div>' +
        '<div class="oi-kv-card"><strong>肩上背囊</strong>' + bagHtml + '</div>' +
        '<div class="oi-kv-card"><strong>宿房床頭厚木箱' + (canUseLocker ? '（可存取）' : '（回二樓宿房可開啟）') + '</strong>' + lockerHtml + '</div>' +
        '</div>';
      return;
    }

    if (appState.activeSheet === 'recap') {
      titleEl.textContent = '行旅生平與近事錄';
      var selfBioHtml = p.self_bio
        ? ('<div class="oi-kv-card" style="margin-bottom:12px;border-left:3px solid var(--accent-main);">' +
          '<strong>個人生平小傳</strong>' +
          String(p.self_bio).split('\n\n').map(function (para) {
            return '<p style="margin-top:8px;line-height:1.85;text-indent:2em;color:var(--ink-main);">' + esc(para) + '</p>';
          }).join('') + '</div>')
        : '';
      var lines = ((view.scene && view.scene.breaths) || []).slice().reverse();
      bodyEl.innerHTML = selfBioHtml + lines.map(function (l) {
        return '<div class="oi-kv-card" style="margin-bottom:8px;line-height:1.6;">' + esc(l) + '</div>';
      }).join('');
      return;
    }

    if (appState.activeSheet === 'scroll' && appState.scrollData) {
      var sd = appState.scrollData;
      var art = sd.article || {};
      titleEl.textContent = sd.mode === 'annals' ? '海峽與蒼淵兵燹抄本' : '廣場鎮志與開埠石碑';
      var tabsHtml = (sd.catalog || []).map(function (it) {
        var activeStyle = it.id === sd.selected_id
          ? 'background:var(--accent-main);color:#fff;border-color:var(--accent-main);'
          : '';
        return '<button type="button" class="oi-icon-btn" data-scroll-mode="' + esc(sd.mode) + '" data-scroll-id="' + esc(it.id) + '" style="margin:0 5px 6px 0;' + activeStyle + '">' + esc(it.title_zh) + '</button>';
      }).join('');
      var parasHtml = (art.paragraphs || []).map(function (para) {
        return '<p style="margin-top:12px;line-height:1.95;text-indent:2em;text-align:justify;">' + esc(para) + '</p>';
      }).join('');
      bodyEl.innerHTML =
        '<div style="margin-bottom:10px;display:flex;flex-wrap:wrap;">' + tabsHtml + '</div>' +
        '<div class="oi-kv-card" style="padding:16px 18px;">' +
        '<div style="font-size:1.15rem;font-weight:700;margin-bottom:4px;">' + esc(art.title_zh || '') + '</div>' +
        '<div style="font-size:0.84rem;color:var(--ink-muted);margin-bottom:6px;">' + esc(art.era_zh || '') + ' ｜ ' + esc(art.carrier_zh || '') + '</div>' +
        parasHtml +
        '</div>';
    }
  }

  function openWorldScrollSheet(mode, targetId) {
    var qs = '?mode=' + encodeURIComponent(mode) + (targetId ? ('&id=' + encodeURIComponent(targetId)) : '');
    apiJson('/api/island/scroll' + qs, 'GET').then(function (res) {
      if (!res || !res.ok) return;
      appState.scrollData = res;
      openInfoSheet('scroll');
    });
  }

  function appendTalkLogIfOpen(npcId) {
    if (appState.activeSheet !== 'talk' || !appState.talkNpc || appState.talkNpc.id !== npcId) {
      return false;
    }
    var logEl = byId('oi-talk-log');
    if (!logEl) return false;
    var hist = appState.talkHistory[npcId] || [];
    var bubbles = hist.map(function (item) {
      var cls = 'oi-talk-bubble ' + (item.kind || 'npc');
      return '<div class="' + cls + '">' + esc(item.text) + '</div>';
    }).join('');
    setHtmlIfChanged(logEl, bubbles);
    logEl.scrollTop = logEl.scrollHeight;
    setTextIfChanged(byId('oi-sheet-title'), appState.talkNpc.name + '（' + appState.talkNpc.role + '）');
    return true;
  }

  function sendTalkRequest(topic, message) {
    if (!appState.talkNpc) return;
    var npcId = appState.talkNpc.id;
    if (!appState.talkHistory[npcId]) {
      appState.talkHistory[npcId] = [];
    }
    if (message) {
      appState.talkHistory[npcId].push({ kind: 'me', text: '你：「' + message + '」' });
      if (!appendTalkLogIfOpen(npcId)) {
        renderActiveSheet();
      }
    }
    apiJson('/api/island/talk', 'POST', {
      npc_id: npcId,
      topic: topic || 'chat',
      message: message || ''
    }).then(function (res) {
      if (!res || !res.ok) {
        showToast((res && res.msg) || '對方正忙著手頭活計');
        return;
      }
      if (res.npc_name && appState.talkNpc && appState.talkNpc.id === npcId) {
        appState.talkNpc.name = res.npc_name;
      }
      if (res.side_overheard) {
        appState.talkHistory[npcId].push({ kind: 'overheard', text: res.side_overheard });
      }
      var rawReply = String(res.reply || '').trim();
      if (res.is_peer_player) {
        if (rawReply) {
          appState.talkHistory[npcId].push({ kind: 'overheard', text: rawReply });
        }
      } else {
        var spName = res.npc_name || (appState.talkNpc && appState.talkNpc.name) || '街坊';
        var bubbleText = (rawReply.charAt(0) === '「' && rawReply.indexOf('」') > 0 && rawReply.indexOf(spName) > rawReply.indexOf('」'))
          ? rawReply
          : ('「' + rawReply.replace(/^[「『]+|[」』]+$/g, '') + '」' + spName + '應道。');
        appState.talkHistory[npcId].push({
          kind: 'npc',
          text: bubbleText
        });
      }
      if (res.lang_note) {
        showToast(res.lang_note);
      }
      if (res.view) {
        applyServerView(res.view, true);
      }
      if (appState.activeSheet === 'talk' && appState.talkNpc && appState.talkNpc.id === npcId) {
        if (!appendTalkLogIfOpen(npcId)) {
          renderActiveSheet();
        }
      }
    });
  }

  /* ═══ 零外部圖檔・全島單一無縫水彩古卷 Canvas2D 輿圖控制器 ═══ */
  function ensureCanvasElement(wrapEl, canvasId) {
    var existing = byId(canvasId);
    if (existing && existing.parentElement === wrapEl) return existing;
    wrapEl.innerHTML = '';
    var c = document.createElement('canvas');
    c.id = canvasId;
    c.className = 'oi-map-canvas-el';
    c.setAttribute('role', 'img');
    c.setAttribute('aria-label', '銀鷗外島彩色水彩輿圖');
    wrapEl.appendChild(c);
    return c;
  }

  function isCurrentCellInTown(m) {
    if (!m) return false;
    var towns = m.town_micro_maps || {};
    return Object.keys(towns).some(function (k) {
      return towns[k].cell_id === m.current_cell_id;
    });
  }

  function renderDesktopMiniMap() {
    if (appState.mapOpen) return;
    var wrap = byId('oi-mini-map-frame');
    var v = appState.view;
    if (!wrap || !v || !v.scene || !window.StaticNodeMap) return;
    var aside = byId('oi-map-aside');
    if (aside) {
      aside.classList.toggle('is-collapsed', !!appState.mobileMapCollapsed);
    }
    var toggleBtn = byId('oi-btn-toggle-map');
    if (toggleBtn) {
      setTextIfChanged(toggleBtn, appState.mobileMapCollapsed ? '展開地圖' : '收合');
    }
    Array.prototype.forEach.call(document.querySelectorAll('[data-aside-mode]'), function (btn) {
      btn.classList.toggle('active', btn.getAttribute('data-aside-mode') === appState.asideMode);
    });
    if (appState.mobileMapCollapsed && window.innerWidth < 960) return;

    var opts = { isNight: document.body.classList.contains('sw-night') };
    if (appState.asideMode === 'island') {
      window.StaticNodeMap.renderIslandOverview(wrap, v, appState.mapData, opts);
      appState.mappedExitIds = {};
    } else {
      var res = window.StaticNodeMap.renderLocal(wrap, v, opts);
      appState.mappedExitIds = (res && res.mappedExitIds) || {};
      var localTab = document.querySelector('[data-aside-mode="local"]');
      if (localTab && res && res.layerBadge) {
        setTextIfChanged(localTab, res.layerBadge);
      }
    }
  }

  function renderFullscreenMap() {
    var wrap = byId('oi-map-canvas');
    var v = appState.view;
    if (!wrap || !v || !v.scene || !window.StaticNodeMap) return;
    Array.prototype.forEach.call(document.querySelectorAll('[data-map-scale]'), function (btn) {
      btn.classList.toggle('active', btn.getAttribute('data-map-scale') === appState.mapScaleMode);
    });
    var opts = { isNight: document.body.classList.contains('sw-night') };
    if (appState.mapScaleMode === 'island') {
      setTextIfChanged(byId('oi-map-banner'), '銀鷗外島・全島總覽');
      window.StaticNodeMap.renderIslandOverview(wrap, v, appState.mapData, opts);
    } else {
      var res = window.StaticNodeMap.renderLocal(wrap, v, opts);
      var localBtn = document.querySelector('[data-map-scale="local"]');
      if (localBtn && res && res.layerBadge) {
        setTextIfChanged(localBtn, res.layerBadge);
      }
      setTextIfChanged(byId('oi-map-banner'), (v.scene && v.scene.name) || '手繪輿圖');
    }
  }

  /* ═══ 全域事件委派與手機虛擬鍵盤貼齊 ═══ */
  function bindMainEvents() {
    setInterval(tickLocalIslandClock, 500);
    setInterval(pollIslandPulse, 3000);
    document.addEventListener('visibilitychange', function () {
      if (document.visibilityState === 'visible') {
        tickLocalIslandClock();
        pollIslandPulse();
      }
    });
    window.addEventListener('pagehide', function () {
      if (appState.authed && navigator.sendBeacon) {
        navigator.sendBeacon('/api/island/leave', '{}');
      }
    });

    // 場景內人物與行動點擊
    byId('oi-shell').addEventListener('click', function (ev) {
      var crowdBtn = ev.target.closest('[data-toggle-crowd]');
      if (crowdBtn) {
        appState.crowdExpanded = !appState.crowdExpanded;
        if (appState.view) renderSceneStage(appState.view);
        return;
      }

      var personBtn = ev.target.closest('[data-npc-id]');
      if (personBtn) {
        openTalkSheet(
          personBtn.getAttribute('data-npc-id'),
          personBtn.getAttribute('data-npc-name'),
          personBtn.getAttribute('data-npc-role'),
          personBtn.getAttribute('data-is-player') === '1'
        );
        return;
      }

      var objBtn = ev.target.closest('[data-object-id]');
      if (objBtn) {
        var oid = objBtn.getAttribute('data-object-id');
        var sObjs = (appState.view && appState.view.scene && appState.view.scene.objects) || [];
        var foundObj = null;
        for (var oi2 = 0; oi2 < sObjs.length; oi2++) {
          if (sObjs[oi2].object_id === oid) { foundObj = sObjs[oi2]; break; }
        }
        if (!foundObj) return;
        if (foundObj.direct_action && foundObj.direct_action.id) {
          var daId = foundObj.direct_action.id;
          if (daId === 'open_scroll_stele') {
            openWorldScrollSheet('stele', '');
            return;
          }
          if (daId === 'open_scroll_annals') {
            openWorldScrollSheet('annals', '');
            return;
          }
          apiJson('/api/island/act', 'POST', {
            action: daId,
            param: foundObj.direct_action.param || ''
          }).then(function (res) {
            if (res && res.view) applyServerView(res.view);
            if (res && res.msg) showToast(res.msg);
            refreshMapData(false);
          });
          return;
        }
        openObjectSheet(oid);
        return;
      }

      var actBtn = ev.target.closest('[data-act]');
      if (actBtn) {
        var act = actBtn.getAttribute('data-act');
        if (act === 'open_scroll_stele') {
          openWorldScrollSheet('stele', '');
          return;
        }
        if (act === 'open_scroll_annals') {
          openWorldScrollSheet('annals', '');
          return;
        }
        var actParam = actBtn.getAttribute('data-param') || '';
        apiJson('/api/island/act', 'POST', { action: act, param: actParam }).then(function (res) {
          if (res && res.view) applyServerView(res.view);
          if (res && !res.ok && res.msg) showToast(res.msg);
          refreshMapData(false);
        });
        return;
      }

      var asideTab = ev.target.closest('[data-aside-mode]');
      if (asideTab) {
        appState.asideMode = asideTab.getAttribute('data-aside-mode');
        if (appState.mobileMapCollapsed) appState.mobileMapCollapsed = false;
        renderDesktopMiniMap();
        if (appState.view) {
          renderPresenceDoors(appState.view);
          renderExitsStrip(appState.view);
        }
        return;
      }

      var nodeHopBtn = ev.target.closest('[data-node-hop]');
      if (nodeHopBtn) {
        var hopNid = nodeHopBtn.getAttribute('data-node-hop');
        moveNodeTo(hopNid);
        return;
      }

      var mapNode = ev.target.closest('[data-map-node]');
      if (mapNode) {
        var targetNid = mapNode.getAttribute('data-map-node');
        moveNodeTo(targetNid);
        return;
      }

      var moveBtn = ev.target.closest('[data-move-to]');
      if (moveBtn) {
        var dst = moveBtn.getAttribute('data-move-to');
        moveNodeTo(dst);
        return;
      }
    });

    var toggleMapBtn = byId('oi-btn-toggle-map');
    if (toggleMapBtn) {
      toggleMapBtn.addEventListener('click', function () {
        appState.mobileMapCollapsed = !appState.mobileMapCollapsed;
        renderDesktopMiniMap();
        if (appState.view) {
          renderPresenceDoors(appState.view);
          renderExitsStrip(appState.view);
        }
      });
    }

    // 底欄四鍵與展開全幅靜態輿圖
    byId('oi-nav-self').addEventListener('click', function () { openInfoSheet('self'); });
    byId('oi-nav-bag').addEventListener('click', function () { openInfoSheet('bag'); });
    byId('oi-nav-recap').addEventListener('click', function () { openInfoSheet('recap'); });
    byId('oi-nav-explore').addEventListener('click', function () {
      apiJson('/api/island/act', 'POST', { action: 'explore', param: '' }).then(function (res) {
        if (res && res.view) applyServerView(res.view);
        if (res && (!res.ok || !res.view) && res.msg) showToast(res.msg);
      });
    });

    function openMapModal(preferredMode) {
      appState.mapOpen = true;
      appState.mapScaleMode = preferredMode || appState.asideMode || 'local';
      byId('oi-map-modal').hidden = false;
      renderFullscreenMap();
    }
    var navMapBtn = byId('oi-nav-map');
    if (navMapBtn) {
      navMapBtn.addEventListener('click', function () { openMapModal('island'); });
    }
    byId('oi-btn-expand-map').addEventListener('click', function () { openMapModal(appState.asideMode || 'local'); });
    byId('oi-map-close').addEventListener('click', function () {
      appState.mapOpen = false;
      byId('oi-map-modal').hidden = true;
      renderDesktopMiniMap();
    });

    // 全螢幕靜態手繪圖：僅相連節點按鈕可點，點地圖空白處完全不移動
    byId('oi-map-modal').addEventListener('click', function (ev) {
      var scaleBtn = ev.target.closest('[data-map-scale]');
      if (scaleBtn) {
        appState.mapScaleMode = scaleBtn.getAttribute('data-map-scale') || 'local';
        renderFullscreenMap();
        return;
      }
      var hopBtn = ev.target.closest('[data-node-hop]');
      if (hopBtn) {
        moveNodeTo(hopBtn.getAttribute('data-node-hop'));
        return;
      }
    });

    window.addEventListener('resize', function () {
      renderDesktopMiniMap();
      if (appState.mapOpen) renderFullscreenMap();
    });

    // 抽屜內部互動（關閉、對話送出、木箱存取、登出）
    byId('oi-sheet-close').addEventListener('click', closeSheet);
    byId('oi-sheet-backdrop').addEventListener('click', function (ev) {
      if (ev.target === byId('oi-sheet-backdrop')) {
        closeSheet();
        return;
      }
      var topicBtn = ev.target.closest('[data-topic]');
      if (topicBtn) {
        sendTalkRequest(topicBtn.getAttribute('data-topic'), '');
        return;
      }
      var scrollTab = ev.target.closest('[data-scroll-id]');
      if (scrollTab) {
        openWorldScrollSheet(
          scrollTab.getAttribute('data-scroll-mode') || 'stele',
          scrollTab.getAttribute('data-scroll-id')
        );
        return;
      }
      var offerBtn = ev.target.closest('[data-submit-offer]');
      if (offerBtn) {
        var targetPeerId = offerBtn.getAttribute('data-submit-offer');
        var selEl = byId('oi-offer-slug');
        var prEl = byId('oi-offer-price');
        var chosenSlug = selEl ? selEl.value : '';
        var chosenPrice = Math.max(1, Math.min(500, parseInt(prEl && prEl.value, 10) || 10));
        if (!chosenSlug) return;
        apiJson('/api/island/act', 'POST', {
          action: 'peer_offer_item',
          param: targetPeerId + '|' + chosenSlug + '|' + chosenPrice
        }).then(function (res) {
          if (res && res.view) applyServerView(res.view);
          if (res && res.msg) {
            showToast(res.msg);
            if (appState.talkNpc) {
              var nid2 = appState.talkNpc.id;
              if (!appState.talkHistory[nid2]) appState.talkHistory[nid2] = [];
              appState.talkHistory[nid2].push({ kind: 'overheard', text: res.msg });
            }
          }
          if (appState.activeSheet === 'talk') renderActiveSheet();
        });
        return;
      }
      var sheetActBtn = ev.target.closest('[data-sheet-act]');
      if (sheetActBtn) {
        var sAct = sheetActBtn.getAttribute('data-sheet-act');
        var sParam = sheetActBtn.getAttribute('data-sheet-param') || '';
        var closeOnOk = sheetActBtn.getAttribute('data-close-on-ok') === '1';
        apiJson('/api/island/act', 'POST', {
          action: sAct,
          param: sParam
        }).then(function (res) {
          if (res && res.view) applyServerView(res.view);
          if (res && res.msg) {
            showToast(res.msg);
            if (appState.activeSheet === 'talk' && appState.talkNpc) {
              var tnId = appState.talkNpc.id;
              if (!appState.talkHistory[tnId]) appState.talkHistory[tnId] = [];
              appState.talkHistory[tnId].push({ kind: 'overheard', text: res.msg });
            }
          }
          refreshMapData(false);
          if (res && res.ok && closeOnOk) {
            closeSheet();
          } else if (appState.activeSheet) {
            renderActiveSheet();
          }
        });
        return;
      }
      var peerActBtn = ev.target.closest('[data-peer-act]');
      if (peerActBtn) {
        var pactAct = peerActBtn.getAttribute('data-peer-act');
        var pactParam = peerActBtn.getAttribute('data-peer-param') || '';
        apiJson('/api/island/act', 'POST', {
          action: pactAct,
          param: pactParam
        }).then(function (res) {
          if (res && res.view) applyServerView(res.view);
          if (res && res.msg) {
            showToast(res.msg);
            if (appState.talkNpc) {
              var nid = appState.talkNpc.id;
              if (!appState.talkHistory[nid]) appState.talkHistory[nid] = [];
              appState.talkHistory[nid].push({ kind: 'overheard', text: res.msg });
            }
          }
          if (appState.activeSheet === 'talk') renderActiveSheet();
        });
        return;
      }
      var equipBtn = ev.target.closest('[data-equip-slug]');
      if (equipBtn) {
        apiJson('/api/island/act', 'POST', {
          action: 'equip_vessel',
          param: equipBtn.getAttribute('data-equip-slug')
        }).then(function (res) {
          if (res && res.view) applyServerView(res.view);
          if (res && res.msg) showToast(res.msg);
        });
        return;
      }
      var storeBtn = ev.target.closest('[data-store-slug]');
      if (storeBtn) {
        apiJson('/api/island/act', 'POST', {
          action: 'store_item',
          param: storeBtn.getAttribute('data-store-slug')
        }).then(function (res) {
          if (res && res.view) applyServerView(res.view);
          if (res && res.msg) showToast(res.msg);
        });
        return;
      }
      var takeBtn = ev.target.closest('[data-take-slug]');
      if (takeBtn) {
        apiJson('/api/island/act', 'POST', {
          action: 'take_item',
          param: takeBtn.getAttribute('data-take-slug')
        }).then(function (res) {
          if (res && res.view) applyServerView(res.view);
          if (res && res.msg) showToast(res.msg);
        });
        return;
      }
      if (ev.target.closest('#oi-btn-logout')) {
        apiJson('/api/rim/logout', 'POST', {}).then(function () {
          closeSheet();
          appState.authed = false;
          showAuthScreen();
        });
      }
    });

    byId('oi-sheet-backdrop').addEventListener('change', function (ev) {
      if (ev.target && ev.target.id === 'oi-offer-slug') {
        var opt = ev.target.options[ev.target.selectedIndex];
        var pr = byId('oi-offer-price');
        if (opt && pr) {
          pr.value = opt.getAttribute('data-fair') || '10';
        }
      }
    });

    byId('oi-sheet-backdrop').addEventListener('submit', function (ev) {
      if (ev.target && ev.target.id === 'oi-talk-form') {
        ev.preventDefault();
        var inp = byId('oi-talk-input');
        var val = (inp && inp.value || '').trim();
        if (!val) return;
        inp.value = '';
        sendTalkRequest('chat', val);
      }
    });

    // 手機虛擬鍵盤彈出時動態貼齊抽屜底部
    if (window.visualViewport) {
      window.visualViewport.addEventListener('resize', function () {
        var sheet = byId('oi-bottom-sheet');
        if (!sheet) return;
        var offset = Math.max(0, window.innerHeight - window.visualViewport.height);
        sheet.style.transform = offset > 0 ? ('translateY(-' + offset + 'px)') : '';
      });
    }
  }

  document.addEventListener('DOMContentLoaded', function () {
    bindAuthEvents();
    bindMainEvents();
    checkSessionOnBoot();
  });
})();
