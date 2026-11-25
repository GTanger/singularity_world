/* 奇點世界｜銀鷗島實況全開大地圖・側欄名冊、落實度稽核與場所透視面板（< 950 行模組拆分） */
(function () {
  'use strict';

  function esc(s) {
    return String(s == null ? '' : s)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function pad2(n) { return n < 10 ? ('0' + n) : String(n); }
  function byId(id) { return document.getElementById(id); }

  window.WorldAtlasPanels = {
    renderHeader: function (state) {
      if (!state.snap) return;
      var clk = state.snap.clock;
      byId('waClock').textContent =
        clk.calendar + ' ｜ ' + clk.bell + ' ' + pad2(clk.hour) + ':' + pad2(clk.minute) +
        '（' + clk.tide_name + '・' + clk.weather + '）';

      var lg = state.snap.ledger;
      var pill = byId('waLedgerPill');
      pill.textContent = '總銅幣 ' + lg.genesis_total_copper.toLocaleString() + ' 守恆｜差額 ' + lg.conservation_diff;
      pill.classList.toggle('warn', lg.conservation_diff !== 0);
    },

    renderResidentsTab: function (state) {
      if (!state.snap) return;
      var q = state.resQuery.trim().toLowerCase();
      var list = (state.snap.residents || []).filter(function (r) {
        if (state.resTown === 'rural') {
          if (String(r.town_slug || '').indexOf('burg_') !== 0) return false;
        } else if (state.resTown !== 'all' && r.town_slug !== state.resTown) {
          return false;
        }
        if (state.resAct === 'moving' && r.activity_bucket !== 'moving') return false;
        if (state.resAct === 'at_work' && r.activity_bucket !== 'at_work') return false;
        if (state.resAct === 'wild' && r.activity_bucket !== 'wilderness' && r.activity_bucket !== 'pit_or_combat') return false;
        if (state.resAct === 'concern' && !r.top_concern_zh) return false;
        if (q) {
          var hay = (r.name + ' ' + r.role + ' ' + r.race + ' ' + r.current_place_zh + ' ' + r.intention_zh + ' ' + r.top_concern_zh).toLowerCase();
          if (hay.indexOf(q) === -1) return false;
        }
        return true;
      });

      var html = '<div class="wa-sub">符合條件共 <b>' + list.length + '</b> 名常住居民（點選即定位至當前所在場所）</div>';
      list.slice(0, 220).forEach(function (r) {
        var sel = r.actor_id === state.selectedActorId ? ' selected' : '';
        html +=
          '<div class="wa-res-item' + sel + '" data-pick-actor="' + esc(r.actor_id) + '" data-pick-node="' + esc(r.current_node_id) + '" data-pick-cell="' + r.cell_id + '">' +
          '<div class="wa-res-top">' +
          '<span class="wa-res-name">' + esc(r.name) + '</span>' +
          '<span class="wa-res-meta">' + esc(r.race) + '・' + r.age + '歲・' + esc(r.role) + '｜' + r.copper + '銅・糧' + r.larder_rations + '</span>' +
          '</div>' +
          '<div class="wa-res-loc">📍 ' + esc(r.current_place_zh) + (r.route_steps > 0 ? ('（行進中・剩 ' + r.route_steps + ' 步）') : '') + '</div>' +
          '<div class="wa-res-intent">▸ ' + esc(r.intention_zh) + '</div>' +
          (r.top_concern_zh ? ('<div class="wa-res-concern">💭 ' + esc(r.top_concern_zh) + '</div>') : '') +
          '</div>';
      });
      byId('resList').innerHTML = html;
    },

    renderPulseTab: function (state) {
      if (!state.snap) return;
      var p = state.snap.autonomy_pulse;
      var c = p.counts;
      var html =
        '<div class="wa-stat-grid">' +
        '<div class="wa-stat-box">在崗營生<b>' + c.at_work + ' 人</b></div>' +
        '<div class="wa-stat-box">街巷趕路中<b>' + c.moving + ' 人</b></div>' +
        '<div class="wa-stat-box">居家／宿房歇息<b>' + c.home_rest + ' 人</b></div>' +
        '<div class="wa-stat-box">野外採集／探窟<b>' + (c.wilderness + c.pit_or_combat) + ' 人</b></div>' +
        '<div class="wa-stat-box">心懷未解掛念<b>' + c.with_concern + ' 人</b></div>' +
        '<div class="wa-stat-box">有效契約／賒帳<b>' + c.active_contracts + ' 筆</b></div>' +
        '</div>';

      html += '<div class="wa-card accent-jade"><div class="wa-card-title"><span>全島最新自主生活與交易流（' + (p.recent_feed || []).length + ' 則）</span></div>';
      if (!(p.recent_feed || []).length) {
        html += '<div class="wa-kv-row">尚無近期事件紀錄</div>';
      } else {
        p.recent_feed.forEach(function (line) {
          html += '<div class="wa-kv-row">・' + esc(line) + '</div>';
        });
      }
      html += '</div>';

      html += '<div class="wa-card accent-main"><div class="wa-card-title"><span>生效中之託管請託契與民間賒帳（' + (p.contracts || []).length + ' 筆）</span></div>';
      if (!(p.contracts || []).length) {
        html += '<div class="wa-kv-row">目前全島無未結契約或賒帳</div>';
      } else {
        p.contracts.forEach(function (ct) {
          html += '<div class="wa-kv-row">・<b>[' + esc(ct.status) + ']</b> ' + esc(ct.creditor_name) + ' ↔ ' + esc(ct.debtor_name) + '（' + ct.amount_copper + '銅｜' + esc(ct.note_zh) + '）</div>';
        });
      }
      html += '</div>';

      if ((p.notable_vessels || []).length) {
        html += '<div class="wa-card"><div class="wa-card-title"><span>熟練養器／符紋兵刃持有紀錄</span></div>';
        p.notable_vessels.slice(0, 12).forEach(function (v) {
          html += '<div class="wa-kv-row">・<button type="button" class="wa-mini-btn" data-jump-actor="' + esc(v.owner_id) + '">' + esc(v.owner_name) + '</button>【' + esc(v.weapon_name) + '】熟練 ' + v.grip_familiarity + (v.rune_mark ? ('・銘刻：' + esc(v.rune_mark)) : '') + '</div>';
        });
        html += '</div>';
      }

      byId('pulseBody').innerHTML = html;
    },

    renderAuditTab: function (state) {
      if (!state.snap) return;
      var au = state.snap.audit_honesty;
      var lg = state.snap.ledger;
      var html = '<div class="wa-card accent-jade"><div class="wa-card-title"><span>✅ 已落實並線上運作中之系統</span></div>';
      (au.implemented_summary || []).forEach(function (s) {
        html += '<div class="wa-kv-row">・' + esc(s) + '</div>';
      });
      html += '</div>';

      html += '<div class="wa-card accent-gold"><div class="wa-card-title"><span>⚠️ 尚未落實／待升級缺口（誠實盤點零粉飾）</span></div>';
      (au.unimplemented_gaps || []).forEach(function (g) {
        html += '<div class="wa-kv-row" style="margin-bottom:6px;"><strong>' + esc(g.title) + '：</strong>' + esc(g.detail) + '</div>';
      });
      html += '</div>';

      html +=
        '<div class="wa-card"><div class="wa-card-title"><span>💰 全島複式簿記銅幣守恆明細</span></div>' +
        '<div class="wa-kv-row">創世鎖定總額：<b>' + lg.genesis_total_copper.toLocaleString() + '</b> 銅</div>' +
        '<div class="wa-kv-row">全島常住居民（' + (state.snap.residents || []).length + ' 人）合計：<b>' + lg.resident_copper_sum.toLocaleString() + '</b> 銅</div>' +
        '<div class="wa-kv-row">旅人角色合計：<b>' + lg.player_copper_sum.toLocaleString() + '</b> 銅</div>' +
        '<div class="wa-kv-row">市政港稅池：<b>' + lg.municipal_treasury_copper.toLocaleString() + '</b> 銅</div>' +
        '<div class="wa-kv-row">銀號準備金：<b>' + lg.silver_bank_reserve_copper.toLocaleString() + '</b> 銅</div>' +
        '<div class="wa-kv-row">銀號契約託管池：<b>' + lg.contract_escrow_copper.toLocaleString() + '</b> 銅</div>' +
        '<div class="wa-kv-row">跨海清算池：<b>' + lg.mainland_clearing_copper.toLocaleString() + '</b> 銅</div>' +
        '<div class="wa-kv-row">地面遺落包：<b>' + lg.dropped_loot_copper.toLocaleString() + '</b> 銅</div>' +
        '<div class="wa-kv-row"><strong>即時守恆差額：' + lg.conservation_diff + ' 銅</strong></div>' +
        '</div>';

      html += '<div class="wa-card"><div class="wa-card-title"><span>🏘 已落實之 29 座鄉野聚落與 5 處南岸封村遺址（點選定位）</span></div><div>';
      (state.snap.map.cells || []).forEach(function (c) {
        if (c.kind !== 'rural_settlement' && c.kind !== 'plague_ruin') return;
        html += '<button type="button" class="wa-mini-btn" data-focus-cell="' + c.cell_id + '">#' + c.cell_id + ' ' + esc(c.title) + '</button>';
      });
      html += '</div></div>';

      byId('auditBody').innerHTML = html;
    },

    renderInspector: function (state, townSlugForCell) {
      var d = state.nodeDetail;
      var box = byId('inspectorBody');
      if (!d) return;

      var tSlug = townSlugForCell(d.cell_id);
      var html = '';

      var pickedOcc = null;
      (d.occupants || []).forEach(function (o) {
        if (o.actor_id === state.selectedActorId) pickedOcc = o;
      });
      if (pickedOcc) {
        var ms = pickedOcc.mind || {};
        var bagTxt = (pickedOcc.backpack || []).map(function (i) { return i.name + '×' + i.count; }).join('、') || '空囊';
        var lardTxt = (pickedOcc.home_larder || []).map(function (i) { return i.name + '×' + i.count; }).join('、') || '無存糧';
        html +=
          '<div class="wa-card accent-main">' +
          '<div class="wa-card-title">' +
          '<span>👤 人物實況透視：' + esc(pickedOcc.name) + '</span>' +
          '<a class="wa-link-btn" href="/chronicle#' + encodeURIComponent(pickedOcc.actor_id) + '">📖 完整列傳</a>' +
          '</div>' +
          '<div class="wa-sub">' + esc(pickedOcc.race) + '・' + pickedOcc.age + '歲・' + esc(pickedOcc.bond) + '・' + esc(pickedOcc.role) + '</div>' +
          '<div class="wa-kv-row"><strong>隨身身家：</strong><b>' + pickedOcc.copper + '</b> 銅幣｜疲勞 ' + pickedOcc.fatigue + '・飢餓 ' + pickedOcc.hunger + (pickedOcc.wound_part ? ('・傷勢：' + esc(pickedOcc.wound_part)) : '') + '</div>' +
          '<div class="wa-kv-row"><strong>住家／當值：</strong><button type="button" class="wa-mini-btn" data-node-id="' + esc(pickedOcc.home_node) + '">🏠 ' + esc(pickedOcc.home_name) + '</button> <button type="button" class="wa-mini-btn" data-node-id="' + esc(pickedOcc.work_node) + '">⚒ ' + esc(pickedOcc.work_name) + '</button></div>' +
          '<div class="wa-kv-row"><strong>手持器物：</strong>' + esc(pickedOcc.tool_name) + '（' + esc(pickedOcc.tool_provenance) + '）</div>' +
          '<div class="wa-kv-row"><strong>隨身行囊：</strong>' + esc(bagTxt) + '｜<strong>家倉口糧：</strong>' + esc(lardTxt) + '</div>' +
          '<div class="wa-kv-row"><strong>先天脾氣：</strong>' + esc(ms.temperament_zh || '') + '</div>' +
          '<div class="wa-kv-row"><strong>眼下意圖：</strong>' + esc(ms.current_intention_zh || '') + '</div>' +
          '<div class="wa-kv-row"><strong>中程方針：</strong>' + esc(ms.active_strategy_zh || '') + '</div>' +
          ((ms.concerns || []).map(function (c) {
            return '<div class="wa-res-concern">💭 【' + esc(c.emotion_zh) + '・強度' + c.intensity + '】' + esc(c.thought_zh) + '</div>';
          }).join('')) +
          ((pickedOcc.recent_breaths || []).length
            ? ('<div class="wa-kv-row" style="margin-top:6px;"><strong>最新自主行動足跡：</strong>' +
               pickedOcc.recent_breaths.map(function (b) { return '<div style="font-size:12px;color:var(--ink-sub);margin-top:2px;">・' + esc(b) + '</div>'; }).join('') + '</div>')
            : '') +
          '</div>';
      }

      html +=
        '<div class="wa-card">' +
        '<div class="wa-h2">' + esc(d.name) + '</div>' +
        '<div class="wa-sub">' +
        '<span class="wa-tag">地塊 #' + d.cell_id + ' ' + esc(d.cell_title) + '</span>' +
        '<span class="wa-tag">' + esc(d.kind) + '</span>' +
        '<span class="wa-tag">' + (d.dungeon ? '幽暗地底' : (d.indoor ? '有窗室內' : '露天街野')) + '</span>' +
        '</div>';

      var hasCellMap = false;
      var hasDngRoomMap = false;
      if (state.snap && state.snap.map) {
        (state.snap.map.cells || []).forEach(function (c) {
          if (c.cell_id === d.cell_id && c.cell_map && c.cell_map.url) hasCellMap = true;
        });
        (state.snap.map.dungeon_rooms || []).forEach(function (r) {
          if (r.id === d.node_id && r.tile && r.tile.room_url) hasDngRoomMap = true;
        });
      }
      var isCoreIllTown = !!(tSlug && window.TOWN_ILLUSTRATED_MAPS && window.TOWN_ILLUSTRATED_MAPS[tSlug]);
      if (isCoreIllTown && state.scale === 'island') {
        html += '<div style="margin-bottom:8px;"><button type="button" class="active" data-town-jump="' + esc(tSlug) + '">🔍 進入【' + esc(d.cell_title) + '】街廓微觀輿圖</button></div>';
      }
      if (!d.dungeon && hasCellMap && state.scale !== ('cell:' + d.cell_id) && state.scale !== ('town:burg_' + d.cell_id)) {
        html += '<div style="margin-bottom:8px;"><button type="button" class="active" data-cell-map-jump="' + d.cell_id + '">🖼️ 查看【' + esc(d.cell_title) + '】單格手繪地景圖</button></div>';
      }
      if (d.dungeon && hasDngRoomMap) {
        if (state.scale !== ('dngroom:' + d.node_id)) {
          html += '<div style="margin-bottom:8px;"><button type="button" class="active" data-dngroom-jump="' + esc(d.node_id) + '">🕯️ 查看【' + esc(d.name) + '】單室細部圖</button></div>';
        } else {
          html += '<div style="margin-bottom:8px;"><button type="button" data-dngroom-jump="overview">← 返回試煉坑 6 室總覽</button></div>';
        }
      }

      if (d.canon_unbuilt_burg) {
        html +=
          '<div class="wa-card accent-gold" style="margin-top:6px;">' +
          '<strong>⚠️ 正典未落實標記：' + esc(d.canon_unbuilt_burg.name_zh) + '</strong>' +
          '<div class="wa-kv-row">正典定位為「' + esc(d.canon_unbuilt_burg.kind_zh) + '」，目前仍待後續擴充。</div>' +
          '</div>';
      }

      html += '<div class="wa-desc">' + esc(d.live_desc) + '</div>';

      if (d.vacant_backstory) {
        var vb = d.vacant_backstory;
        html +=
          '<div class="wa-card accent-gold">' +
          '<strong>🏚 待售空屋前塵台帳（太陽曆 ' + vb.solar_year + ' 年）</strong>' +
          '<div class="wa-kv-row">原屋主：' + esc(vb.former_head_zh) + '（' + esc(vb.former_race) + '・' + esc(vb.former_trade) + '）</div>' +
          '<div class="wa-kv-row">現場物證：' + esc(vb.room_trace_zh) + '</div>' +
          (vb.sold_to_resident ? ('<div class="wa-kv-row">現已由居民【' + esc(vb.sold_to_resident) + '】購得入住</div>') : '<div class="wa-kv-row">現況：市政廳 360 銅掛牌待售中</div>') +
          '</div>';
      }

      if (d.household && (d.household.members || []).length) {
        html += '<div class="wa-kv-row"><strong>戶籍常住成員：</strong>' + esc(d.household.members.join('、')) + '</div>';
      }

      html += '<div class="wa-kv-row" style="margin-top:8px;"><strong>📦 此處木箱／鋪倉實存物資：</strong></div>';
      if (!(d.stored_boxes || []).length) {
        html += '<div class="wa-kv-row" style="color:var(--ink-muted);">此處木箱目前無存放物資</div>';
      } else {
        d.stored_boxes.forEach(function (bx) {
          var itStr = (bx.items || []).map(function (i) { return i.name + ' ×' + i.count; }).join('、');
          html += '<div class="wa-kv-row">・<b>' + esc(bx.owner_name) + '</b>（' + esc(bx.owner_role) + '）：' + esc(itStr) + '</div>';
        });
      }

      if ((d.exits || []).length) {
        html += '<div class="wa-kv-row" style="margin-top:8px;"><strong>🧭 連通動線：</strong><div>';
        d.exits.forEach(function (ex) {
          html += '<button type="button" class="wa-mini-btn" data-node-id="' + esc(ex.to) + '">' + esc(ex.dir) + ' → ' + esc(ex.to_name) + '</button>';
        });
        html += '</div></div>';
      }
      html += '</div>';

      html += '<div class="wa-card accent-jade"><div class="wa-card-title"><span>👥 此刻在場常住居民（' + (d.occupants || []).length + ' 人）</span></div>';
      if (!(d.occupants || []).length) {
        html += '<div class="wa-kv-row" style="color:var(--ink-muted);">此刻無常住居民停留於此節點</div>';
      } else {
        d.occupants.forEach(function (o) {
          var ms = o.mind || {};
          html +=
            '<div class="wa-res-item' + (o.actor_id === state.selectedActorId ? ' selected' : '') + '" data-select-occ="' + esc(o.actor_id) + '">' +
            '<div class="wa-res-top">' +
            '<span class="wa-res-name">' + esc(o.name) + '</span>' +
            '<span class="wa-res-meta">' + esc(o.race) + '・' + o.age + '歲・' + esc(o.role) + '｜' + o.copper + '銅</span>' +
            '</div>' +
            '<div class="wa-res-intent">▸ ' + esc(ms.current_intention_zh || '') + '</div>' +
            '</div>';
        });
      }
      html += '</div>';

      box.innerHTML = html;
    }
  };
})();
