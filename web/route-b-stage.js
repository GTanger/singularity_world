/**
 * 《奇點世界》Route B 主舞台控制器（敘事＋當面過招＋空間節點拓撲＋直連出入口）
 *
 * 設計重點：
 * 1. 敘事與當面過招為主舞台，人物卡片直接帶出後端真實戲劇鉤子（熟客、欠單、債主、飢乏等）。
 * 2. 桌機右欄繪製即時「空間節點拓撲 SVG」＋「直連出入口」＋「近處街坊風聞（點擊即作談資）」。
 * 3. 手機下方直接呈現俐落好按的「可前往空間出入口選項」，多跳空間拓撲收進「拓撲圖／全圖」彈窗。
 */
(function () {
  'use strict';

  var KIND_META = {
    street: { stroke: '#366f73', fill: '#fffdf8', text: '#245256', label: '街道' },
    shop: { stroke: '#a35224', fill: '#fffdf8', text: '#8f4a21', label: '店面' },
    residence: { stroke: '#7a5a78', fill: '#faf7fa', text: '#5c415a', label: '住宅' },
    forest: { stroke: '#2f725b', fill: '#f5fbf8', text: '#225745', label: '林區' },
    water: { stroke: '#2c6d7a', fill: '#eef7f8', text: '#1e505a', label: '水脈' }
  };

  var state = {
    roomId: '',
    roomName: '',
    desc: '',
    objects: [],
    npcs: [],
    spatialGraph: null,
    activeNpcId: '',
    transcripts: {},       // { [npcId]: Array<{ role: 'narration'|'player'|'npc'|'witness'|'pending', speaker?: string, text: string }> }
    choicesByNpc: {},      // { [npcId]: Array<{ action: string, meta: string, label: string }> }
    pendingNpcId: '',
    lastExitsKey: ''
  };

  function esc(s) {
    return String(s == null ? '' : s)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function el(id) {
    return document.getElementById(id);
  }

  function sendWs(payload) {
    if (typeof window.gameSend === 'function') {
      window.gameSend(payload);
    }
  }

  function normalizeNpcList(rawList) {
    if (!Array.isArray(rawList)) return [];
    return rawList.map(function (n) {
      var id = n.npc_id || n.id || '';
      var name = n.display_name || n.name || '街坊';
      var act = n.current_activity || n.behavior_text || '';
      return {
        id: id,
        display_name: name,
        card_state: n.card_state || 'stranger',
        current_activity: act,
        hooks: Array.isArray(n.hooks) ? n.hooks : [],
        dossier: n.dossier || null
      };
    }).filter(function (n) { return !!n.id; });
  }

  function shortActivity(act) {
    if (!act) return '';
    var s = String(act).trim();
    if (s.indexOf('熱食') >= 0 || s.indexOf('溫湯') >= 0 || s.indexOf('充飢') >= 0) return '端湯充飢';
    if (s.indexOf('鍛') >= 0 || s.indexOf('刃') >= 0 || s.indexOf('修繕') >= 0) return '爐畔校刃';
    if (s.indexOf('帳') >= 0 || s.indexOf('債') >= 0) return '結算舊帳';
    if (s.indexOf('竹') >= 0 || s.indexOf('筍') >= 0 || s.indexOf('採') >= 0) return '入林起貨';
    if (s.indexOf('歇') >= 0 || s.indexOf('休') >= 0) return '歇腳喘息';
    if (s.indexOf('賣') >= 0 || s.indexOf('換') >= 0 || s.indexOf('行市') >= 0) return '街頭議價';
    if (s.length <= 6) return s;
    return s.slice(0, 6);
  }

  function parseChoices(rawChoices) {
    if (!Array.isArray(rawChoices) || !rawChoices.length) return [];
    return rawChoices.map(function (entry) {
      var parts = String(entry || '').split('|');
      if (parts.length >= 3) {
        return { action: parts[0].trim(), meta: parts[1].trim(), label: parts.slice(2).join('|').trim() };
      }
      return { action: 'Talk', meta: '過招', label: String(entry || '').trim() };
    }).filter(function (c) { return !!c.label; });
  }

  function defaultChoicesForNpc(npc) {
    var privacy = (state.spatialGraph && state.spatialGraph.current && state.spatialGraph.current.privacy) || 'public';
    var hasDebt = (npc && npc.hooks || []).some(function (h) { return h.label && h.label.indexOf('欠單') >= 0; });
    var isHungry = (npc && npc.hooks || []).some(function (h) { return h.label && h.label.indexOf('飢') >= 0; });
    return [
      { action: 'SocialBargain', meta: '易物議價', label: '「我手頭有鮮貨，先開個熟客價來聽聽」' },
      {
        action: 'SocialTreat',
        meta: isHungry ? '請食套話' : '請茶套話',
        label: isHungry ? '「熱湯算我的，坐下來把這條街的底細細說來聽」' : '「請你喝盞涼茶，近來街坊間可有什麼暗流？」'
      },
      hasDebt
        ? { action: 'SocialSettleDebt', meta: '周旋清帳', label: '「聽說你腰間壓著一筆舊帳？說不定我能替你周旋」' }
        : { action: 'SocialDebtInquiry', meta: '打聽債權', label: '「這條街上最近誰手頭緊、欠了哪家的帳？」' },
      privacy === 'public'
        ? { action: 'SocialPrivateTalk', meta: '避人私談', label: '「街面耳目雜，進內室避開旁人再細談」' }
        : { action: 'Talk', meta: '趁靜問底', label: '「這會兒沒外人旁聽，你把真正顧忌的事直說吧」' }
    ];
  }

  function getActiveNpc() {
    if (!state.activeNpcId) return null;
    for (var i = 0; i < state.npcs.length; i++) {
      if (state.npcs[i].id === state.activeNpcId) return state.npcs[i];
    }
    return null;
  }

  function appendTurn(npcId, turn) {
    if (!npcId) return;
    if (!state.transcripts[npcId]) state.transcripts[npcId] = [];
    // 移除暫態「正在思索」列
    state.transcripts[npcId] = state.transcripts[npcId].filter(function (t) { return t.role !== 'pending'; });
    state.transcripts[npcId].push(turn);
    if (state.transcripts[npcId].length > 24) {
      state.transcripts[npcId] = state.transcripts[npcId].slice(-24);
    }
  }

  function renderHeaderAndScene() {
    var sg = state.spatialGraph;
    var cur = sg && sg.current;
    var nameEl = el('rb-place-name');
    var subEl = el('rb-place-sub');
    var privEl = el('rb-privacy-pill');
    var proseEl = el('rb-scene-prose');
    var objBar = el('rb-objects-bar');

    var displayPlace = (cur && cur.name) || state.roomName || '西麓街鎮';
    var exitCount = (sg && Array.isArray(sg.exits)) ? sg.exits.length : 0;
    if (nameEl) nameEl.textContent = displayPlace;
    if (subEl) {
      var parts = [];
      if (cur && cur.region_name) parts.push(cur.region_name);
      if (cur && cur.short_sub) parts.push(cur.short_sub);
      if (exitCount > 0) parts.push(exitCount + ' 處出入口');
      subEl.textContent = parts.join('・');
    }
    if (privEl) {
      var pText = (cur && cur.privacy_zh) || '公開街面・旁人可聽見';
      privEl.textContent = pText;
      privEl.className = 'rb-privacy-pill' +
        (cur && cur.privacy === 'private' ? ' is-private' : (cur && cur.privacy === 'semi_private' ? ' is-semi' : ''));
    }

    // 同步頂欄右方地名
    var topPlace = el('sw-top-phase-place');
    if (topPlace && displayPlace) {
      var curTop = topPlace.textContent || '';
      var sepIdx = curTop.indexOf('｜');
      if (sepIdx >= 0) {
        topPlace.textContent = curTop.slice(0, sepIdx + 1) + ' ' + displayPlace;
      }
    }

    if (proseEl) {
      var desc = (state.desc || '').trim();
      if (!desc) {
        desc = displayPlace + '在眼前展開，街聲與風意交錯。';
      }
      proseEl.textContent = desc;
    }

    if (objBar) {
      var objs = Array.isArray(state.objects) ? state.objects.slice(0, 6) : [];
      if (!objs.length) {
        objBar.hidden = true;
        objBar.innerHTML = '';
      } else {
        objBar.hidden = false;
        objBar.innerHTML = objs.map(function (o) {
          var oid = o.id || o.name || '';
          var oname = o.name || o.id || '景物';
          return '<button type="button" class="rb-obj-chip" data-rb-obj-id="' + esc(oid) + '" data-rb-obj-name="' + esc(oname) + '">' + esc(oname) + '</button>';
        }).join('');
      }
    }
  }

  function renderCastBar() {
    var bar = el('rb-cast-bar');
    if (!bar) return;
    if (!state.npcs.length) {
      bar.innerHTML = '<span class="rb-cast-empty">此處一時靜無人聲，可往相連出入口走動尋人，或查看眼前器物。</span>';
      return;
    }
    var visible = state.npcs.slice(0, 6);
    var html = visible.map(function (n) {
      var isAct = n.id === state.activeNpcId;
      var hooksHtml = (n.hooks || []).slice(0, 1).map(function (h) {
        var tone = h.tone === 'warn' ? ' warn' : (h.tone === 'warm' ? ' warm' : '');
        return '<span class="rb-p-hook' + tone + '">' + esc(h.label) + '</span>';
      }).join('');
      var actStr = shortActivity(n.current_activity);
      return '<button type="button" class="rb-person-chip' + (isAct ? ' active' : '') + '" data-rb-npc-id="' + esc(n.id) + '">' +
        '<span class="rb-p-dot"></span>' +
        '<span class="rb-p-name">' + esc(n.display_name) + '</span>' +
        hooksHtml +
        (actStr ? '<span class="rb-p-act">' + esc(actStr) + '</span>' : '') +
        '</button>';
    }).join('');
    if (state.npcs.length > 6) {
      html += '<span class="rb-person-chip" style="border-style:dashed;cursor:default;">＋ 另有 ' + (state.npcs.length - 6) + ' 位街坊</span>';
    }
    bar.innerHTML = html;
  }

  function renderDialogueStage() {
    var dossierEl = el('rb-dossier-strip');
    var transEl = el('rb-transcript');
    var choicesEl = el('rb-choices-wrap');
    if (!dossierEl || !transEl || !choicesEl) return;

    var npc = getActiveNpc();
    if (!npc) {
      dossierEl.innerHTML = '<span class="rb-dossier-who">在場動態</span>' +
        '<span class="rb-dossier-tag">點選上方任一位街坊即可當面過招</span>';
      transEl.innerHTML = state.npcs.length
        ? '<div class="rb-turn narration">眼前有 ' + state.npcs.length + ' 位街坊在此落腳。直接點選上方人名，即可端詳對方底細、當面談價、請食套話或打聽街坊舊帳。</div>'
        : '<div class="rb-turn narration">此處暫無旁人駐足。點選下方（或右欄）任一出入口即可邁步前往相連的街道、店面、住宅或林區。</div>';
      choicesEl.hidden = true;
      return;
    }

    choicesEl.hidden = false;
    var d = npc.dossier;
    var dossierHtml = '<span class="rb-dossier-who">正在當面過招：' + esc(npc.display_name) + '</span>';
    if (d) {
      if (d.edge_label) dossierHtml += '<span class="rb-dossier-tag">' + esc(d.edge_label) + '</span>';
      if (d.body_vibe) dossierHtml += '<span class="rb-dossier-tag">' + esc(d.body_vibe) + '</span>';
      if (d.vessel_summary && d.vessel_summary !== '身無長物') {
        dossierHtml += '<span class="rb-dossier-vessel">隨身：' + esc(d.vessel_summary) + '</span>';
      }
    }
    dossierHtml += '<span class="rb-dossier-actions">' +
      '<button type="button" class="rb-mini-act" data-rb-inspect="' + esc(npc.id) + '">細看端詳</button>' +
      '</span>';
    dossierEl.innerHTML = dossierHtml;

    var hist = state.transcripts[npc.id] || [];
    if (!hist.length) {
      var relHint = (d && d.relation_summary) ? '（' + d.relation_summary + '）' : '';
      transEl.innerHTML = '<div class="rb-turn narration">你走近' + esc(npc.display_name) + '身旁' + esc(relHint) + '。對方' + esc(npc.current_activity || '正忙著手頭活計') + '，不妨擇一機鋒試探，或直接開口問話。</div>';
    } else {
      transEl.innerHTML = hist.map(function (t) {
        if (t.role === 'witness') {
          return '<div class="rb-turn witness">' + esc(t.text) + '</div>';
        }
        if (t.role === 'npc') {
          var spk = t.speaker || npc.display_name || '';
          var bodyText = t.text || '';
          if (spk && bodyText.indexOf(spk) === 0) {
            bodyText = bodyText.slice(spk.length).trim();
          }
          return '<div class="rb-turn npc"><span class="rb-spk">' + esc(spk) + '</span>' + esc(bodyText) + '</div>';
        }
        if (t.role === 'player') {
          return '<div class="rb-turn player">你：「' + esc(t.text) + '」</div>';
        }
        if (t.role === 'pending') {
          return '<div class="rb-turn pending">' + esc(t.text) + '</div>';
        }
        return '<div class="rb-turn narration">' + esc(t.text) + '</div>';
      }).join('');
      requestAnimationFrame(function () {
        transEl.scrollTop = transEl.scrollHeight;
      });
    }

    var choices = (state.choicesByNpc[npc.id] && state.choicesByNpc[npc.id].length)
      ? state.choicesByNpc[npc.id]
      : defaultChoicesForNpc(npc);
    var gridEl = el('rb-choice-grid');
    if (gridEl) {
      gridEl.innerHTML = choices.slice(0, 4).map(function (c, idx) {
        return '<button type="button" class="rb-choice-btn' + (idx === 0 ? ' accent' : '') + '" ' +
          'data-rb-choice-action="' + esc(c.action) + '" data-rb-choice-label="' + esc(c.label) + '">' +
          '<span>' + esc(c.label) + '</span>' +
          '<span class="rb-choice-meta">' + esc(c.meta) + '</span>' +
          '</button>';
      }).join('');
    }

    var inp = el('rb-free-input');
    if (inp) {
      inp.placeholder = '直接輸入想對' + npc.display_name + '說的話（試探、還價、問帳或放風聲）……';
    }
  }

  function renderExitsButtons(targetEl, exits) {
    if (!targetEl) return;
    if (!Array.isArray(exits) || !exits.length) {
      targetEl.innerHTML = '<span class="rb-exits-lbl">暫無直連出入口</span>';
      return;
    }
    targetEl.innerHTML = exits.map(function (ex) {
      var kcls = 'kind-' + (ex.target_kind || 'street');
      var badge = ex.live_badge ? '<span class="rb-exit-badge">' + esc(ex.live_badge) + '</span>' : '';
      var popDot = ex.pop_count > 0 ? '<span style="color:#c9463b;font-size:10px;">●</span>' : '';
      return '<button type="button" class="rb-exit-pill ' + kcls + '" ' +
        'data-rb-exit-dir="' + esc(ex.direction) + '" ' +
        'data-rb-exit-gx="' + esc(ex.target_gx) + '" ' +
        'data-rb-exit-gy="' + esc(ex.target_gy) + '">' +
        '<span class="rb-exit-kind">' + esc(ex.target_kind_zh || '處') + '</span>' +
        '<span class="rb-exit-dir">' + esc(ex.dir_label) + '</span>' +
        '<span>' + esc(ex.target_name) + '</span>' +
        popDot + badge +
        '</button>';
    }).join('');
  }

  function isNightMode() {
    var app = el('app-grid');
    if (app && app.getAttribute('data-tide') === 'night') return true;
    if (document.body && document.body.getAttribute('data-tide') === 'night') return true;
    var bg = app ? getComputedStyle(app).backgroundColor : '';
    if (bg) {
      var m = bg.match(/\d+/g);
      if (m && m.length >= 3) {
        var lum = (Number(m[0]) * 299 + Number(m[1]) * 587 + Number(m[2]) * 114) / 1000;
        if (lum < 90) return true;
      }
    }
    return false;
  }

  function buildTopologySvg(sg) {
    var night = isNightMode();
    if (!sg || !Array.isArray(sg.nodes) || !sg.nodes.length) {
      return '<svg viewBox="0 0 460 310" xmlns="http://www.w3.org/2000/svg"><text x="230" y="155" text-anchor="middle" font-size="13" fill="' + (night ? '#9aa3b5' : '#726a5d') + '">正在載入空間節點拓撲……</text></svg>';
    }
    var W = 460;
    var H = 310;
    var cx = W / 2;
    var cy = H / 2;
    var scaleX = 104;
    var scaleY = 66;

    var posMap = {};
    var regionBoxes = {};
    sg.nodes.forEach(function (n) {
      var x = Math.max(54, Math.min(W - 54, cx + (n.rx || 0) * scaleX));
      var y = Math.max(34, Math.min(H - 34, cy + (n.ry || 0) * scaleY));
      posMap[n.id] = { x: x, y: y, node: n };
      var rid = n.region_id || 'other';
      if (!regionBoxes[rid]) {
        regionBoxes[rid] = { name: n.region_name || '', minX: x, maxX: x, minY: y, maxY: y, count: 0 };
      }
      var rb = regionBoxes[rid];
      rb.minX = Math.min(rb.minX, x);
      rb.maxX = Math.max(rb.maxX, x);
      rb.minY = Math.min(rb.minY, y);
      rb.maxY = Math.max(rb.maxY, y);
      rb.count += 1;
    });

    var svg = '<svg viewBox="0 0 ' + W + ' ' + H + '" xmlns="http://www.w3.org/2000/svg">';

    // 區域群組虛線輪廓（例：刺竹林區多節點相連）
    Object.keys(regionBoxes).forEach(function (rid) {
      var rb = regionBoxes[rid];
      if (rb.count < 2) return;
      var padX = 52;
      var padY = 27;
      var rx = Math.max(6, rb.minX - padX);
      var ry = Math.max(6, rb.minY - padY - 8);
      var rw = Math.min(W - 12 - rx, (rb.maxX - rb.minX) + padX * 2);
      var rh = Math.min(H - 12 - ry, (rb.maxY - rb.minY) + padY * 2 + 8);
      var isForest = rid === 'zhulin_grove';
      var isWater = rid === 'spring_bank';
      var fill = isForest ? 'rgba(47,114,91,0.065)' : (isWater ? 'rgba(44,109,122,0.06)' : 'rgba(43,40,35,0.02)');
      var stroke = isForest ? 'rgba(47,114,91,0.34)' : (isWater ? 'rgba(44,109,122,0.34)' : 'rgba(43,40,35,0.14)');
      var lblColor = isForest ? '#2b6451' : (isWater ? '#265c67' : '#776e60');
      svg += '<rect x="' + rx.toFixed(1) + '" y="' + ry.toFixed(1) + '" width="' + rw.toFixed(1) + '" height="' + rh.toFixed(1) + '" rx="13" fill="' + fill + '" stroke="' + stroke + '" stroke-width="1.1" stroke-dasharray="4 3"/>';
      if (rb.name) {
        svg += '<text x="' + (rx + 9).toFixed(1) + '" y="' + (ry + 12).toFixed(1) + '" font-size="9.5" fill="' + lblColor + '" font-weight="600">' + esc(rb.name) + '</text>';
      }
    });

    // 出入口連線
    (sg.edges || []).forEach(function (e) {
      var a = posMap[e.from_id];
      var b = posMap[e.to_id];
      if (!a || !b) return;
      var stroke = night
        ? (e.is_direct ? '#b9b0a1' : '#5e6678')
        : (e.is_direct ? '#5a4f43' : '#9a9183');
      var sw = e.is_direct ? '2.0' : '1.25';
      var dash = e.dashed ? ' stroke-dasharray="3.5 2.5"' : '';
      svg += '<line x1="' + a.x.toFixed(1) + '" y1="' + a.y.toFixed(1) + '" x2="' + b.x.toFixed(1) + '" y2="' + b.y.toFixed(1) + '" stroke="' + stroke + '" stroke-width="' + sw + '" stroke-linecap="round"' + dash + '/>';
    });

    // 空間節點
    sg.nodes.forEach(function (n) {
      var p = posMap[n.id];
      if (!p) return;
      var km = KIND_META[n.kind] || KIND_META.street;
      var bw = n.is_current ? 96 : (n.is_direct_exit ? 86 : 78);
      var bh = n.is_current ? 42 : 34;
      var x0 = p.x - bw / 2;
      var y0 = p.y - bh / 2;
      var fill = night
        ? (n.is_current ? '#3a2326' : '#222938')
        : (n.is_current ? '#f9ebe7' : km.fill);
      var stroke = night
        ? (n.is_current ? '#e06b5c' : (n.kind === 'shop' ? '#d9935b' : (n.kind === 'forest' ? '#6dc4a2' : (n.kind === 'residence' ? '#c59bc4' : '#6cc2c8'))))
        : (n.is_current ? '#a43a2e' : km.stroke);
      var sw = n.is_current ? '2.1' : (n.is_direct_exit ? '1.55' : '1.15');
      var dash = (n.kind === 'residence' || n.needs_knock) ? ' stroke-dasharray="3 2"' : '';
      var titleColor = night
        ? (n.is_current ? '#ffd8d2' : '#ece6d9')
        : (n.is_current ? '#7d2218' : '#221f1a');
      var subColor = night
        ? (n.is_current ? '#f0a296' : '#9fb0c2')
        : (n.is_current ? '#8f3126' : km.text);
      var subText = n.is_current ? ('你在此・' + (sg.exits ? sg.exits.length : 0) + '出口') : (n.kind_zh + (n.short_sub ? '・' + n.short_sub.slice(0, 2) : ''));

      svg += '<g class="rb-svg-node" transform="translate(' + x0.toFixed(1) + ',' + y0.toFixed(1) + ')" ' +
        'data-rb-node-current="' + (n.is_current ? '1' : '0') + '" ' +
        'data-rb-node-direct="' + (n.is_direct_exit ? '1' : '0') + '" ' +
        'data-rb-node-dir="' + esc(n.direction || '') + '" ' +
        'data-rb-node-gx="' + esc(n.gx) + '" data-rb-node-gy="' + esc(n.gy) + '">';
      svg += '<rect class="rb-node-box" width="' + bw + '" height="' + bh + '" rx="8" fill="' + fill + '" stroke="' + stroke + '" stroke-width="' + sw + '"' + dash + '/>';
      if (n.pop_count > 0 && !n.is_current) {
        svg += '<circle cx="' + (bw - 8) + '" cy="8" r="2.8" fill="#e06b5c"/>';
      }
      if (n.is_current) {
        svg += '<rect x="' + ((bw - 42) / 2).toFixed(1) + '" y="-8" width="42" height="13" rx="6.5" fill="' + (night ? '#c85647' : '#a43a2e') + '"/>';
        svg += '<text x="' + (bw / 2).toFixed(1) + '" y="1.5" text-anchor="middle" font-size="8.5" fill="#fffaf2" font-weight="700">你在此</text>';
      } else if (n.live_badge) {
        svg += '<rect x="' + ((bw - 42) / 2).toFixed(1) + '" y="-8" width="42" height="13" rx="6.5" fill="' + (night ? '#c85647' : '#a43a2e') + '"/>';
        svg += '<text x="' + (bw / 2).toFixed(1) + '" y="1.5" text-anchor="middle" font-size="8" fill="#fffaf2" font-weight="600">' + esc(n.live_badge) + '</text>';
      }
      var labelName = n.name.length > 6 ? n.name.slice(0, 6) : n.name;
      svg += '<text x="' + (bw / 2).toFixed(1) + '" y="' + (n.is_current ? 20 : 15) + '" text-anchor="middle" font-size="' + (n.is_current ? 11.5 : 10.5) + '" font-weight="700" fill="' + titleColor + '">' + esc(labelName) + '</text>';
      svg += '<text x="' + (bw / 2).toFixed(1) + '" y="' + (n.is_current ? 33 : 27) + '" text-anchor="middle" font-size="8" fill="' + subColor + '">' + esc(subText) + '</text>';
      svg += '</g>';
    });

    svg += '</svg>';
    return svg;
  }

  function renderSpatialAndEchoes() {
    var sg = state.spatialGraph;
    var svgHtml = buildTopologySvg(sg);
    var desktopSvgWrap = el('rb-topo-svg-wrap');
    if (desktopSvgWrap) desktopSvgWrap.innerHTML = svgHtml;
    var modalSvgWrap = el('rb-topo-modal-svg');
    if (modalSvgWrap) modalSvgWrap.innerHTML = svgHtml;

    var exits = (sg && sg.exits) || [];
    renderExitsButtons(el('rb-desktop-exits'), exits);
    renderExitsButtons(el('rb-mobile-exits-list'), exits);
    renderExitsButtons(el('rb-modal-exits'), exits);

    var echoBox = el('rb-echo-box');
    if (echoBox) {
      var echoes = (sg && Array.isArray(sg.echoes)) ? sg.echoes : [];
      if (!echoes.length) {
        echoBox.innerHTML = '<div class="rb-echo-item"><span class="rb-echo-loc">西麓街市 ・ 靜候風聲</span>街面上暫無新的風聞傳出，走進店面或與街坊請茶套話常能聽見真內幕。</div>';
      } else {
        echoBox.innerHTML = echoes.map(function (ec) {
          return '<button type="button" class="rb-echo-item' + (ec.hot ? ' hot' : '') + '" data-rb-echo-text="' + esc(ec.summary) + '" data-rb-echo-loc="' + esc(ec.space_name) + '">' +
            '<span class="rb-echo-loc">' + esc(ec.space_name) + ' ・ 街坊風聞</span>' +
            esc(ec.summary) +
            '</button>';
        }).join('');
      }
    }
  }

  function selectNpc(npcId, autoInspect) {
    state.activeNpcId = npcId;
    renderCastBar();
    renderDialogueStage();
    if (autoInspect && npcId && (!state.transcripts[npcId] || !state.transcripts[npcId].length)) {
      sendWs({ type: 'do_action', entity_id: npcId, action: 'Look', player_input: '' });
    }
  }

  function triggerNpcAction(action, playerInput, displayPlayerLine) {
    var npc = getActiveNpc();
    if (!npc) return;
    if (displayPlayerLine) {
      appendTurn(npc.id, { role: 'player', text: displayPlayerLine });
    }
    state.pendingNpcId = npc.id;
    appendTurn(npc.id, { role: 'pending', text: npc.display_name + '正在聽你說話……' });
    renderDialogueStage();
    sendWs({
      type: 'do_action',
      entity_id: npc.id,
      action: action || 'Talk',
      player_input: playerInput || ''
    });
  }

  function openTopologyModal() {
    var bd = el('rb-topo-modal-backdrop');
    if (!bd) return;
    bd.hidden = false;
    renderSpatialAndEchoes();
  }

  function closeTopologyModal() {
    var bd = el('rb-topo-modal-backdrop');
    if (bd) bd.hidden = true;
  }

  function bindEvents() {
    var root = el('sw-route-b-main');
    if (!root || root.dataset.rbBound === '1') return;
    root.dataset.rbBound = '1';

    document.body.classList.add('sw-route-b-active');

    document.addEventListener('click', function (ev) {
      var t = ev.target;
      if (!t || !t.closest) return;

      // 1. 點選人物卡
      var pChip = t.closest('[data-rb-npc-id]');
      if (pChip) {
        selectNpc(pChip.getAttribute('data-rb-npc-id'), true);
        return;
      }

      // 2. 細看端詳鈕
      var insBtn = t.closest('[data-rb-inspect]');
      if (insBtn) {
        var tid = insBtn.getAttribute('data-rb-inspect');
        if (tid) sendWs({ type: 'do_action', entity_id: tid, action: 'Look', player_input: '' });
        return;
      }

      // 3. 機鋒選項
      var chBtn = t.closest('[data-rb-choice-action]');
      if (chBtn) {
        var act = chBtn.getAttribute('data-rb-choice-action') || 'Talk';
        var lbl = (chBtn.getAttribute('data-rb-choice-label') || '').replace(/^[「『]+|[」』]+$/g, '');
        triggerNpcAction(act, lbl, lbl);
        return;
      }

      // 4. 直連出入口按鈕
      var exBtn = t.closest('[data-rb-exit-dir]');
      if (exBtn) {
        var dir = exBtn.getAttribute('data-rb-exit-dir');
        if (dir) {
          sendWs({ type: 'move', direction: dir });
          closeTopologyModal();
        }
        return;
      }

      // 5. SVG 拓撲圖節點點擊
      var svgNode = t.closest('.rb-svg-node');
      if (svgNode) {
        if (svgNode.getAttribute('data-rb-node-current') === '1') return;
        var isDirect = svgNode.getAttribute('data-rb-node-direct') === '1';
        var ndir = svgNode.getAttribute('data-rb-node-dir');
        if (isDirect && ndir) {
          sendWs({ type: 'move', direction: ndir });
        } else {
          var gx = parseInt(svgNode.getAttribute('data-rb-node-gx'), 10);
          var gy = parseInt(svgNode.getAttribute('data-rb-node-gy'), 10);
          if (!isNaN(gx) && !isNaN(gy)) {
            sendWs({ type: 'move', x: gx, y: gy, direction: '行旅' });
          }
        }
        closeTopologyModal();
        return;
      }

      // 6. 點擊場景物件
      var objBtn = t.closest('[data-rb-obj-id]');
      if (objBtn) {
        var oid = objBtn.getAttribute('data-rb-obj-id');
        if (oid) {
          sendWs({ type: 'do_action', action: 'Look', entity_id: oid, target_id: oid, object_id: oid });
        }
        return;
      }

      // 7. 點擊街坊風聞預填為對質談資
      var echoBtn = t.closest('[data-rb-echo-text]');
      if (echoBtn) {
        var loc = echoBtn.getAttribute('data-rb-echo-loc') || '街上';
        var txt = echoBtn.getAttribute('data-rb-echo-text') || '';
        var inp = el('rb-free-input');
        if (inp) {
          inp.value = '聽說剛才' + loc + '那邊——' + txt + '，這事你知道多少？';
          inp.focus();
        }
        return;
      }

      // 8. 展開／關閉拓撲圖彈窗
      if (t.closest('#rb-btn-open-topo') || t.closest('#btn-toggle-map')) {
        ev.preventDefault();
        ev.stopPropagation();
        openTopologyModal();
        return;
      }
      if (t.closest('#rb-topo-modal-close') || t === el('rb-topo-modal-backdrop')) {
        closeTopologyModal();
      }
    }, true);

    var sendBtn = el('rb-send-btn');
    var freeInp = el('rb-free-input');
    if (sendBtn && freeInp) {
      var submitFree = function () {
        var val = (freeInp.value || '').trim();
        if (!val) return;
        freeInp.value = '';
        triggerNpcAction('Talk', val, val);
      };
      sendBtn.addEventListener('click', submitFree);
      freeInp.addEventListener('keydown', function (e) {
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          submitFree();
        }
      });
    }
  }

  window.RouteBStage = {
    init: bindEvents,

    onRoomEnter: function (msg) {
      bindEvents();
      var prevRoom = state.roomId;
      state.roomId = msg.room_id || state.roomId;
      state.roomName = (msg.place && msg.place.name) || (msg.terrain && msg.terrain.name) || msg.room_name || state.roomName;
      state.desc = msg.desc || state.desc;
      state.objects = Array.isArray(msg.objects) ? msg.objects : [];
      state.npcs = normalizeNpcList(msg.npcs || msg.entities || []);
      if (msg.spatial_graph) {
        state.spatialGraph = msg.spatial_graph;
      }

      // 換空間或原本人物離開時，自動選中第一位在場人物（若無則清空）
      var stillHere = state.npcs.some(function (n) { return n.id === state.activeNpcId; });
      if (!stillHere || (prevRoom && prevRoom !== state.roomId)) {
        state.activeNpcId = state.npcs.length ? state.npcs[0].id : '';
      }

      renderHeaderAndScene();
      renderCastBar();
      renderDialogueStage();
      renderSpatialAndEchoes();
      // 監看 data-tide 日夜切換時重繪拓撲配色
      var appEl = el('app-grid');
      if (appEl && !appEl.dataset.rbTideWatch && typeof MutationObserver === 'function') {
        appEl.dataset.rbTideWatch = '1';
        new MutationObserver(function () { renderSpatialAndEchoes(); })
          .observe(appEl, { attributes: true, attributeFilter: ['data-tide'] });
      }
      setTimeout(renderSpatialAndEchoes, 150);
    },

    onActionResult: function (msg) {
      if (!msg) return;
      var tid = msg.target_id || '';
      var isNpc = tid.indexOf('zl_res_') === 0 ||
        state.npcs.some(function (n) { return n.id === tid; });
      if (!isNpc) return;

      // 更新顯示真名
      if (msg.target_name) {
        for (var i = 0; i < state.npcs.length; i++) {
          if (state.npcs[i].id === tid) {
            state.npcs[i].display_name = msg.target_name;
            state.npcs[i].card_state = 'named';
            break;
          }
        }
      }

      if (Array.isArray(msg.tactical_choices) && msg.tactical_choices.length) {
        state.choicesByNpc[tid] = parseChoices(msg.tactical_choices);
      }

      var rawText = String(msg.narrative || '').trim();
      if (rawText) {
        var lines = rawText.split('\n').map(function (x) { return x.trim(); }).filter(Boolean);
        lines.forEach(function (ln) {
          if (ln.indexOf('〔街坊目擊〕') === 0) {
            appendTurn(tid, { role: 'witness', text: ln });
          } else if (msg.action === 'Look') {
            appendTurn(tid, { role: 'narration', text: ln });
          } else {
            appendTurn(tid, { role: 'npc', speaker: msg.target_name || '', text: ln });
          }
        });
      }

      if (msg.suggested_action === 'enter_indoor') {
        appendTurn(tid, { role: 'witness', text: '〔提示〕你可直接點選「進內室」出入口，步入裡間避開街面耳目。' });
      }

      if (state.activeNpcId !== tid) {
        state.activeNpcId = tid;
      }
      renderCastBar();
      renderDialogueStage();
    }
  };
})();
