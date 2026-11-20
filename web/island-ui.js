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
    mapScaleMode: 'town', // 'island' | 'town' | 'dungeon'
    asideMode: 'town', // 'town' | 'island'
    lastCellId: null,
    lastNodeId: null,
    crowdExpanded: false,
    cam: { cx: 716, cy: 536, zoom: 1.0, dragging: false, lastX: 0, lastY: 0 },
    townCam: { cx: 318, cy: 425, zoom: 1.0 }
  };

  function byId(id) {
    return document.getElementById(id);
  }

  function esc(str) {
    return String(str == null ? '' : str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
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

  function syncTheme() {
    var isNight = false;
    if (appState.themeOverride === 'night') {
      isNight = true;
    } else if (appState.themeOverride === 'day') {
      isNight = false;
    } else if (appState.view && appState.view.clock) {
      isNight = !!appState.view.clock.is_night;
    }
    document.body.classList.toggle('sw-night', isNight);
    var btn = byId('oi-btn-theme');
    if (btn) {
      btn.textContent = isNight ? '燭夜' : '晴晝';
    }
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

  function refreshMapData(centerCamera) {
    apiJson('/api/island/map', 'GET').then(function (m) {
      if (!m || !m.ok) return;
      var prevCellId = appState.lastCellId;
      appState.lastCellId = m.current_cell_id;
      appState.mapData = m;
      if (centerCamera && m.visited_cells && m.visited_cells.length) {
        var curCell = m.visited_cells.find(function (c) {
          return c.cell_id === m.current_cell_id;
        }) || m.visited_cells[0];
        if (curCell && curCell.center) {
          appState.cam.cx = curCell.center[0];
          appState.cam.cy = curCell.center[1];
          appState.cam.zoom = 1.15;
        }
      }
      if (m.in_dungeon) {
        appState.mapScaleMode = 'dungeon';
      } else {
        var towns = m.town_micro_maps || {};
        var inVisitedTown = Object.keys(towns).some(function (k) {
          return towns[k].cell_id === m.current_cell_id;
        });
        if (!inVisitedTown && appState.mapScaleMode === 'town') {
          appState.mapScaleMode = 'island';
        }
        if (!inVisitedTown && appState.asideMode === 'town') {
          appState.asideMode = 'island';
        }
        if (inVisitedTown && prevCellId !== m.current_cell_id) {
          appState.mapScaleMode = 'town';
          appState.asideMode = 'town';
        }
        if (centerCamera && inVisitedTown) {
          appState.townCam.cx = 318;
          appState.townCam.cy = 425;
          appState.townCam.zoom = 1.0;
        }
      }
      renderDesktopMiniMap();
      if (appState.mapOpen) {
        renderFullscreenMap();
      }
    });
  }

  function applyServerView(view) {
    var prevNid = appState.lastNodeId;
    var nextNid = view && view.scene && view.scene.node_id;
    if (nextNid && nextNid !== prevNid) {
      appState.lastNodeId = nextNid;
      appState.crowdExpanded = false;
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
    renderExitsStrip(view);
    if (appState.activeSheet && appState.activeSheet !== 'talk') {
      renderActiveSheet();
    }
  }

  /* ═══ 三區主畫面渲染 ═══ */
  function renderTopbar(view) {
    var clk = view.clock || {};
    byId('oi-clock-text').textContent = (clk.calendar || '') + ' ｜ ' + (clk.bell || '');
  }

  function renderSceneStage(view) {
    var sc = view.scene || {};
    var p = view.player || {};
    byId('oi-scene-title').textContent = sc.name || '';
    byId('oi-copper-pill').textContent = (p.copper || 0) + ' 枚銅幣';

    var woundEl = byId('oi-wound-banner');
    if (p.wound) {
      woundEl.hidden = false;
      woundEl.textContent = '傷處提醒：' + p.wound + (p.severe_warning ? '（未敷藥包紮前切莫硬撼重甲異獸）' : '（可前往苦艾草藥堂敷藥消腫）');
    } else {
      woundEl.hidden = true;
    }

    byId('oi-scene-desc').textContent = sc.desc || '';

    // 交鋒對峙或馬車乘行狀態卡
    var skWrap = byId('oi-skirmish-wrap');
    if (view.skirmish) {
      var sk = view.skirmish;
      var movesHtml = (sk.moves || []).map(function (mv) {
        return '<button type="button" class="oi-deed-btn" data-act="' + esc(mv.id) + '">' +
          '<strong>' + esc(mv.label) + '</strong>' +
          '<small>' + esc(mv.hint || '') + '</small></button>';
      }).join('');
      skWrap.hidden = false;
      skWrap.innerHTML =
        '<div class="oi-skirmish-card">' +
        '<h3 class="oi-skirmish-title">短兵對峙：' + esc(sk.foe_name) + '（第 ' + esc(sk.round) + ' 合）</h3>' +
        '<div style="font-size:0.9rem;color:var(--ink-sub);">掌中兵刃：' + esc(p.weapon && p.weapon.name) +
        (sk.staggered ? ' ｜ 對方重心已晃，正可追擊！' : ' ｜ 留意對方前撲手腕與架勢') + '</div>' +
        '<div class="oi-skirmish-grid">' + movesHtml + '</div>' +
        '</div>';
    } else if (view.transit) {
      var tr = view.transit;
      skWrap.hidden = false;
      skWrap.innerHTML =
        '<div class="oi-skirmish-card">' +
        '<h3 class="oi-skirmish-title">' + esc(tr.line_name) + '</h3>' +
        '<div style="font-size:0.9rem;color:var(--ink-sub);">車廂沿大路平穩前行（進度 ' + esc(tr.step_idx) + ' / ' + esc(tr.total_steps) + '）</div>' +
        '<div class="oi-skirmish-grid">' +
        '<button type="button" class="oi-deed-btn" data-act="carriage_next"><strong>繼續乘車前行</strong><small>順大路推進</small></button>' +
        '<button type="button" class="oi-deed-btn" data-act="carriage_alight"><strong>拍板中途下車</strong><small>改為步行探索</small></button>' +
        '</div></div>';
    } else {
      skWrap.hidden = true;
      skWrap.innerHTML = '';
    }

    // 在場街坊居民（釘選關鍵人物＋人群摺疊展開，零截斷消失）
    var npcs = sc.npcs || [];
    var overflow = sc.crowd_overflow || [];
    var npcSec = byId('oi-npcs-section');
    var npcList = byId('oi-npcs-list');
    function renderPersonCard(n) {
      return '<button type="button" class="oi-person-btn" data-npc-id="' + esc(n.id) + '" data-npc-name="' + esc(n.name) + '" data-npc-role="' + esc(n.race + n.role) + '">' +
        '<span class="oi-person-top">' + esc(n.name) + '</span>' +
        '<span class="oi-person-sub">' + esc(n.race) + '・' + esc(n.role) + '（' + esc(n.cue) + '）</span>' +
        '</button>';
    }
    if ((npcs.length > 0 || overflow.length > 0) && !view.skirmish && !view.transit) {
      npcSec.hidden = false;
      var htmlParts = npcs.map(renderPersonCard);
      if (overflow.length > 0) {
        htmlParts.push(
          '<button type="button" class="oi-person-btn" data-toggle-crowd="1" style="border-style:dashed;">' +
          '<span class="oi-person-top">' +
          (appState.crowdExpanded
            ? ('收起旁側街坊與過客（' + overflow.length + ' 人）▴')
            : ('旁有 ' + overflow.length + ' 名街坊與過客 ▾')) +
          '</span>' +
          '<span class="oi-person-sub">' +
          (appState.crowdExpanded ? '點擊收合人群列表' : '點擊展開查看並上前搭話') +
          '</span></button>'
        );
        if (appState.crowdExpanded) {
          for (var oi = 0; oi < overflow.length; oi++) {
            htmlParts.push(renderPersonCard(overflow[oi]));
          }
        }
      }
      npcList.innerHTML = htmlParts.join('');
    } else {
      npcSec.hidden = true;
      npcList.innerHTML = '';
    }

    // 場景可辦事項
    var acts = sc.actions || [];
    var actSec = byId('oi-actions-section');
    var actList = byId('oi-actions-list');
    if (acts.length > 0 && !view.skirmish && !view.transit) {
      actSec.hidden = false;
      actList.innerHTML = acts.map(function (a) {
        return '<button type="button" class="oi-deed-btn" data-act="' + esc(a.id) + '">' +
          '<strong>' + esc(a.label) + '</strong>' +
          '<small>' + esc(a.sub || '') + '</small></button>';
      }).join('');
    } else {
      actSec.hidden = true;
      actList.innerHTML = '';
    }

    // 當下呼吸紀錄（取最近 4 筆）
    var breaths = sc.breaths || [];
    var recent = breaths.slice(-4);
    byId('oi-breaths-box').innerHTML = recent.map(function (line, idx) {
      var cls = idx === recent.length - 1 ? 'oi-breath-item latest' : 'oi-breath-item';
      return '<p class="' + cls + '">' + esc(line) + '</p>';
    }).join('');
  }

  function renderExitsStrip(view) {
    var strip = byId('oi-exits-strip');
    if (view.skirmish) {
      strip.innerHTML = '<span style="font-size:0.86rem;color:var(--accent-main);padding:6px 4px;">短兵相接中，請於上方選擇招式或虛晃退回</span>';
      return;
    }
    if (view.transit) {
      strip.innerHTML = '<span style="font-size:0.86rem;color:var(--ink-sub);padding:6px 4px;">正在雙馬篷車廂內，抵達或中途下車後即可自由邁步</span>';
      return;
    }
    var exits = (view.scene && view.scene.exits) || [];
    strip.innerHTML = exits.map(function (ex) {
      var shortDest = (ex.to_name || '').split('・').pop() || ex.to_name;
      var cleanDir = String(ex.dir || '').replace(/（[^）]*）/g, '').trim();
      var label = cleanDir && cleanDir !== shortDest ? (cleanDir + ' · ' + shortDest) : shortDest;
      var cls = ex.kind === 'secret' ? 'oi-exit-pill secret' : 'oi-exit-pill';
      return '<button type="button" class="' + cls + '" data-move-to="' + esc(ex.to) + '" title="' + esc(ex.ambient_cue || ex.to_name) + '">' +
        esc(label) +
        '</button>';
    }).join('');
  }

  /* ═══ 抽屜面板（當面交談／自身／行囊木箱／近事） ═══ */
  function openTalkSheet(npcId, npcName, npcRole) {
    appState.activeSheet = 'talk';
    appState.talkNpc = { id: npcId, name: npcName, role: npcRole };
    byId('oi-sheet-backdrop').hidden = false;
    renderActiveSheet();
    if (!appState.talkHistory[npcId] || !appState.talkHistory[npcId].length) {
      sendTalkRequest('greet', '');
    }
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
        bubbles = '<div class="oi-talk-bubble">正在上前搭話……</div>';
      }
      var hasRubbing = (p.backpack || []).some(function (i) { return i.slug === 'stele_rubbing'; });
      var rubbingBtn = hasRubbing
        ? '<button type="button" class="oi-topic-chip" data-topic="show_rubbing">出示古碑拓片</button>'
        : '';

      bodyEl.innerHTML =
        '<div class="oi-talk-log" id="oi-talk-log">' + bubbles + '</div>' +
        '<div class="oi-talk-topics">' +
        '<button type="button" class="oi-topic-chip" data-topic="work">打聽活計與價目</button>' +
        '<button type="button" class="oi-topic-chip" data-topic="rumor">探聽北林與渡口風聲</button>' +
        '<button type="button" class="oi-topic-chip" data-topic="learn_lang">請教幾句日常母語</button>' +
        rubbingBtn +
        '</div>' +
        '<form class="oi-talk-form" id="oi-talk-form">' +
        '<input type="text" class="oi-talk-input" id="oi-talk-input" maxlength="80" autocomplete="off" placeholder="直接輸入你想對他說的話……">' +
        '<button type="submit" class="oi-btn oi-btn-primary" style="flex:0 0 auto;">開口</button>' +
        '</form>';

      var logEl = byId('oi-talk-log');
      if (logEl) logEl.scrollTop = logEl.scrollHeight;
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
        (w.rune ? ' ｜ 已鑲【' + esc(w.rune) + '】' : '') + '</div></div>' +
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
            var lockBtn = (canUseLocker && !it.is_voucher)
              ? '<button type="button" class="oi-icon-btn" data-store-slug="' + esc(it.slug) + '">鎖入木箱</button>'
              : '';
            var tag = it.is_voucher ? '（隨身憑據）' : (it.is_unclaimed ? '（拾遺未報備）' : '');
            return '<div class="oi-item-row"><span>' + esc(it.name) + ' × ' + esc(it.count) + ' <small style="color:var(--ink-muted);">' + esc(tag) + '</small></span>' + lockBtn + '</div>';
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
      titleEl.textContent = '行旅近事錄';
      var notes = p.memorable_notes || [];
      var notesHtml = notes.length
        ? ('<div class="oi-kv-card" style="margin-bottom:10px;"><strong>睡眠沉澱之深刻見聞</strong>' +
          notes.map(function (n) {
            return '<div style="margin-top:5px;font-size:0.9rem;color:var(--ink-sub);">・' + esc(n) + '</div>';
          }).join('') + '</div>')
        : '';
      var lines = ((view.scene && view.scene.breaths) || []).slice().reverse();
      bodyEl.innerHTML = notesHtml + lines.map(function (l) {
        return '<div class="oi-kv-card" style="margin-bottom:8px;line-height:1.6;">' + esc(l) + '</div>';
      }).join('');
    }
  }

  function sendTalkRequest(topic, message) {
    if (!appState.talkNpc) return;
    var npcId = appState.talkNpc.id;
    if (!appState.talkHistory[npcId]) {
      appState.talkHistory[npcId] = [];
    }
    if (message) {
      appState.talkHistory[npcId].push({ kind: 'me', text: '你：「' + message + '」' });
      renderActiveSheet();
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
      appState.talkHistory[npcId].push({
        kind: 'npc',
        text: res.npc_name + '：' + res.reply
      });
      if (res.lang_note) {
        showToast(res.lang_note);
      }
      if (res.view) {
        applyServerView(res.view);
      }
      if (appState.activeSheet === 'talk') {
        renderActiveSheet();
      }
    });
  }

  /* ═══ 三尺度手繪羊皮紙彩繪輿圖引擎（外島山海／海港古城街廓／地底石室草圖） ═══ */
  function detHash(str) {
    var h = 2166136261;
    var s = String(str || '');
    for (var i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h = Math.imul(h, 16777619);
    }
    return h >>> 0;
  }

  function cellFillColor(cell, isNight) {
    if (cell.kind === 'settlement') {
      return isNight ? '#5f432c' : '#dfc39d';
    }
    if (cell.flux >= 30) {
      return isNight ? '#2c4d54' : '#a8cbbd';
    }
    if (cell.height >= 24) {
      return isNight ? '#4d4438' : '#c6b699';
    }
    if (cell.is_coast) {
      return isNight ? '#3d4b3c' : '#d2d7ab';
    }
    return isNight ? '#2b4534' : '#9bbd8e';
  }

  function polyPointsAttr(poly) {
    return (poly || []).map(function (pt) {
      return pt[0] + ',' + pt[1];
    }).join(' ');
  }

  function buildIslandSvgHtml(m, viewBoxStr, interactiveLabels, idSuffix) {
    var uid = idSuffix || 'isl';
    var isNight = document.body.classList.contains('sw-night');
    var seaOuter = isNight ? '#131f26' : '#c4d8df';
    var seaInner = isNight ? '#1b2c36' : '#d8e7ec';
    var rippleCol = isNight ? '#466878' : '#8fb3c0';
    var ink = isNight ? '#f5ecdf' : '#261b11';
    var halo = isNight ? '#171411' : '#f8f3e7';

    var parts = [];
    parts.push('<svg viewBox="' + viewBoxStr + '" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="外島彩色輿圖">');
    parts.push(
      '<defs>' +
      '<radialGradient id="oiSeaGrad_' + uid + '" cx="50%" cy="50%" r="65%">' +
      '<stop offset="0%" stop-color="' + seaInner + '" />' +
      '<stop offset="100%" stop-color="' + seaOuter + '" />' +
      '</radialGradient>' +
      '</defs>'
    );

    var vbNums = viewBoxStr.split(' ').map(Number);
    parts.push('<rect x="' + (vbNums[0] - 450) + '" y="' + (vbNums[1] - 600) + '" width="' + (vbNums[2] + 900) + '" height="' + (vbNums[3] + 1200) + '" fill="' + seaOuter + '" />');
    parts.push('<rect x="' + (vbNums[0] - 450) + '" y="' + (vbNums[1] - 600) + '" width="' + (vbNums[2] + 900) + '" height="' + (vbNums[3] + 1200) + '" fill="url(#oiSeaGrad_' + uid + ')" />');

    var cells = m.visited_cells || [];

    // 1. 海岸雙層潮汐水紋襯底
    for (var w = 0; w < cells.length; w++) {
      if (cells[w].is_coast) {
        var wp = polyPointsAttr(cells[w].polygon);
        parts.push('<polygon points="' + wp + '" fill="none" stroke="' + rippleCol + '" stroke-width="2.3" stroke-linejoin="round" opacity="0.32" />');
        parts.push('<polygon points="' + wp + '" fill="none" stroke="' + rippleCol + '" stroke-width="1.1" stroke-linejoin="round" opacity="0.52" />');
      }
    }

    // 2. 鄰接 1 環無色羊皮紙迷霧輪廓
    var outlines = m.outline_cells || [];
    for (var i = 0; i < outlines.length; i++) {
      parts.push(
        '<polygon points="' + polyPointsAttr(outlines[i].polygon) + '" ' +
        'fill="' + (isNight ? '#241f19' : '#ebe3cf') + '" ' +
        'fill-opacity="0.58" stroke="' + (isNight ? '#594e40' : '#9c8d74') + '" ' +
        'stroke-width="0.28" stroke-dasharray="1.2,1.0" />'
      );
    }

    // 3. 已踏足彩色多邊形地塊與手繪地貌符號（山巒／古林／溪流）
    for (var j = 0; j < cells.length; j++) {
      var c = cells[j];
      var fill = cellFillColor(c, isNight);
      var stroke = c.state_known ? (isNight ? '#cfa048' : '#8a4322') : (isNight ? '#5b5144' : '#7a6c58');
      var sw = c.cell_id === m.current_cell_id ? '0.68' : '0.32';
      parts.push(
        '<polygon points="' + polyPointsAttr(c.polygon) + '" fill="' + fill + '" stroke="' + stroke + '" stroke-width="' + sw + '" stroke-linejoin="round" />'
      );

      var cx = c.center[0];
      var cy = c.center[1];
      var reliefStroke = isNight ? '#968874' : '#5e503d';
      if (c.kind !== 'settlement') {
        if (c.height >= 24) {
          parts.push(
            '<path d="M ' + (cx - 2.1) + ',' + (cy + 0.9) + ' L ' + (cx - 0.7) + ',' + (cy - 1.2) + ' L ' + (cx + 0.5) + ',' + (cy + 0.7) +
            ' M ' + (cx - 0.1) + ',' + (cy + 0.9) + ' L ' + (cx + 1.1) + ',' + (cy - 0.8) + ' L ' + (cx + 2.2) + ',' + (cy + 0.9) +
            '" fill="none" stroke="' + reliefStroke + '" stroke-width="0.34" stroke-linecap="round" stroke-linejoin="round" opacity="0.72" />'
          );
        } else if (c.flux >= 30) {
          parts.push(
            '<path d="M ' + (cx - 1.8) + ',' + cy + ' Q ' + (cx - 0.9) + ',' + (cy - 0.6) + ' ' + cx + ',' + cy +
            ' T ' + (cx + 1.8) + ',' + cy + '" fill="none" stroke="' + (isNight ? '#7ab8cc' : '#2d6e84') + '" stroke-width="0.36" stroke-linecap="round" />'
          );
        } else if (!c.is_coast) {
          parts.push(
            '<circle cx="' + (cx - 0.8) + '" cy="' + (cy - 0.2) + '" r="0.75" fill="' + (isNight ? '#3c6148' : '#658c5a') + '" opacity="0.75" />' +
            '<circle cx="' + (cx + 0.7) + '" cy="' + (cy + 0.2) + '" r="0.68" fill="' + (isNight ? '#3c6148' : '#658c5a') + '" opacity="0.75" />' +
            '<circle cx="' + cx + '" cy="' + (cy - 0.8) + '" r="0.72" fill="' + (isNight ? '#497356' : '#739c67') + '" opacity="0.82" />'
          );
        }
      }
    }

    // 4. 海上航道虛線
    var seas = m.sea_corridors || [];
    for (var s = 0; s < seas.length; s++) {
      var sp = polyPointsAttr(seas[s].points);
      parts.push(
        '<polyline points="' + sp + '" fill="none" stroke="' + (isNight ? '#76b0cc' : '#316d8c') + '" stroke-width="0.45" stroke-dasharray="1.4,1.3" stroke-linecap="round" />'
      );
    }

    // 5. 雙層描邊島上幹道曲線
    var hwys = m.highways || [];
    for (var h = 0; h < hwys.length; h++) {
      var hp = polyPointsAttr(hwys[h].points);
      parts.push('<polyline points="' + hp + '" fill="none" stroke="' + (isNight ? '#261e16' : '#f5ebd6') + '" stroke-width="1.05" stroke-linecap="round" stroke-linejoin="round" />');
      parts.push('<polyline points="' + hp + '" fill="none" stroke="' + (isNight ? '#d9824c' : '#993d1d') + '" stroke-width="0.58" stroke-linecap="round" stroke-linejoin="round" />');
    }

    // 6. 聚落城堡徽記、地名與自身所在火漆印記
    for (var k = 0; k < cells.length; k++) {
      var vc = cells[k];
      var vx = vc.center[0];
      var vy = vc.center[1];
      if (vc.kind === 'settlement') {
        parts.push(
          '<g transform="translate(' + vx + ',' + vy + ')">' +
          '<rect x="-1.4" y="-1.1" width="2.8" height="2.2" rx="0.3" fill="#fcf6e8" stroke="#6e2c14" stroke-width="0.38" />' +
          '<path d="M -1.6,-1.1 L 0,-2.2 L 1.6,-1.1 Z" fill="#b34727" stroke="#6e2c14" stroke-width="0.3" />' +
          '</g>'
        );
      }
      if (vc.cell_id === m.current_cell_id) {
        parts.push('<circle cx="' + vx + '" cy="' + vy + '" r="2.35" fill="none" stroke="#d99f38" stroke-width="0.52" stroke-dasharray="1.1,0.7" />');
        parts.push('<circle cx="' + vx + '" cy="' + vy + '" r="0.82" fill="#c93824" stroke="#fdf8ee" stroke-width="0.28" />');
      }
      if (interactiveLabels && (vc.kind === 'settlement' || vc.cell_id === m.current_cell_id)) {
        var lblW = Math.max(8, vc.title.length * 2.3 + 1.8);
        parts.push(
          '<rect x="' + (vx - lblW / 2).toFixed(2) + '" y="' + (vy - 4.7).toFixed(2) + '" width="' + lblW.toFixed(2) + '" height="2.8" rx="0.6" fill="' + halo + '" />' +
          '<text x="' + vx + '" y="' + (vy - 2.6) + '" text-anchor="middle" font-size="2.15" font-weight="700" fill="' + ink + '">' +
          esc(vc.title) + '</text>'
        );
      }
    }

    parts.push('</svg>');
    return parts.join('');
  }

  function currentTownViewBox(baseW, baseH) {
    var bw = baseW || 640;
    var bh = baseH || 860;
    var z = Math.max(0.85, Math.min(3.2, appState.townCam.zoom));
    var w = bw / z;
    var h = bh / z;
    var cx = Math.max(110, Math.min(530, appState.townCam.cx)) * (bw / 640);
    var cy = Math.max(120, Math.min(740, appState.townCam.cy)) * (bh / 860);
    return (cx - w / 2).toFixed(1) + ' ' + (cy - h / 2).toFixed(1) + ' ' + w.toFixed(1) + ' ' + h.toFixed(1);
  }

  var wgPreloadImg = new Image();
  wgPreloadImg.src = '/maps/white_gull_harbor.webp?v=20261120a17';

  function buildWhiteGullIllustratedSvgHtml(m, activeTown, useFullViewBox, boxW, boxH) {
    var isNight = document.body.classList.contains('sw-night');
    var imgUrl = '/maps/white_gull_harbor.webp?v=20261120a17';
    var cw = Math.max(220, boxW || 380);
    var ch = Math.max(260, boxH || 560);
    var baseW = Math.min(cw, ch * 0.75);
    var baseH = baseW * (4 / 3);

    var z = useFullViewBox ? 1.0 : Math.max(0.85, Math.min(3.2, appState.townCam.zoom));
    var stW = baseW * z;
    var stH = baseH * z;
    var cx640 = useFullViewBox ? 320 : Math.max(110, Math.min(530, appState.townCam.cx));
    var cy860 = useFullViewBox ? 430 : Math.max(120, Math.min(740, appState.townCam.cy));
    var offX = ((320 - cx640) / 640) * stW * 2;
    var offY = ((430 - cy860) / 860) * stH * 2;
    var baseFontPx = Math.max(9.5, Math.min(13.5, baseW * 0.026 * Math.pow(z, 0.35))).toFixed(1);

    var parts = [];
    parts.push(
      '<div class="oi-wg-viewport' + (isNight ? ' is-night' : '') + '" role="region" aria-label="白鷗港手繪水彩海港輿圖">' +
      '<div class="oi-wg-stage" style="width:' + stW.toFixed(1) + 'px;height:' + stH.toFixed(1) + 'px;margin-left:' + offX.toFixed(1) + 'px;margin-top:' + offY.toFixed(1) + 'px;--wg-font:' + baseFontPx + 'px;">' +
      '<img class="oi-wg-bg-img" src="' + imgUrl + '" alt="" draggable="false" />'
    );

    if (isNight) {
      parts.push('<div class="oi-wg-night-shade"></div>');
      [
        [246, 306, 85],
        [340, 598, 105],
        [252, 885, 95],
        [645, 598, 80],
        [816, 320, 68],
        [342, 202, 58],
        [340, 1068, 58]
      ].forEach(function (gl) {
        var lx = (gl[0] / 900 * 100).toFixed(2);
        var ly = (gl[1] / 1200 * 100).toFixed(2);
        var rw = (gl[2] * 2 / 900 * 100).toFixed(2);
        var rh = (gl[2] * 2 / 1200 * 100).toFixed(2);
        parts.push('<span class="oi-wg-glow" style="left:' + lx + '%;top:' + ly + '%;width:' + rw + '%;height:' + rh + '%;"></span>');
      });
    }

    var nodeCoords = {
      'oi:white_gull_harbor:north_gate': [342, 202],
      'oi:white_gull_harbor:smithy_st': [342, 345],
      'oi:white_gull_harbor:plaza': [340, 598],
      'oi:white_gull_harbor:tavern_st': [340, 825],
      'oi:white_gull_harbor:south_gate': [340, 1068],
      'oi:white_gull_harbor:dock': [645, 598],
      'oi:white_gull_harbor:bal_forge': [248, 306],
      'oi:white_gull_harbor:herb_hall': [445, 306],
      'oi:white_gull_harbor:rusty_anchor_tavern': [252, 885],
      'oi:white_gull_harbor:inn_upstairs': [252, 885],
      'oi:white_gull_harbor:gale_carriage': [462, 898]
    };

    var alleyDefs = [
      { nid: 'oi:white_gull_harbor:iron_anvil_ln', name: '鐵砧巷', lx: 500, ly: 424, p0: [390, 455], p1: [555, 396] },
      { nid: 'oi:white_gull_harbor:sail_ln', name: '船帆巷', lx: 146, ly: 368, p0: [185, 396], p1: [118, 326] },
      { nid: 'oi:white_gull_harbor:copper_ln', name: '銅幣巷', lx: 526, ly: 506, p0: [455, 482], p1: [575, 522] },
      { nid: 'oi:white_gull_harbor:salted_fish_ln', name: '鹹魚巷', lx: 206, ly: 446, p0: [305, 465], p1: [114, 425] },
      { nid: 'oi:white_gull_harbor:cooper_ln', name: '桶匠巷', lx: 502, ly: 668, p0: [405, 628], p1: [568, 695] },
      { nid: 'oi:white_gull_harbor:rope_ln', name: '繩索巷', lx: 176, ly: 600, p0: [278, 598], p1: [98, 598] },
      { nid: 'oi:white_gull_harbor:lantern_ln', name: '石燈巷', lx: 496, ly: 746, p0: [398, 725], p1: [568, 772] },
      { nid: 'oi:white_gull_harbor:seabreeze_ln', name: '海風巷', lx: 198, ly: 758, p0: [308, 728], p1: [106, 782] },
      { nid: 'oi:white_gull_harbor:old_well_ln', name: '老井巷', lx: 564, ly: 840, p0: [545, 795], p1: [582, 885] },
      { nid: 'oi:white_gull_harbor:south_wall_ln', name: '南牆根巷', lx: 162, ly: 944, p0: [108, 855], p1: [212, 1022] }
    ];

    alleyDefs.forEach(function (ad) {
      nodeCoords[ad.nid] = [ad.lx, ad.ly - 4];
      for (var hi = 1; hi <= 12; hi++) {
        var t = (hi - 0.5) / 10;
        var hx = ad.p0[0] + (ad.p1[0] - ad.p0[0]) * Math.min(1, t);
        var hy = ad.p0[1] + (ad.p1[1] - ad.p0[1]) * Math.min(1, t) + ((hi % 2 === 0) ? 16 : -16);
        nodeCoords[ad.nid + ':h' + hi] = [hx, hy];
      }
    });

    function artLabel(nid, text, x, y, tier) {
      var lx = (x / 900 * 100).toFixed(2);
      var ly = ((y - 6) / 1200 * 100).toFixed(2);
      var cls = 'oi-wg-pill' + (tier ? (' oi-wg-pill-' + tier) : '');
      return (
        '<button type="button" class="' + cls + '" data-map-node="' + esc(nid) + '" style="left:' + lx + '%;top:' + ly + '%;">' +
        esc(text) +
        '</button>'
      );
    }

    parts.push(artLabel('oi:white_gull_harbor:north_gate', '北城門', 342, 162, 'main'));
    parts.push(artLabel('oi:white_gull_harbor:smithy_st', '鐵匠街', 342, 348, 'street'));
    parts.push(artLabel('oi:white_gull_harbor:plaza', '市集廣場', 340, 560, 'main'));
    parts.push(artLabel('oi:white_gull_harbor:tavern_st', '驛站街', 340, 828, 'street'));
    parts.push(artLabel('oi:white_gull_harbor:south_gate', '南城門', 340, 1114, 'main'));
    parts.push(artLabel('oi:white_gull_harbor:dock', '港務碼頭', 736, 605, 'main'));

    parts.push(artLabel('oi:white_gull_harbor:bal_forge', '巴爾鍛鐵鋪', 248, 376, 'shop'));
    parts.push(artLabel('oi:white_gull_harbor:herb_hall', '苦艾草藥堂', 448, 376, 'shop'));
    parts.push(artLabel('oi:white_gull_harbor:rusty_anchor_tavern', '鏽錨酒館', 254, 976, 'shop'));
    parts.push(artLabel('oi:white_gull_harbor:gale_carriage', '疾風驛馬行', 462, 840, 'shop'));

    alleyDefs.forEach(function (ad) {
      parts.push(artLabel(ad.nid, ad.name, ad.lx, ad.ly, 'alley'));
    });

    var curNid = m.current_node_id || '';
    var markerPt = nodeCoords[curNid];
    if (!markerPt && curNid.indexOf(':h') > 0) {
      markerPt = nodeCoords[curNid.replace(/:h\d+$/, '')];
    }
    if (markerPt) {
      var mx = (markerPt[0] / 900 * 100).toFixed(2);
      var my = (markerPt[1] / 1200 * 100).toFixed(2);
      parts.push('<span class="oi-wg-marker" style="left:' + mx + '%;top:' + my + '%;" aria-label="當前所在位置"></span>');
    }

    parts.push('</div></div>');
    return parts.join('');
  }

  function drawPitchedRoofSvg(rx, ry, rw, rh, angleDeg, sunCol, shadeCol, ridgeCol, shadowCol, strokeCol, titleText, hasWing) {
    var halfW = rw / 2;
    var halfH = rh / 2;
    var ridgeX = Math.max(2, halfW - 1.8);
    var wingSvg = '';
    if (hasWing) {
      var ww = (rw * 0.44).toFixed(1);
      var wh = (rh * 0.62).toFixed(1);
      wingSvg =
        '<rect x="' + (-halfW + 1.5).toFixed(1) + '" y="' + (halfH - 1.2).toFixed(1) + '" width="' + ww + '" height="' + wh + '" rx="0.4" fill="' + shadowCol + '" transform="translate(1.8,2.0)" />' +
        '<rect x="' + (-halfW + 1.5).toFixed(1) + '" y="' + (halfH - 1.2).toFixed(1) + '" width="' + ww + '" height="' + wh + '" rx="0.4" fill="' + shadeCol + '" stroke="' + strokeCol + '" stroke-width="0.8" />';
    }
    return (
      '<g transform="translate(' + rx.toFixed(1) + ',' + ry.toFixed(1) + ') rotate(' + angleDeg.toFixed(1) + ')">' +
      (titleText ? '<title>' + esc(titleText) + '</title>' : '') +
      '<rect x="' + (-halfW + 2.0).toFixed(1) + '" y="' + (-halfH + 2.4).toFixed(1) + '" width="' + rw.toFixed(1) + '" height="' + rh.toFixed(1) + '" rx="0.5" fill="' + shadowCol + '" />' +
      wingSvg +
      '<rect x="' + (-halfW).toFixed(1) + '" y="' + (-halfH).toFixed(1) + '" width="' + rw.toFixed(1) + '" height="' + halfH.toFixed(1) + '" fill="' + sunCol + '" />' +
      '<rect x="' + (-halfW).toFixed(1) + '" y="0" width="' + rw.toFixed(1) + '" height="' + halfH.toFixed(1) + '" fill="' + shadeCol + '" />' +
      '<rect x="' + (-halfW).toFixed(1) + '" y="' + (-halfH).toFixed(1) + '" width="' + rw.toFixed(1) + '" height="' + rh.toFixed(1) + '" rx="0.5" fill="none" stroke="' + strokeCol + '" stroke-width="0.9" />' +
      '<line x1="-' + ridgeX.toFixed(1) + '" y1="0" x2="' + ridgeX.toFixed(1) + '" y2="0" stroke="' + ridgeCol + '" stroke-width="1.05" stroke-linecap="round" />' +
      '</g>'
    );
  }

  function buildTownFishboneSvgHtml(m, useFullViewBox, boxW, boxH) {
    var isNight = document.body.classList.contains('sw-night');
    var towns = m.town_micro_maps || {};
    var curCid = m.current_cell_id;
    var activeTown = null;
    Object.keys(towns).forEach(function (slug) {
      if (towns[slug].cell_id === curCid) {
        activeTown = towns[slug];
      }
    });
    if (!activeTown) {
      var firstKey = Object.keys(towns)[0];
      activeTown = firstKey ? towns[firstKey] : null;
    }
    if (!activeTown) {
      return '<div style="padding:32px;text-align:center;">尚未踏入城鎮街區</div>';
    }

    var slug = activeTown.slug || 'white_gull_harbor';
    if (slug === 'white_gull_harbor') {
      return buildWhiteGullIllustratedSvgHtml(m, activeTown, useFullViewBox, boxW, boxH);
    }
    var vb = useFullViewBox ? '0 0 640 860' : currentTownViewBox(640, 860);
    var coastDir = activeTown.coast_dir || (activeTown.has_port ? 'east' : 'none');

    // 手繪古港羊皮紙與燭夜水彩調色盤
    var landInner = isNight ? '#272119' : '#f4ebd6';
    var landOuter = isNight ? '#1b1611' : '#e0d2b4';
    var plotFill = isNight ? '#211c15' : '#e6d8bc';
    var plotStroke = isNight ? '#42382c' : '#b2a183';
    var seaDeep = isNight ? '#12212a' : '#96bac7';
    var seaShallow = isNight ? '#1c323e' : '#bed9e2';
    var beachSand = isNight ? '#383024' : '#e5d6b5';
    var waveStroke = isNight ? '#4f788c' : '#5e8ea0';
    var roadKerb = isNight ? '#382f25' : '#948266';
    var roadFill = isNight ? '#57493a' : '#d5c5a7';
    var quayFill = isNight ? '#4a4035' : '#c7baa2';
    var wallShadow = isNight ? '#110e0b' : '#695b49';
    var wallStone = isNight ? '#827463' : '#d1c6b2';
    var wallStroke = isNight ? '#383027' : '#42362a';
    var shadowCol = isNight ? 'rgba(0,0,0,0.52)' : 'rgba(42,28,15,0.24)';
    var roofStroke = isNight ? '#211710' : '#472d1c';
    var ridgeLight = isNight ? '#c48d68' : '#f5cdb4';
    var roofPairs = isNight
      ? [
          ['#7d4b34', '#5c3524'],
          ['#87553b', '#633d29'],
          ['#6b4b3b', '#4d3529'],
          ['#5e5248', '#423932']
        ]
      : [
          ['#cf7148', '#a65331'],
          ['#c2653e', '#994b2b'],
          ['#b8784b', '#8f5934'],
          ['#8f7b68', '#6e5d4e']
        ];
    var vacantPair = isNight ? ['#a68438', '#7d6126'] : ['#dfb351', '#b88d35'];
    var ink = isNight ? '#f4eadb' : '#241910';
    var subInk = isNight ? '#ccbba4' : '#4f3e2d';
    var halo = isNight ? '#1c1712' : '#f4ebd6';

    var uid = useFullViewBox ? 'mini' : 'modal';
    var parts = [];
    parts.push('<svg viewBox="' + vb + '" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="' + esc(activeTown.name) + '街廓輿圖">');
    parts.push(
      '<defs>' +
      '<radialGradient id="townLandGrad_' + uid + '" cx="42%" cy="48%" r="55%">' +
      '<stop offset="0%" stop-color="' + landInner + '" />' +
      '<stop offset="100%" stop-color="' + landOuter + '" />' +
      '</radialGradient>' +
      '<linearGradient id="townSeaGrad_' + uid + '" x1="0%" y1="0%" x2="100%" y2="0%">' +
      '<stop offset="0%" stop-color="' + seaShallow + '" />' +
      '<stop offset="100%" stop-color="' + seaDeep + '" />' +
      '</linearGradient>' +
      '<pattern id="parchGrain_' + uid + '" width="28" height="28" patternUnits="userSpaceOnUse">' +
      '<circle cx="5" cy="7" r="0.55" fill="' + wallStroke + '" opacity="0.12" />' +
      '<circle cx="19" cy="15" r="0.5" fill="' + wallStroke + '" opacity="0.10" />' +
      '<circle cx="11" cy="24" r="0.45" fill="' + wallStroke + '" opacity="0.11" />' +
      '</pattern>' +
      '</defs>'
    );

    // 1. 無縫滿版羊皮紙陸地底圖（向四方大幅延伸，消除任何螢幕比例之藍邊裁切感）
    parts.push('<rect x="-900" y="-1200" width="2500" height="3300" fill="' + landOuter + '" />');
    parts.push('<rect x="-900" y="-1200" width="2500" height="3300" fill="url(#townLandGrad_' + uid + ')" />');
    parts.push('<rect x="-900" y="-1200" width="2500" height="3300" fill="url(#parchGrain_' + uid + ')" />');

    // 西側緩坡等高線與城外南北聯外古道
    var contourCol = isNight ? '#4a4033' : '#b8a88c';
    if (coastDir !== 'west') {
      parts.push('<path d="M -60,-300 C 12,-80 36,160 22,390 C 8,620 34,860 -20,1150" fill="none" stroke="' + contourCol + '" stroke-width="1.2" stroke-dasharray="8,5" opacity="0.5" />');
      parts.push('<path d="M -18,-260 C 42,-60 52,180 40,410 C 28,630 52,840 12,1120" fill="none" stroke="' + contourCol + '" stroke-width="0.9" opacity="0.4" />');
    }

    // 2. 滿版連續海岸線、潮間沙灘、四重手繪海圖水紋、石築防波堤燈塔、港務石岸與帆船群
    var dockPos = [542, 407];
    if (coastDir === 'east') {
      var eastCoast =
        'M 516,-1200 C 482,-820 532,-420 496,-60 ' +
        'C 476,85 502,195 482,298 ' +
        'C 464,376 468,448 492,535 ' +
        'C 516,640 484,745 508,860 ' +
        'C 534,1160 486,1560 522,2100 L 1650,2100 L 1650,-1200 Z';
      var eastShore =
        'M 516,-1200 C 482,-820 532,-420 496,-60 ' +
        'C 476,85 502,195 482,298 ' +
        'C 464,376 468,448 492,535 ' +
        'C 516,640 484,745 508,860 ' +
        'C 534,1160 486,1560 522,2100';
      parts.push('<path d="' + eastCoast + '" fill="' + seaDeep + '" />');
      parts.push('<path d="' + eastCoast + '" fill="url(#townSeaGrad_' + uid + ')" />');
      parts.push('<path d="' + eastShore + '" fill="none" stroke="' + beachSand + '" stroke-width="15" opacity="0.88" />');
      parts.push('<path d="' + eastShore + '" fill="none" stroke="' + wallStroke + '" stroke-width="1.7" opacity="0.72" />');
      [7, 15, 25, 38].forEach(function (off, wi) {
        parts.push(
          '<g transform="translate(' + off + ',0)">' +
          '<path d="' + eastShore + '" fill="none" stroke="' + waveStroke + '" stroke-width="' + (1.25 - wi * 0.22).toFixed(2) + '" ' +
          (wi >= 2 ? 'stroke-dasharray="14,7" ' : '') + 'opacity="' + (0.56 - wi * 0.11).toFixed(2) + '" />' +
          '</g>'
        );
      });

      // 港灣北翼弧形石築防波堤與銅頂燈標塔
      parts.push('<path d="M 480,302 Q 552,306 602,338" fill="none" stroke="' + wallShadow + '" stroke-width="11" stroke-linecap="round" />');
      parts.push('<path d="M 480,302 Q 552,306 602,338" fill="none" stroke="' + wallStone + '" stroke-width="6.8" stroke-linecap="round" />');
      parts.push('<path d="M 480,302 Q 552,306 602,338" fill="none" stroke="' + wallStroke + '" stroke-width="1.8" stroke-dasharray="6,4" />');
      parts.push('<circle cx="603" cy="338" r="7.2" fill="' + wallStone + '" stroke="' + wallStroke + '" stroke-width="1.4" />');
      parts.push('<circle cx="603" cy="338" r="3.8" fill="#e6a235" stroke="' + roofStroke + '" stroke-width="0.9" />');

      // 港務石砌駁岸平台與棧橋倉廄
      parts.push('<rect x="466" y="354" width="32" height="106" rx="2" fill="' + quayFill + '" stroke="' + wallStroke + '" stroke-width="1.4" />');
      parts.push(drawPitchedRoofSvg(483, 370, 18, 13, 2, roofPairs[3][0], roofPairs[3][1], ridgeLight, shadowCol, roofStroke, '港務北貨棧', false));
      parts.push(drawPitchedRoofSvg(483, 444, 18, 13, -2, roofPairs[2][0], roofPairs[2][1], ridgeLight, shadowCol, roofStroke, '港務鹽倉', false));

      // 延伸入海的重木石椿長棧橋與繫船柱
      parts.push('<rect x="494" y="396" width="98" height="22" rx="1.5" fill="' + shadowCol + '" transform="translate(2.5,3)" />');
      parts.push('<rect x="494" y="396" width="98" height="22" rx="1.5" fill="' + (isNight ? '#6e5036' : '#ba8d60') + '" stroke="' + roofStroke + '" stroke-width="1.3" />');
      for (var px = 502; px <= 586; px += 7) {
        parts.push('<line x1="' + px + '" y1="397" x2="' + px + '" y2="417" stroke="' + roofStroke + '" stroke-width="0.8" opacity="0.52" />');
      }
      [[508, 398], [538, 398], [568, 398], [508, 416], [538, 416], [568, 416]].forEach(function (bl) {
        parts.push('<circle cx="' + bl[0] + '" cy="' + bl[1] + '" r="1.5" fill="' + roofStroke + '" />');
      });

      // 停泊雙桅橫帆遠洋商船與兩艘沿岸漁舟
      parts.push(
        '<g transform="translate(568,468) rotate(-9)">' +
        '<ellipse cx="0" cy="2" rx="16" ry="40" fill="none" stroke="' + waveStroke + '" stroke-width="0.9" stroke-dasharray="5,4" opacity="0.55" />' +
        '<path d="M 0,-38 C 13,-20 14,18 8,34 L -8,34 C -14,18 -13,-20 0,-38 Z" fill="' + shadowCol + '" transform="translate(2.5,3)" />' +
        '<path d="M 0,-38 C 13,-20 14,18 8,34 L -8,34 C -14,18 -13,-20 0,-38 Z" fill="' + (isNight ? '#5c3e28' : '#99643b') + '" stroke="' + roofStroke + '" stroke-width="1.35" />' +
        '<path d="M 0,-29 C 8.5,-14 9,14 5,27 L -5,27 C -9,14 -8.5,-14 0,-29 Z" fill="' + (isNight ? '#7a563a' : '#c99a69') + '" />' +
        '<line x1="0" y1="-38" x2="0" y2="-48" stroke="' + roofStroke + '" stroke-width="1.5" stroke-linecap="round" />' +
        '<line x1="-16" y1="-11" x2="16" y2="-7" stroke="' + (isNight ? '#e3d9c8' : '#fcf8f0') + '" stroke-width="3.3" stroke-linecap="round" />' +
        '<line x1="-14" y1="12" x2="14" y2="15" stroke="' + (isNight ? '#e3d9c8' : '#fcf8f0') + '" stroke-width="2.9" stroke-linecap="round" />' +
        '<circle cx="0" cy="-9" r="2.3" fill="' + roofStroke + '" />' +
        '<circle cx="0" cy="13" r="2.1" fill="' + roofStroke + '" />' +
        '</g>'
      );
      [[524, 368, 18], [522, 444, -14]].forEach(function (sk) {
        parts.push(
          '<g transform="translate(' + sk[0] + ',' + sk[1] + ') rotate(' + sk[2] + ')">' +
          '<path d="M 0,-11 C 4.5,-5 4.5,6 2.5,11 L -2.5,11 C -4.5,6 -4.5,-5 0,-11 Z" fill="' + (isNight ? '#694b34' : '#ab774c') + '" stroke="' + roofStroke + '" stroke-width="1" />' +
          '</g>'
        );
      });
      dockPos = [542, 407];
    } else if (coastDir === 'west') {
      var westCoast =
        'M 142,-1200 C 174,-760 126,-360 154,0 ' +
        'C 172,140 138,260 162,380 ' +
        'C 176,455 168,530 146,640 ' +
        'C 124,740 160,820 138,920 ' +
        'C 118,1280 164,1680 132,2100 L -950,2100 L -950,-1200 Z';
      var westShore =
        'M 142,-1200 C 174,-760 126,-360 154,0 ' +
        'C 172,140 138,260 162,380 ' +
        'C 176,455 168,530 146,640 ' +
        'C 124,740 160,820 138,920 ' +
        'C 118,1280 164,1680 132,2100';
      parts.push('<path d="' + westCoast + '" fill="' + seaDeep + '" />');
      parts.push('<path d="' + westCoast + '" fill="url(#townSeaGrad_' + uid + ')" />');
      parts.push('<path d="' + westShore + '" fill="none" stroke="' + beachSand + '" stroke-width="15" opacity="0.88" />');
      parts.push('<path d="' + westShore + '" fill="none" stroke="' + wallStroke + '" stroke-width="1.6" opacity="0.72" />');
      [-8, -17, -28].forEach(function (off, wi) {
        parts.push('<g transform="translate(' + off + ',0)"><path d="' + westShore + '" fill="none" stroke="' + waveStroke + '" stroke-width="1.0" opacity="' + (0.5 - wi * 0.12).toFixed(2) + '" /></g>');
      });
      parts.push('<rect x="64" y="396" width="106" height="22" rx="1.5" fill="' + (isNight ? '#6e5036' : '#ba8d60') + '" stroke="' + roofStroke + '" stroke-width="1.3" />');
      dockPos = [112, 407];
    } else if (coastDir === 'south') {
      parts.push('<path d="M -900,790 Q -120,765 280,792 T 1550,782 L 1550,2100 L -900,2100 Z" fill="' + seaDeep + '" />');
      parts.push('<path d="M -900,790 Q -120,765 280,792 T 1550,782 L 1550,2100 L -900,2100 Z" fill="url(#townSeaGrad_' + uid + ')" />');
      parts.push('<rect x="462" y="396" width="96" height="20" rx="1.5" fill="' + (isNight ? '#6e5036' : '#ba8d60') + '" stroke="' + roofStroke + '" stroke-width="1.3" />');
      dockPos = [510, 406];
    } else if (coastDir === 'north') {
      parts.push('<path d="M -900,48 Q -120,72 280,45 T 1550,58 L 1550,-1200 L -900,-1200 Z" fill="' + seaDeep + '" />');
      parts.push('<path d="M -900,48 Q -120,72 280,45 T 1550,58 L 1550,-1200 L -900,-1200 Z" fill="url(#townSeaGrad_' + uid + ')" />');
      parts.push('<rect x="462" y="396" width="96" height="20" rx="1.5" fill="' + (isNight ? '#6e5036' : '#ba8d60') + '" stroke="' + roofStroke + '" stroke-width="1.3" />');
      dockPos = [510, 406];
    }

    // 城外南北延伸古道與郊野森林樹冠群
    var extRoadN = 'M 276,68 C 268,-40 296,-170 270,-340 C 248,-490 288,-680 266,-900';
    var extRoadS = 'M 276,758 C 284,865 258,995 282,1160 C 302,1310 268,1480 286,1700';
    [extRoadN, extRoadS].forEach(function (er) {
      parts.push('<path d="' + er + '" fill="none" stroke="' + roadKerb + '" stroke-width="15" stroke-linecap="round" opacity="0.72" />');
      parts.push('<path d="' + er + '" fill="none" stroke="' + roadFill + '" stroke-width="11" stroke-linecap="round" opacity="0.82" />');
    });

    var treeShade = isNight ? '#243a2b' : '#6c8a5e';
    var treeSun = isNight ? '#33523d' : '#8ead7c';
    [
      [112, 22], [138, 12], [164, 28], [386, 18], [418, 30], [198, -28], [356, -22], [130, -68], [404, -64],
      [96, 804], [128, 818], [162, 802], [388, 808], [420, 796], [196, 858], [362, 864],
      [22, 196], [30, 218], [18, 468], [28, 492], [24, 636]
    ].forEach(function (pt) {
      if (coastDir === 'west' && pt[0] < 180) return;
      parts.push('<circle cx="' + (pt[0] + 2) + '" cy="' + (pt[1] + 2) + '" r="10.5" fill="' + shadowCol + '" />');
      parts.push('<circle cx="' + pt[0] + '" cy="' + pt[1] + '" r="10" fill="' + treeShade + '" opacity="0.82" />');
      parts.push('<circle cx="' + (pt[0] - 2.2) + '" cy="' + (pt[1] - 2.2) + '" r="6.8" fill="' + treeSun + '" opacity="0.85" />');
    });

    // 3. 城內院落街廓底圖（先鋪石牆後院地塊，消除屋頂懸浮感）
    var alleys = activeTown.alleys || [];
    var roofs = activeTown.roofs || [];
    var totalRows = Math.max(1, Math.ceil(alleys.length / 2));

    function rowConfig(idx) {
      var isEast = alleys[idx].side === 'east';
      var r = Math.floor(idx / 2);
      if (totalRows >= 5) {
        if (r === 0) return { y: isEast ? 130 : 136, x0: isEast ? 286 : 266, x1: isEast ? 430 : 110, plazaBranch: false };
        if (r === 1) return { y: isEast ? 208 : 214, x0: isEast ? 286 : 266, x1: isEast ? 444 : 96, plazaBranch: false };
        if (r === 2) {
          return isEast
            ? { y: 360, x0: 314, x1: 446, plazaBranch: true, startY: 382 }
            : { y: 404, x0: 230, x1: 94, plazaBranch: true, startY: 404 };
        }
        if (r === 3) return { y: isEast ? 596 : 602, x0: isEast ? 286 : 266, x1: isEast ? 444 : 96, plazaBranch: false };
        return { y: isEast ? 680 : 686, x0: isEast ? 286 : 266, x1: isEast ? 426 : 114, plazaBranch: false };
      }
      var simpleY = 178 + r * 195 + (isEast ? -6 : 6);
      if (Math.abs(simpleY - 404) < 55) {
        return isEast
          ? { y: 360, x0: 314, x1: 444, plazaBranch: true, startY: 382 }
          : { y: 404, x0: 230, x1: 96, plazaBranch: true, startY: 404 };
      }
      return { y: simpleY, x0: isEast ? 286 : 266, x1: isEast ? 438 : 102, plazaBranch: false };
    }

    // 四大地標專屬石砌院落襯底（完全避開所有支巷與廣場，零重疊！）
    var lmPlots = [
      { x: 158, y: 256, w: 94, h: 52 }, // 巴爾鍛鐵鋪院落
      { x: 300, y: 256, w: 98, h: 52 }, // 苦艾草藥堂藥圃院
      { x: 150, y: 472, w: 102, h: 56 }, // 鏽錨酒館前庭院
      { x: 300, y: 472, w: 106, h: 56 }  // 疾風驛馬行車馬院
    ];
    lmPlots.forEach(function (pl) {
      parts.push('<rect x="' + pl.x + '" y="' + pl.y + '" width="' + pl.w + '" height="' + pl.h + '" rx="3" fill="' + plotFill + '" stroke="' + plotStroke + '" stroke-width="1.0" />');
    });

    // 各支巷兩側石籬民居地塊襯底
    alleys.forEach(function (al, idx) {
      var rc = rowConfig(idx);
      var minX = Math.min(rc.x0, rc.x1);
      var spanW = Math.abs(rc.x1 - rc.x0);
      parts.push('<rect x="' + minX.toFixed(1) + '" y="' + (rc.y - 26).toFixed(1) + '" width="' + spanW.toFixed(1) + '" height="52" rx="4" fill="' + plotFill + '" stroke="' + plotStroke + '" stroke-width="0.75" opacity="0.86" />');
    });

    // 4. 城內石板主幹道、港務大道、環城巡道、古井市集廣場與支巷路網
    var spinePath = 'M 276,68 C 271,185 281,295 276,404 C 271,520 281,640 276,758';
    parts.push('<path d="' + spinePath + '" fill="none" stroke="' + roadKerb + '" stroke-width="24" stroke-linecap="round" />');
    parts.push('<path d="' + spinePath + '" fill="none" stroke="' + roadFill + '" stroke-width="19" stroke-linecap="round" />');

    if (activeTown.has_port) {
      var dockRoad = coastDir === 'west'
        ? 'M 232,404 C 202,402 182,409 162,407'
        : 'M 320,406 C 372,403 422,410 494,407';
      parts.push('<path d="' + dockRoad + '" fill="none" stroke="' + roadKerb + '" stroke-width="20" stroke-linecap="round" />');
      parts.push('<path d="' + dockRoad + '" fill="none" stroke="' + roadFill + '" stroke-width="15.5" stroke-linecap="round" />');
    }

    // 中央市集廣場（同心石板紋、八方地磚星芒、有頂石井與四隅布篷攤位）
    parts.push('<circle cx="276" cy="404" r="45" fill="' + roadFill + '" stroke="' + roadKerb + '" stroke-width="2.6" />');
    parts.push('<circle cx="276" cy="404" r="35" fill="none" stroke="' + roadKerb + '" stroke-width="1.0" stroke-dasharray="4,3.5" />');
    parts.push('<circle cx="276" cy="404" r="22" fill="none" stroke="' + roadKerb + '" stroke-width="0.8" opacity="0.65" />');
    [[250, 378, '#c95b3c'], [300, 376, '#4c8062'], [248, 428, '#bf8c36'], [302, 430, '#4f6f8c']].forEach(function (st) {
      parts.push('<rect x="' + (st[0] - 5) + '" y="' + (st[1] - 3.5) + '" width="10" height="7" rx="1" fill="' + st[2] + '" stroke="' + roofStroke + '" stroke-width="0.7" opacity="0.88" />');
    });
    parts.push('<circle cx="276" cy="404" r="8.5" fill="' + wallStone + '" stroke="' + wallStroke + '" stroke-width="1.6" />');
    parts.push('<circle cx="276" cy="404" r="4.2" fill="' + seaDeep + '" />');

    var nodeCoords = {};
    var sp = activeTown.spine || {};
    nodeCoords[sp.north_gate || ('oi:' + slug + ':north_gate')] = [276, 72];
    nodeCoords[sp.smithy_st || ('oi:' + slug + ':smithy_st')] = [276, 294];
    nodeCoords[sp.plaza || ('oi:' + slug + ':plaza')] = [276, 404];
    nodeCoords[sp.tavern_st || ('oi:' + slug + ':tavern_st')] = [276, 502];
    nodeCoords[sp.south_gate || ('oi:' + slug + ':south_gate')] = [276, 754];
    if (activeTown.has_port) {
      nodeCoords[sp.dock || ('oi:' + slug + ':dock')] = dockPos;
    }

    // 5. 支巷石板路與緊密錯落雙坡斜瓦屋頂群（附後院橄欖綠庭樹）
    var alleyLabelQueue = [];
    alleys.forEach(function (al, idx) {
      var isEast = al.side === 'east';
      var rc = rowConfig(idx);
      var sY = rc.plazaBranch ? rc.startY : rc.y;
      var ctrlX = (rc.x0 + rc.x1) / 2;
      var bowY = rc.y + ((idx % 2 === 0) ? -6 : 6);
      var aPath = 'M ' + rc.x0 + ',' + sY + ' Q ' + ctrlX.toFixed(1) + ',' + bowY.toFixed(1) + ' ' + rc.x1 + ',' + rc.y;
      parts.push('<path d="' + aPath + '" fill="none" stroke="' + roadKerb + '" stroke-width="10.5" stroke-linecap="round" />');
      parts.push('<path d="' + aPath + '" fill="none" stroke="' + roadFill + '" stroke-width="7.6" stroke-linecap="round" />');

      var aNid = al.node_id || ('oi:' + slug + ':' + idx);
      nodeCoords[aNid] = [(rc.x0 + rc.x1) / 2, rc.y];
      var edgeInset = (idx <= 1) ? 16 : 2;
      alleyLabelQueue.push({
        nid: aNid,
        name: al.name,
        x: isEast ? (rc.x1 - edgeInset) : (rc.x1 + edgeInset),
        y: rc.y - 29,
        anchor: isEast ? 'end' : 'start'
      });

      var myRoofs = roofs.filter(function (r) { return r.alley_idx === idx; });
      var topRoofs = [];
      var botRoofs = [];
      myRoofs.forEach(function (rf, rIdx) {
        if (rIdx % 2 === 0) topRoofs.push(rf);
        else botRoofs.push(rf);
      });

      [topRoofs, botRoofs].forEach(function (rowList, sideIdx) {
        var sideSign = sideIdx === 0 ? -1 : 1;
        var count = Math.max(1, rowList.length);
        rowList.forEach(function (rf, k) {
          var t = (k + 0.55) / (count + 0.15);
          // 避開廣場邊緣，從支巷內側整齊排開
          var tMapped = rc.plazaBranch ? (0.14 + t * 0.82) : (0.08 + t * 0.86);
          var inv = 1 - tMapped;
          var curveX = inv * inv * rc.x0 + 2 * inv * tMapped * ctrlX + tMapped * tMapped * rc.x1;
          var curveY = inv * inv * sY + 2 * inv * tMapped * bowY + tMapped * tMapped * rc.y;
          var hval = detHash(rf.label || (idx + ':' + sideIdx + ':' + k));
          var rw = 19.5 + (hval % 5);
          var rh = 12.2 + ((hval >> 3) % 4);
          var ry = curveY + sideSign * (13.6 + (hval % 2));
          var ang = (((hval >> 6) % 7) - 3) * 0.9;
          var pair = rf.vacant ? vacantPair : roofPairs[hval % roofPairs.length];
          var hasWing = !rf.vacant && ((hval % 5) === 0) && sideSign > 0;
          if (rf.node_id) {
            nodeCoords[rf.node_id] = [curveX, ry];
          }
          if ((hval % 3) === 0) {
            parts.push('<circle cx="' + (curveX + (sideSign * 6)).toFixed(1) + '" cy="' + (ry + sideSign * 8.5).toFixed(1) + '" r="3.8" fill="' + treeSun + '" opacity="0.72" />');
          }
          parts.push(drawPitchedRoofSvg(curveX, ry, rw, rh, ang, pair[0], pair[1], ridgeLight, shadowCol, roofStroke, rf.label, hasWing));
        });
      });
    });

    // 6. 四大具名地標建築群（座落各自獨立石砌院落，與支巷零干涉）
    var lms = activeTown.landmarks || [];
    var lmConfigs = [
      { kind: 'forge', x: 204, y: 281, w: 38, h: 21, ang: -2, sun: isNight ? '#69584d' : '#997a64', shade: isNight ? '#4a3d35' : '#735947', labelY: 323 },
      { kind: 'herb', x: 334, y: 281, w: 34, h: 20, ang: 2, sun: isNight ? '#496b54' : '#789e7a', shade: isNight ? '#334d3b' : '#567858', labelY: 323 },
      { kind: 'tavern', x: 202, y: 498, w: 42, h: 23, ang: 1.5, sun: isNight ? '#8c4e34' : '#c95c38', shade: isNight ? '#633522' : '#963f22', labelY: 544 },
      { kind: 'carriage', x: 334, y: 498, w: 38, h: 21, ang: -1.5, sun: isNight ? '#7d5e3f' : '#b58759', shade: isNight ? '#59422b' : '#8a643e', labelY: 544 }
    ];
    lmConfigs.forEach(function (cfg, lIdx) {
      var lm = lms[lIdx] || { node_id: '', label: '' };
      if (lm.node_id) {
        nodeCoords[lm.node_id] = [cfg.x, cfg.y];
        if (cfg.kind === 'tavern') {
          nodeCoords['oi:' + slug + ':inn_upstairs'] = [cfg.x, cfg.y];
        }
      }
      if (cfg.kind === 'herb') {
        // 藥圃三畦青草畦床
        [266, 278, 290].forEach(function (gy) {
          parts.push('<rect x="360" y="' + gy + '" width="28" height="7" rx="1.5" fill="' + (isNight ? '#2f4d38' : '#94ba82') + '" stroke="' + roofStroke + '" stroke-width="0.75" />');
        });
      } else if (cfg.kind === 'carriage') {
        // 車馬木欄圍場與飲馬水槽
        parts.push('<rect x="362" y="482" width="36" height="30" rx="2" fill="none" stroke="' + roofStroke + '" stroke-width="1.2" stroke-dasharray="4,2.5" />');
        parts.push('<rect x="368" y="487" width="14" height="5" rx="1" fill="' + seaShallow + '" stroke="' + roofStroke + '" stroke-width="0.7" />');
      } else if (cfg.kind === 'tavern') {
        // 酒館前庭橡木酒桶
        parts.push('<circle cx="234" cy="486" r="3.2" fill="#9c6840" stroke="' + roofStroke + '" stroke-width="0.7" />');
        parts.push('<circle cx="241" cy="491" r="3.2" fill="#9c6840" stroke="' + roofStroke + '" stroke-width="0.7" />');
      }
      parts.push(drawPitchedRoofSvg(cfg.x, cfg.y, cfg.w, cfg.h, cfg.ang, cfg.sun, cfg.shade, ridgeLight, shadowCol, roofStroke, lm.label, cfg.kind === 'tavern' || cfg.kind === 'forge'));
      if (cfg.kind === 'forge') {
        // 矮人石鍛爐紅炭煙囪與鐵砧
        parts.push('<circle cx="174" cy="274" r="5.2" fill="' + wallStone + '" stroke="' + wallStroke + '" stroke-width="1.1" />');
        parts.push('<circle cx="174" cy="274" r="2.8" fill="#eb6b26" />');
      }
    });

    // 7. 多邊形厚石城牆、垛口與圓形角樓／海門雙塔（無牆聚落繪原木柵欄哨塔）
    if (activeTown.walled) {
      var wallWest = coastDir === 'west'
        ? 'M 256,68 L 172,82 L 162,388 M 162,426 L 172,738 L 256,758'
        : 'M 256,68 L 132,80 L 72,138 L 58,274 L 58,548 L 74,686 L 140,746 L 256,758';
      var wallEast = (activeTown.has_port && coastDir !== 'west')
        ? 'M 296,68 L 414,82 L 458,144 L 468,288 L 466,388 M 466,426 L 468,554 L 452,688 L 396,746 L 296,758'
        : 'M 296,68 L 414,82 L 458,144 L 468,410 L 452,688 L 396,746 L 296,758';

      [wallWest, wallEast].forEach(function (seg) {
        parts.push('<path d="' + seg + '" fill="none" stroke="' + wallShadow + '" stroke-width="11.5" stroke-linejoin="round" stroke-linecap="round" />');
        parts.push('<path d="' + seg + '" fill="none" stroke="' + wallStone + '" stroke-width="7" stroke-linejoin="round" stroke-linecap="round" />');
        parts.push('<path d="' + seg + '" fill="none" stroke="' + wallStroke + '" stroke-width="2.2" stroke-dasharray="5,5" stroke-linejoin="round" />');
      });

      var towers = [
        [256, 68], [296, 68], [256, 758], [296, 758],
        [414, 82], [458, 144], [452, 688], [396, 746]
      ];
      if (coastDir !== 'west') {
        towers.push([132, 80], [72, 138], [58, 274], [58, 548], [74, 686], [140, 746]);
      }
      if (activeTown.has_port && coastDir !== 'west') {
        towers.push([466, 388], [466, 426]);
      } else if (activeTown.has_port && coastDir === 'west') {
        towers.push([162, 388], [162, 426]);
      }
      towers.forEach(function (tw) {
        parts.push('<circle cx="' + (tw[0] + 1.8) + '" cy="' + (tw[1] + 2.2) + '" r="8.6" fill="' + shadowCol + '" />');
        parts.push('<circle cx="' + tw[0] + '" cy="' + tw[1] + '" r="8.2" fill="' + wallStone + '" stroke="' + wallStroke + '" stroke-width="1.6" />');
        parts.push('<circle cx="' + tw[0] + '" cy="' + tw[1] + '" r="3.4" fill="' + (isNight ? '#594a3c' : '#96846e') + '" />');
      });
    } else {
      [[258, 72], [294, 72], [258, 754], [294, 754]].forEach(function (pt) {
        parts.push('<circle cx="' + pt[0] + '" cy="' + pt[1] + '" r="6" fill="' + (isNight ? '#6e5036' : '#a67b51') + '" stroke="' + roofStroke + '" stroke-width="1.3" />');
      });
    }

    // 8. 可點擊地標與支巷書法字標（清晰描邊光暈、零壓字）
    function mapLabel(nid, text, x, y, size, weight, anchor, fillOverride) {
      var attr = nid ? (' data-map-node="' + esc(nid) + '" style="cursor:pointer;"') : '';
      var pw = Math.max(44, text.length * (size + 1) + 12);
      var ph = size + 8;
      var px = (anchor === 'start') ? (x - 6) : (anchor === 'end' ? (x - pw + 6) : (x - pw / 2));
      return '<g' + attr + '>' +
        '<rect x="' + px.toFixed(1) + '" y="' + (y - size * 0.84 - 3).toFixed(1) + '" width="' + pw.toFixed(1) + '" height="' + ph.toFixed(1) + '" rx="5" fill="' + halo + '" />' +
        '<text x="' + x + '" y="' + y + '" text-anchor="' + (anchor || 'middle') + '" font-size="' + size + '" font-weight="' + (weight || '700') + '" fill="' + (fillOverride || ink) + '">' +
        esc(text) + '</text></g>';
    }

    var gateWordN = activeTown.walled ? '北城門' : '北哨柵口';
    var gateWordS = activeTown.walled ? '南城門' : '南哨柵口';
    parts.push(mapLabel(sp.north_gate, gateWordN, 276, 44, 13.5, '700', 'middle'));
    parts.push(mapLabel(sp.smithy_st, '鐵匠街', 276, 253, 12.2, '700', 'middle', subInk));
    parts.push(mapLabel(sp.plaza, '市集廣場', 276, 386, 13, '700', 'middle'));
    parts.push(mapLabel(sp.tavern_st, '驛站街', 276, 468, 12.2, '700', 'middle', subInk));
    parts.push(mapLabel(sp.south_gate, gateWordS, 276, 786, 13.5, '700', 'middle'));
    if (activeTown.has_port) {
      parts.push(mapLabel(sp.dock, '港務碼頭', dockPos[0], dockPos[1] - 18, 13, '700', 'middle'));
    }

    lmConfigs.forEach(function (cfg, lIdx) {
      var lm = lms[lIdx];
      if (lm && lm.label) {
        var lx = (lIdx === 1 || lIdx === 3) ? (cfg.x + 15) : cfg.x;
        parts.push(mapLabel(lm.node_id, lm.label, lx, cfg.labelY, 11.4, '700', 'middle', isNight ? '#ebb988' : '#6e2c14'));
      }
    });

    alleyLabelQueue.forEach(function (al) {
      parts.push(mapLabel(al.nid, al.name, al.x, al.y, 11.2, '600', al.anchor, subInk));
    });

    // 9. 古典航海八芒羅盤玫瑰
    var compX = coastDir === 'west' ? 576 : 580;
    var compY = 96;
    parts.push(
      '<g transform="translate(' + compX + ',' + compY + ')" opacity="0.86">' +
      '<circle cx="0" cy="0" r="23" fill="none" stroke="' + wallStroke + '" stroke-width="1.1" />' +
      '<circle cx="0" cy="0" r="18.5" fill="none" stroke="' + wallStroke + '" stroke-width="0.65" stroke-dasharray="2,2" />' +
      '<polygon points="0,-25 4.2,-5 0,-9 -4.2,-5" fill="#b33922" />' +
      '<polygon points="0,25 4.2,5 0,9 -4.2,5" fill="' + wallStroke + '" />' +
      '<polygon points="-25,0 -5,-4.2 -9,0 -5,4.2" fill="' + wallStroke + '" />' +
      '<polygon points="25,0 5,-4.2 9,0 5,4.2" fill="' + wallStroke + '" />' +
      '<text x="0" y="-29" text-anchor="middle" font-size="10.5" font-weight="700" fill="' + ink + '">北</text>' +
      '</g>'
    );

    // 10. 當前所在位置朱紅火漆雙環印記
    var curNid = m.current_node_id || '';
    var markerPt = nodeCoords[curNid];
    if (!markerPt && curNid.indexOf(':h') > 0) {
      var parentAlley = curNid.replace(/:h\d+$/, '');
      markerPt = nodeCoords[parentAlley];
    }
    if (markerPt) {
      parts.push(
        '<g transform="translate(' + markerPt[0].toFixed(1) + ',' + markerPt[1].toFixed(1) + ')">' +
        '<circle cx="0" cy="0" r="15" fill="rgba(201,56,36,0.18)" stroke="#d99f38" stroke-width="1.9" />' +
        '<circle cx="0" cy="0" r="6.2" fill="#c93824" stroke="#fdf8ee" stroke-width="2" />' +
        '</g>'
      );
    }

    parts.push('</svg>');
    return parts.join('');
  }

  function buildDungeonSvgHtml(m) {
    var dng = m.dungeon_local;
    if (!dng || !dng.rooms) {
      return '<div style="padding:32px;text-align:center;">目前不在地底坑道之中</div>';
    }
    var rooms = dng.rooms;
    var roomById = {};
    rooms.forEach(function (r) { roomById[r.id] = r; });

    var parts = [];
    parts.push('<svg viewBox="-60 -60 360 320" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="坑道手繪局部草圖">');
    parts.push('<rect x="-60" y="-60" width="360" height="320" fill="#181512" />');

    rooms.forEach(function (r) {
      var x1 = r.pos[0] * 110;
      var y1 = r.pos[1] * 95;
      (r.exits || []).forEach(function (ex) {
        var dst = roomById[ex.to];
        if (dst) {
          var x2 = dst.pos[0] * 110;
          var y2 = dst.pos[1] * 95;
          parts.push('<line x1="' + x1 + '" y1="' + y1 + '" x2="' + x2 + '" y2="' + y2 + '" stroke="#6e5d49" stroke-width="6" />');
        }
      });
    });

    rooms.forEach(function (r) {
      var cx = r.pos[0] * 110;
      var cy = r.pos[1] * 95;
      var fill = r.current ? '#8b3a1e' : '#2d2620';
      var stroke = r.current ? '#f0c36d' : '#8a7a66';
      var shortName = (r.name || '').split('・').pop() || r.name;
      parts.push('<rect x="' + (cx - 38) + '" y="' + (cy - 20) + '" width="76" height="40" rx="7" fill="' + fill + '" stroke="' + stroke + '" stroke-width="2" />');
      parts.push('<text x="' + cx + '" y="' + (cy + 4) + '" text-anchor="middle" font-size="11" fill="#f3eadb">' + esc(shortName) + '</text>');
    });

    parts.push('</svg>');
    return parts.join('');
  }

  function currentIslandViewBox(m) {
    var b = (m && m.camera_bounds) || [690, 510, 742, 562];
    var minX = b[0], minY = b[1], maxX = b[2], maxY = b[3];
    var baseW = Math.max(24, maxX - minX);
    var baseH = Math.max(24, maxY - minY);
    var z = Math.max(0.85, Math.min(4.5, appState.cam.zoom));
    var w = baseW / z;
    var h = baseH / z;
    var cx = Math.max(minX, Math.min(maxX, appState.cam.cx));
    var cy = Math.max(minY, Math.min(maxY, appState.cam.cy));
    appState.cam.cx = cx;
    appState.cam.cy = cy;
    return (cx - w / 2).toFixed(2) + ' ' + (cy - h / 2).toFixed(2) + ' ' + w.toFixed(2) + ' ' + h.toFixed(2);
  }

  function renderDesktopMiniMap() {
    var wrap = byId('oi-mini-map-frame');
    var m = appState.mapData;
    if (!wrap || !m) return;
    Array.prototype.forEach.call(document.querySelectorAll('[data-aside-mode]'), function (btn) {
      btn.classList.toggle('active', btn.getAttribute('data-aside-mode') === appState.asideMode);
    });
    if (m.in_dungeon) {
      wrap.innerHTML = buildDungeonSvgHtml(m);
      return;
    }
    var towns = m.town_micro_maps || {};
    var inTown = Object.keys(towns).some(function (k) {
      return towns[k].cell_id === m.current_cell_id;
    });
    if (appState.asideMode === 'town' && inTown) {
      wrap.innerHTML = buildTownFishboneSvgHtml(m, true, wrap.clientWidth, wrap.clientHeight);
      return;
    }
    var b = m.camera_bounds || [690, 510, 742, 562];
    var vb = b[0].toFixed(1) + ' ' + b[1].toFixed(1) + ' ' + (b[2] - b[0]).toFixed(1) + ' ' + (b[3] - b[1]).toFixed(1);
    wrap.innerHTML = buildIslandSvgHtml(m, vb, true, 'mini');
  }

  function renderFullscreenMap() {
    var m = appState.mapData;
    var canvas = byId('oi-map-canvas');
    if (!m || !canvas) return;

    var dngTab = byId('oi-map-tab-dungeon');
    if (dngTab) dngTab.hidden = !m.in_dungeon;

    Array.prototype.forEach.call(document.querySelectorAll('[data-map-scale]'), function (btn) {
      btn.classList.toggle('active', btn.getAttribute('data-map-scale') === appState.mapScaleMode);
    });

    if (appState.mapScaleMode === 'town') {
      byId('oi-map-banner').textContent = m.current_node_name || m.world_name;
      canvas.innerHTML = buildTownFishboneSvgHtml(m, false, canvas.clientWidth, canvas.clientHeight);
    } else if (appState.mapScaleMode === 'dungeon' && m.in_dungeon) {
      byId('oi-map-banner').textContent = m.current_node_name || '地底石室';
      canvas.innerHTML = buildDungeonSvgHtml(m);
    } else {
      byId('oi-map-banner').textContent = m.state_name || m.world_name;
      var vb = currentIslandViewBox(m);
      canvas.innerHTML = buildIslandSvgHtml(m, vb, true, 'modal');
    }
  }

  /* ═══ 全域事件委派與手機虛擬鍵盤貼齊 ═══ */
  function bindMainEvents() {
    // 晝夜切換
    byId('oi-btn-theme').addEventListener('click', function () {
      var currentlyNight = document.body.classList.contains('sw-night');
      appState.themeOverride = currentlyNight ? 'day' : 'night';
      syncTheme();
      renderDesktopMiniMap();
      if (appState.mapOpen) renderFullscreenMap();
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
          personBtn.getAttribute('data-npc-role')
        );
        return;
      }

      var actBtn = ev.target.closest('[data-act]');
      if (actBtn) {
        var act = actBtn.getAttribute('data-act');
        apiJson('/api/island/act', 'POST', { action: act, param: '' }).then(function (res) {
          if (res && res.view) applyServerView(res.view);
          if (res && !res.ok && res.msg) showToast(res.msg);
          refreshMapData(false);
        });
        return;
      }

      var asideTab = ev.target.closest('[data-aside-mode]');
      if (asideTab) {
        appState.asideMode = asideTab.getAttribute('data-aside-mode');
        renderDesktopMiniMap();
        return;
      }

      var mapNode = ev.target.closest('[data-map-node]');
      if (mapNode) {
        var targetNid = mapNode.getAttribute('data-map-node');
        if (targetNid && targetNid !== (appState.view && appState.view.scene && appState.view.scene.node_id)) {
          apiJson('/api/island/move', 'POST', { to: targetNid }).then(function (res) {
            if (res && res.view) applyServerView(res.view);
            if (res && !res.ok && res.msg) showToast(res.msg);
            refreshMapData(false);
          });
        }
        return;
      }

      var moveBtn = ev.target.closest('[data-move-to]');
      if (moveBtn) {
        var dst = moveBtn.getAttribute('data-move-to');
        apiJson('/api/island/move', 'POST', { to: dst }).then(function (res) {
          if (res && res.view) applyServerView(res.view);
          if (res && !res.ok && res.msg) showToast(res.msg);
          refreshMapData(true);
        });
        return;
      }
    });

    // 底欄五鍵與桌機展開輿圖
    byId('oi-nav-self').addEventListener('click', function () { openInfoSheet('self'); });
    byId('oi-nav-bag').addEventListener('click', function () { openInfoSheet('bag'); });
    byId('oi-nav-recap').addEventListener('click', function () { openInfoSheet('recap'); });
    byId('oi-nav-explore').addEventListener('click', function () {
      apiJson('/api/island/act', 'POST', { action: 'explore', param: '' }).then(function (res) {
        if (res && res.view) applyServerView(res.view);
        if (res && (!res.ok || !res.view) && res.msg) showToast(res.msg);
        refreshMapData(false);
      });
    });

    function openMapModal() {
      appState.mapOpen = true;
      byId('oi-map-modal').hidden = false;
      renderFullscreenMap();
    }
    byId('oi-nav-map').addEventListener('click', openMapModal);
    byId('oi-btn-expand-map').addEventListener('click', openMapModal);
    byId('oi-map-close').addEventListener('click', function () {
      appState.mapOpen = false;
      byId('oi-map-modal').hidden = true;
    });

    // 輿圖尺度切換、地標點擊移動與縮放平移
    byId('oi-map-modal').addEventListener('click', function (ev) {
      var scaleBtn = ev.target.closest('[data-map-scale]');
      if (scaleBtn) {
        appState.mapScaleMode = scaleBtn.getAttribute('data-map-scale');
        renderFullscreenMap();
        return;
      }
      var mapNode = ev.target.closest('[data-map-node]');
      if (mapNode) {
        var targetNid = mapNode.getAttribute('data-map-node');
        if (targetNid && targetNid !== (appState.view && appState.view.scene && appState.view.scene.node_id)) {
          apiJson('/api/island/move', 'POST', { to: targetNid }).then(function (res) {
            if (res && res.view) applyServerView(res.view);
            if (res && !res.ok && res.msg) showToast(res.msg);
            refreshMapData(false);
          });
        }
      }
    });

    byId('oi-map-zoom-in').addEventListener('click', function () {
      if (appState.mapScaleMode === 'town') {
        appState.townCam.zoom = Math.min(3.2, appState.townCam.zoom * 1.35);
      } else {
        appState.cam.zoom = Math.min(4.5, appState.cam.zoom * 1.35);
        if (appState.cam.zoom >= 2.8 && appState.mapScaleMode === 'island' && appState.mapData) {
          var towns = appState.mapData.town_micro_maps || {};
          var inTown = Object.keys(towns).some(function (k) {
            return towns[k].cell_id === appState.mapData.current_cell_id;
          });
          if (inTown) appState.mapScaleMode = 'town';
        }
      }
      renderFullscreenMap();
    });

    byId('oi-map-zoom-out').addEventListener('click', function () {
      if (appState.mapScaleMode === 'town') {
        if (appState.townCam.zoom > 1.05) {
          appState.townCam.zoom = Math.max(0.85, appState.townCam.zoom / 1.35);
        } else {
          appState.mapScaleMode = 'island';
          appState.cam.zoom = 1.6;
        }
      } else {
        appState.cam.zoom = Math.max(0.85, appState.cam.zoom / 1.35);
      }
      renderFullscreenMap();
    });

    byId('oi-map-locate').addEventListener('click', function () {
      appState.townCam.cx = 318;
      appState.townCam.cy = 425;
      appState.townCam.zoom = 1.0;
      refreshMapData(true);
    });

    var mapCanvas = byId('oi-map-canvas');
    mapCanvas.addEventListener('pointerdown', function (ev) {
      if (ev.target.closest('[data-map-node]')) return;
      if (appState.mapScaleMode === 'dungeon') return;
      appState.cam.dragging = true;
      appState.cam.lastX = ev.clientX;
      appState.cam.lastY = ev.clientY;
      mapCanvas.setPointerCapture(ev.pointerId);
    });
    mapCanvas.addEventListener('pointermove', function (ev) {
      if (!appState.cam.dragging) return;
      var dx = ev.clientX - appState.cam.lastX;
      var dy = ev.clientY - appState.cam.lastY;
      appState.cam.lastX = ev.clientX;
      appState.cam.lastY = ev.clientY;
      if (appState.mapScaleMode === 'town') {
        var tf = 1.15 / Math.max(0.85, appState.townCam.zoom);
        appState.townCam.cx -= dx * tf;
        appState.townCam.cy -= dy * tf;
      } else if (appState.mapScaleMode === 'island') {
        var factor = 0.085 / Math.max(0.85, appState.cam.zoom);
        appState.cam.cx -= dx * factor;
        appState.cam.cy -= dy * factor;
      }
      renderFullscreenMap();
    });
    mapCanvas.addEventListener('pointerup', function () {
      appState.cam.dragging = false;
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
