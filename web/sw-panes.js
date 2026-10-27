// 狀態/裝備/技能/星盤/背包 pane 渲染。自包含，內部私有 esc/fmt，外部依賴 window.gameSend。v0.20.42
(function () {
	function esc(s) {
		if (!s) return '';
		var div = document.createElement('div');
		div.textContent = s;
		return div.innerHTML;
	}
	function fmt(x) {
		if (x == null || x === '') return '—';
		var n = Number(x);
		return isNaN(n) ? '—' : Math.round(n);
	}
	function _send(payload) {
		if (window.gameSend) window.gameSend(payload);
	}
	var ITEM_NAME_MAP = {
		'harvest_herb': '野地草藥',
		'harvest_wood': '堅韌原木',
		'harvest_ore': '百工礦石',
		'harvest_fish': '鮮活河魚',
		'harvest_crystal': '溫潤碎晶',
		'herb_lingzhi': '靈芝仙草',
		'ore_black_iron': '玄鐵重礦',
		'wood_iron_oak': '鐵櫟木材',
		'ore_copper': '黃銅原礦',
		'ore_silver': '秘銀精礦',
		'tea_leaf': '翠嶺嫩茶',
		'wild_fruit': '山野靈果',
		'spring_water': '清冽泉露',
		'mat_iron_scrap': '回收熟鐵料塊',
		'mat_timber_log': '伐採原木',
		'mat_reed_bundle': '水澤老蘆草綑',
		'mat_foraged_herb': '山野鮮藥草',
		'mat_coarse_paper_ream': '原漿宣紙刀',
		'mat_hemp_rope_roll': '浸油粗麻繩卷',
		'mat_wet_grass_bundle': '濕青草束',
		'mat_kindling_bundle': '乾燥柴枝束',
		'mat_hay_bundle': '乾草束',
		'mat_rough_ore': '粗鐵礦石',
		'mat_bast_fiber': '強韌葛藤',
		'mat_plank': '樟松木板料',
		'mat_charcoal': '硬木白炭袋',
		'mat_torch': '浸脂松明火把',
		'mat_firewood': '乾燥柴枝束',
		'mat_timber': '伐採原木'
	};

	function formatItemName(rawName, rawId) {
		if (rawName && rawName !== rawId && !/^[a-z0-9_:-]+$/i.test(rawName)) {
			return rawName;
		}
		if (rawId && ITEM_NAME_MAP[rawId]) return ITEM_NAME_MAP[rawId];
		if (rawName && ITEM_NAME_MAP[rawName]) return ITEM_NAME_MAP[rawName];
		var key = (rawId || rawName || '').toLowerCase();
		if (key) {
			if (ITEM_NAME_MAP[key]) return ITEM_NAME_MAP[key];
			if (key.indexOf('mut_') !== 0 && key.indexOf('craft_') !== 0) {
				if (key.indexOf('herb') !== -1) return '野地草藥';
				if (key.indexOf('ore') !== -1) return '百工礦石';
				if (key.indexOf('wood') !== -1) return '堅韌原木';
				if (key.indexOf('fish') !== -1) return '鮮活河魚';
				if (key.indexOf('crystal') !== -1) return '溫潤碎晶';
			}
		}
		var fallback = rawName || rawId;
		if (!fallback || /^[a-z0-9_:-]+$/i.test(fallback)) {
			return '隨身器物';
		}
		return fallback;
	}

	var lastStatusMsg = null;
	var lastInventoryMsg = null;

	function inventorySummaryText() {
		if (!lastInventoryMsg) return '讀取中';
		var items = Array.isArray(lastInventoryMsg.items) ? lastInventoryMsg.items : [];
		if (!items.length) return '背包空';
		var shown = items.slice(0, 4).map(function (it) {
			var qty = it.qty > 1 ? (' ×' + it.qty) : '';
			return formatItemName(it.name, it.item_id) + qty;
		});
		if (items.length > shown.length) shown.push('等' + items.length + '項');
		return shown.join('、');
	}

	function toolSummaryText(msg) {
		var names = (msg && msg.equipment_names) || {};
		var held = [];
		if (names.hold_l) held.push(names.hold_l);
		if (names.hold_r) held.push(names.hold_r);
		if (held.length) return held.join('、');
		return '未持工具';
	}

	function locationSummaryText(msg) {
		if (msg && msg.room_name) return msg.room_name;
		var st = window.gameState ? window.gameState() : null;
		if (st && st.room_name) return st.room_name;
		if (st && st.me && st.me.room_name) return st.me.room_name;
		return '—';
	}

	function describeHp(cur, max) {
		if (cur == null || max == null || max <= 0) return '氣息勻暢';
		var r = cur / max;
		if (r >= 0.9) return '氣血充盈、生機磅礴';
		if (r >= 0.7) return '體膚溫潤、氣色勻和';
		if (r >= 0.4) return '氣息微促、偶有滯澀';
		if (r >= 0.2) return '步履虛浮、面有疲色';
		return '氣若游絲、創痕隱痛';
	}

	function describeInner(cur, max) {
		if (cur == null || max == null || max <= 0) return '氣息平穩';
		var r = cur / max;
		if (r >= 0.85) return '氣脈順暢、充沛綿長';
		if (r >= 0.6) return '氣脈通暢、運轉自如';
		if (r >= 0.3) return '氣息微弱、餘力稍緩';
		return '氣力匱乏、需稍作調適';
	}

	function describeSpirit(cur, max) {
		if (cur == null || max == null || max <= 0) return '心神清爽';
		var r = cur / max;
		if (r >= 0.85) return '思緒清明、敏銳專注';
		if (r >= 0.6) return '思緒敏銳、精神集中';
		if (r >= 0.3) return '心神微倦、略顯散亂';
		return '心神疲憊、亟需安歇';
	}

	function describeStamina(cur, max) {
		if (cur == null || max == null || max <= 0) return '步調輕盈';
		var r = cur / max;
		if (r >= 0.85) return '四肢沉實、輕健有力';
		if (r >= 0.6) return '步調沉穩、行止得宜';
		if (r >= 0.3) return '筋骨疲酸、動作微滯';
		return '力竭氣促、動輒喘息';
	}

	function describeDimension(val, highDesc, midDesc, lowDesc) {
		if (val == null) return midDesc;
		var n = Number(val);
		if (n >= 15) return highDesc;
		if (n >= 8) return midDesc;
		return lowDesc;
	}

	function renderStatusPane(msg) {
		lastStatusMsg = msg;
		var wrap = document.getElementById('status-content');
		if (!wrap) return;
		var isSelf = msg.is_self === true;
		var vitDesc = describeDimension(msg.vit, '筋骨淬鍊、磐石之質', '體格堅實', '骨格初成');
		var qiDesc  = describeDimension(msg.qi,  '氣息綿長、底蘊深厚', '脈絡通達', '脈氣尚淺');
		var dexDesc = describeDimension(msg.dex, '身形靈動、矯健敏捷', '手眼敏捷', '步履平緩');

		var title = (msg.display_title && msg.display_title.trim()) ? msg.display_title.trim() : '旅人';
		var origin = (msg.origin_sentence && msg.origin_sentence.trim()) ? msg.origin_sentence.trim() : '';

		var hpDesc = describeHp(msg.hp_cur, msg.hp_max);
		var innerDesc = describeInner(msg.inner_cur, msg.inner_max);
		var spiritDesc = describeSpirit(msg.spirit_cur, msg.spirit_max);
		var stamDesc = describeStamina(msg.stamina_cur, msg.stamina_max);

		// 同步更新底欄去數值化身心體感 Cue
		var vibeCueEl = document.getElementById('player-vibe') || document.getElementById('sw-vibe-cue');
		if (vibeCueEl) {
			var shortHp = String(hpDesc || '').split('、')[0] || '氣息勻暢';
			vibeCueEl.textContent = shortHp + ' · 查看自身';
		}

		var html = '<dl class="status-dl">';
		html += '<dt>[ 命途 ]</dt><dd>' + esc(title) + '</dd>';
		if (origin) {
			html += '<dt>[ 本源 ]</dt><dd>「' + esc(origin) + '」</dd>';
		}
		html += '<dt>[ 體魄氣脈 ]</dt><dd class="status-qualitative">' + esc(vitDesc) + ' ｜ ' + esc(qiDesc) + ' ｜ ' + esc(dexDesc) + '</dd>';
		html += '<dt>[ 身心氣感 ]</dt><dd class="status-four-qualitative">'
			+ '<div class="sw-vibe-row"><span class="sw-vibe-label">氣血：</span><span class="sw-vibe-val">' + esc(hpDesc) + '</span></div>'
			+ '<div class="sw-vibe-row"><span class="sw-vibe-label">氣息：</span><span class="sw-vibe-val">' + esc(innerDesc) + '</span></div>'
			+ '<div class="sw-vibe-row"><span class="sw-vibe-label">心神：</span><span class="sw-vibe-val">' + esc(spiritDesc) + '</span></div>'
			+ '<div class="sw-vibe-row"><span class="sw-vibe-label">行止：</span><span class="sw-vibe-val">' + esc(stamDesc) + '</span></div>'
			+ '</dd>';
		if (isSelf) {
			html += '<dt>[ 所在 ]</dt><dd>' + esc(locationSummaryText(msg)) + '</dd>';
		}
		// 唯一保留數值：金錢「鎂」
		var magVal = (isSelf && msg.magnesium != null) ? Number(msg.magnesium).toLocaleString('zh-TW') + ' 鎂' : '—';
		html += '<dt>[ 資財 ]</dt><dd class="status-money">' + esc(magVal) + '</dd>';
		if (isSelf) {
			html += '<dt>[ 隨身物 ]</dt><dd>' + esc(inventorySummaryText()) + '</dd>';
			html += '<dt>[ 執握工具 ]</dt><dd>' + esc(toolSummaryText(msg)) + '</dd>';
		}
		html += '</dl>';
		wrap.innerHTML = html;
	}

	var CANONICAL_EQUIP_SLOTS = [
		['head', '【冠帽】'],
		['face', '【面飾】'],
		['neck', '【項飾】'],
		['undershirt', '【內襯】'],
		['inner_armor', '【內甲】'],
		['body', '【外衫】'],
		['cloak', '【披掛】'],
		['shoulder', '【護肩】'],
		['arm', '【臂甲】'],
		['wrist', '【護腕】'],
		['hand', '【手部】'],
		['waist', '【腰佩】'],
		['legs', '【下裳】'],
		['leg_armor', '【脛甲】'],
		['feet', '【鞋履】'],
		['ring_l', '【左戒】'],
		['ring_r', '【右戒】'],
		['trinket', '【隨身】'],
		['hold_l', '【左持】'],
		['hold_r', '【右持】']
	];
	var LEGACY_EQUIP_SLOTS = [
		['weapon', '【主武】'],
		['armor', '【甲冑】'],
		['helmet', '【頭盔】'],
		['boots', '【軍靴】'],
		['accessory', '【佩飾】'],
		['tool', '【器具】']
	];
	var EQUIP_SLOTS = CANONICAL_EQUIP_SLOTS;
	var lastEquipMsg = null;

	function renderEquipmentPane(msg) {
		if (msg && (msg.equipment_slots || msg.equipment_names)) {
			lastEquipMsg = msg;
			window.lastEquipMsg = lastEquipMsg;
		} else {
			msg = lastEquipMsg || window.lastEquipMsg || {};
		}
		var wrap = document.getElementById('equip-content') || document.getElementById('player-modal-pane-equip');
		if (!wrap) return;
		var names = (msg && msg.equipment_names) || {};
		var slots = (msg && msg.equipment_slots) || {};
		var descs = (msg && msg.equipment_descs) || {};
		wrap.innerHTML = '';
		var renderSlots = CANONICAL_EQUIP_SLOTS.slice();
		var knownCodes = {};
		for (var k = 0; k < renderSlots.length; k++) {
			knownCodes[renderSlots[k][0]] = true;
		}
		for (var l = 0; l < LEGACY_EQUIP_SLOTS.length; l++) {
			var lCode = LEGACY_EQUIP_SLOTS[l][0];
			if (slots[lCode] || names[lCode]) {
				renderSlots.push(LEGACY_EQUIP_SLOTS[l]);
				knownCodes[lCode] = true;
			}
		}
		for (var sKey in slots) {
			if (slots.hasOwnProperty(sKey) && slots[sKey] && !knownCodes[sKey]) {
				renderSlots.push([sKey, '【' + sKey + '】']);
				knownCodes[sKey] = true;
			}
		}

		// 1. 當前穿戴與百工工具
		var sectionEquip = document.createElement('div');
		sectionEquip.className = 'equip-section';
		var titleEquip = document.createElement('div');
		titleEquip.className = 'equip-section-title';
		titleEquip.textContent = '【 當前穿戴與百工工具 】';
		sectionEquip.appendChild(titleEquip);

		var dl = document.createElement('dl');
		dl.className = 'status-dl equip-dl';
		for (var i = 0; i < renderSlots.length; i++) {
			(function (code, label) {
				var dt = document.createElement('dt');
				dt.textContent = label;
				dl.appendChild(dt);

				var dd = document.createElement('dd');
				var itemName = names[code];
				var itemID = slots[code];
				if (itemName && itemID) {
					dd.className = 'equip-has-item';
					dd.innerHTML = '▸ ' + esc(formatItemName(itemName, itemID));
					dd.addEventListener('click', function () {
						var existing = dl.querySelector('.equip-item-expand');
						var wasThis = existing && existing.getAttribute('data-slot') === code;
						if (existing) existing.remove();
						dl.querySelectorAll('dd.expanded').forEach(function (el) {
							el.classList.remove('expanded');
							el.innerHTML = el.innerHTML.replace('▾', '▸');
						});
						if (wasThis) return;

						dd.classList.add('expanded');
						dd.innerHTML = dd.innerHTML.replace('▸', '▾');
						var expand = document.createElement('div');
						expand.className = 'equip-item-expand';
						expand.setAttribute('data-slot', code);
						var desc = descs[code] || '';
						var descHtml = desc ? '<div class="equip-item-desc">┃ ' + esc(desc) + '</div>' : '';
						var actionsHtml = '<div class="inventory-item-actions">';
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="unequip">脫下</button>';
						actionsHtml += '</div>';
						expand.innerHTML = descHtml + actionsHtml;

						dd.after(expand);

						expand.querySelector('.inv-action-btn[data-action="unequip"]').addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'unequip_item', slot: code });
						});
					});
				} else {
					var emptyHint = (code === 'hold_l' || code === 'hold_r') ? '素手隨行' : '素常衣飾';
					dd.innerHTML = '<span class="text-muted">' + emptyHint + '</span>';
				}
				dl.appendChild(dd);
			})(renderSlots[i][0], renderSlots[i][1]);
		}
		sectionEquip.appendChild(dl);
		wrap.appendChild(sectionEquip);

		var sectionCraft = document.createElement('div');
		sectionCraft.className = 'equip-section';
		sectionCraft.innerHTML = '<div class="equip-section-title">【 手作與淬鍊 】</div>' +
			'<div class="equip-section-body craft-summary">' +
			'<div class="equip-action-row" style="display:flex;gap:8px;flex-wrap:wrap;">' +
			'<button type="button" class="inv-action-btn craft-auto-btn" data-action="craft_auto">煉製隨身原料</button>' +
			'<button type="button" class="inv-action-btn mutate-auto-btn" data-action="mutate_auto">催動器物異變</button>' +
			'</div>' +
			'</div>';
		wrap.appendChild(sectionCraft);

		var craftAutoBtn = sectionCraft.querySelector('.craft-auto-btn');
		if (craftAutoBtn) {
			craftAutoBtn.addEventListener('click', function (e) {
				e.stopPropagation();
				_send({ type: 'craft_item', item_id: 'auto' });
			});
		}
		var mutateAutoBtn = sectionCraft.querySelector('.mutate-auto-btn');
		if (mutateAutoBtn) {
			mutateAutoBtn.addEventListener('click', function (e) {
				e.stopPropagation();
				_send({ type: 'mutate_item', item_id: 'auto' });
			});
		}
	}
	function renderSkillPane(msg) {
		var wrap = document.getElementById('skill-content');
		if (!wrap) return;
		var skills = (msg && Array.isArray(msg.natural_skills)) ? msg.natural_skills : [];
		var html = '<div class="skill-section">';
		if (!skills.length) {
			html += '<div class="skill-section-title">【 身手與手藝 】</div>';
			html += '<div class="skill-section-body"><div class="skill-placeholder">氣脈初凝，隨日常採獵、百工與吐納漸次成形。</div></div>';
		} else {
			var martial = skills.filter(function (s) { return s.domain === 'martial'; });
			var prod = skills.filter(function (s) { return s.domain === 'production'; });
			if (martial.length) {
				html += '<div class="equip-section-title" style="margin:4px 0 8px 0;">【 臨陣身手 】</div>';
				for (var i = 0; i < martial.length; i++) {
					var ms = martial[i];
					var isBreathHabit = (ms.category_label || '').indexOf('內息底色') !== -1 || ms.category === 'inner_breath';
					var slotKey = 'opening';
					if (ms.category_label && ms.category_label.indexOf('起手') !== -1) slotKey = 'opening';
					else if (ms.category_label && ms.category_label.indexOf('連攜') !== -1) slotKey = 'midgame';
					else if (ms.category_label && ms.category_label.indexOf('破綻') !== -1) slotKey = 'finisher';
					else if (ms.category_label && ms.category_label.indexOf('守勢') !== -1) slotKey = 'defense';

					html += '<div class="starplate-block" style="margin-bottom:8px;padding:8px 10px;">';
					html += '<div style="display:flex;justify-content:space-between;align-items:center;">';
					html += '<div><strong>◆ 《' + esc(ms.name) + '》</strong> <span class="text-muted">｜ ' + esc(ms.category_label) + '</span></div>';
					if (!isBreathHabit) {
						html += '<button type="button" class="transcribe-manual-btn" data-slot="' + slotKey + '" style="cursor:pointer;padding:2px 8px;font-size:0.85em;border:1px solid #7c6f5a;background:rgba(218,165,32,0.1);color:#d4af37;border-radius:3px;">著譜立說</button>';
					}
					html += '</div>';
					html += '<div style="font-size:0.92em;margin-top:3px;">' + esc(ms.origin_basis) + '</div>';
					if (isBreathHabit && ms.auto_trigger_desc) {
						html += '<div style="font-size:0.88em;margin-top:2px;color:#d4af37;">行氣節奏：' + esc(ms.auto_trigger_desc) + '</div>';
					}
					if (ms.learned_and_familiar) {
						html += '<div class="text-muted" style="font-size:0.88em;margin-top:2px;">' + esc(ms.learned_and_familiar) + '</div>';
					}
					html += '</div>';
				}
			}
			if (prod.length) {
				html += '<div class="equip-section-title" style="margin:14px 0 8px 0;">【 百工手藝 】</div>';
				for (var j = 0; j < prod.length; j++) {
					var ps = prod[j];
					html += '<div class="starplate-block" style="margin-bottom:8px;padding:8px 10px;">';
					html += '<div><strong>◇ 《' + esc(ps.name) + '》</strong> <span class="text-muted">｜ ' + esc(ps.category_label) + '</span></div>';
					html += '<div style="font-size:0.92em;margin-top:3px;">' + esc(ps.origin_basis) + '</div>';
					if (ps.learned_and_familiar) {
						html += '<div class="text-muted" style="font-size:0.88em;margin-top:2px;">' + esc(ps.learned_and_familiar) + '</div>';
					}
					html += '</div>';
				}
			}
		}
		html += '</div>';
		wrap.innerHTML = html;
		wrap.querySelectorAll('.transcribe-manual-btn').forEach(function (btn) {
			btn.addEventListener('click', function (e) {
				e.stopPropagation();
				var slot = this.getAttribute('data-slot');
				_send({ type: 'inventory_action', action: 'TranscribeManual', item_id: slot });
			});
		});
	}

	// 二十主樞（361 規格 §2.2）：代碼 N001～N020、名稱
	var HUB_NAMES = ['天極', '脈衝', '震淵', '游離', '弦絲', '曜核', '凜晶', '淵流', '萬象', '解離', '鎮閾', '衡定', '穹壁', '重塑', '逆熵', '神淵', '識閾', '坍縮', '無相', '越權'];
	function formatNodeName(nodeId) {
		if (!nodeId) return '天極主樞';
		var str = String(nodeId).trim();
		if (str === 'N000') return '生之奇點';
		var m = str.match(/^N(\d{3})$/);
		if (m) {
			var idx = parseInt(m[1], 10);
			if (idx >= 1 && idx <= 20) {
				return HUB_NAMES[idx - 1] + '主樞';
			}
			return '星軌節點';
		}
		return str;
	}
	// Cost 文字化五級（狀態與星盤分頁規格 §5.5）
	function costToLabel(cost) {
		if (cost == null) return { text: '未知', css: 'cost-unknown' };
		if (cost <= 7)  return { text: '暢流', css: 'cost-flow' };
		if (cost <= 11) return { text: '順通', css: 'cost-easy' };
		if (cost <= 16) return { text: '平穩', css: 'cost-mid' };
		if (cost <= 21) return { text: '滯澀', css: 'cost-slow' };
		return { text: '險阻', css: 'cost-hard' };
	}
	function costSpan(cost) {
		var label = costToLabel(cost);
		return '<span class="' + label.css + '">' + label.text + '</span>';
	}
	// 760 邊序（361 §6.1.0）：型 A 0..19，型 B 20..119，型 C 120..419，型 D 420..659，型 E 660..759
	function getTypeCCosts(costs, hubIndex) {
		if (!costs || costs.length < 120 + (hubIndex + 1) * 15) return [];
		return costs.slice(120 + hubIndex * 15, 120 + hubIndex * 15 + 15);
	}
	function getCostA(costs, hubIndex) {
		if (!costs || costs.length <= hubIndex) return null;
		return costs[hubIndex];
	}
	function getCostB(costs, hubIndex, blueIdx) {
		var i = 20 + hubIndex * 5 + blueIdx;
		return (costs && costs.length > i) ? costs[i] : null;
	}
	function getCostD(costs, hubIndex, greenIdx) {
		var i = 420 + hubIndex * 12 + greenIdx;
		return (costs && costs.length > i) ? costs[i] : null;
	}
	function getCostE(costs, hubIndex, blueIdx) {
		var i = 660 + hubIndex * 5 + blueIdx;
		return (costs && costs.length > i) ? costs[i] : null;
	}
	var BLUE_NAMES = ['\u8d77', '\u627f', '\u8f49', '\u5354', '\u5408'];
	var GREEN_LABELS = ['【探】', '【觸】', '【納】', '【蓄】', '【濾】', '【析】', '【融】', '【衍】', '【律】', '【束】', '【釋】', '【散】'];
	var GREEN_SHORT = ['【探】', '【觸】', '【納】', '【蓄】', '【濾】', '【析】', '【融】', '【衍】', '【律】', '【束】', '【釋】', '【散】'];
	var GREEN_PER_BLUE = [[0, 1, 2], [2, 3, 4], [4, 5, 6], [7, 8, 9], [9, 10, 11]];
	var GREEN_SHARED = { 2: ' (\u8207[\u627f]\u5171\u7528)', 4: ' (\u8207[\u8f49]\u5171\u7528)', 9: ' (\u8207[\u5408]\u5171\u7528)' };

	function buildHubExpandHTML(hubIndex, costs) {
		var costA = getCostA(costs, hubIndex);
		var typeCCosts = getTypeCCosts(costs, hubIndex);
		var html = '<div class="starplate-hub-cost-card">';
		html += '<span class="hub-entry-label">主樞貫通調和：</span>' + costSpan(costA);
		html += '</div>';

		html += '<div class="starplate-gates-grid">';
		var idx = 0;
		for (var b = 0; b < 5; b++) {
			var costB = getCostB(costs, hubIndex, b);
			var nextBlue = BLUE_NAMES[(b + 1) % 5];
			var ringE = getCostE(costs, hubIndex, b);
			html += '<div class="starplate-gate-card">';
			html += '<div class="gate-header">';
			html += '<span class="gate-name">【' + BLUE_NAMES[b] + '】門</span>';
			html += '<span class="gate-cost">' + costSpan(costB) + '</span>';
			html += '<span class="gate-cycle" title="星軌旋動">⟳ ' + nextBlue + ' ' + costSpan(ringE) + '</span>';
			html += '</div>';

			html += '<div class="gate-subnodes">';
			for (var g = 0; g < 3; g++) {
				var greenIdx = GREEN_PER_BLUE[b][g];
				var costC = typeCCosts.length > idx ? typeCCosts[idx] : null;
				var ringD = getCostD(costs, hubIndex, greenIdx);
				var nextGreen = GREEN_SHORT[(greenIdx + 1) % 12];
				var shared = GREEN_SHARED[greenIdx] || '';
				html += '<div class="gate-subnode-row">';
				html += '<span class="subnode-tag">' + GREEN_LABELS[greenIdx] + '</span>';
				html += '<span class="subnode-cost">' + costSpan(costC) + '</span>';
				if (shared) html += '<span class="subnode-shared">' + shared + '</span>';
				html += '<span class="subnode-ring">⟳ ' + nextGreen + ' ' + costSpan(ringD) + '</span>';
				html += '</div>';
				idx++;
			}
			html += '</div></div>';
		}
		html += '</div>';
		return html;
	}

	function getWordLabel(w) {
		if (!w) return '';
		if (typeof w === 'object') return w.char_text || w.ch || w.word || w.name || w.word_id || w.id || '';
		return String(w);
	}

	var activeTacticalSlot = 'opening';
	var selectedWordPlateWordId = null;
	window.selectedWordPlateWordId = null;

	var lastTacticalState = {
		casket: ['疾', '穿', '震', '卸', '纏', '崩', '化', '凝', '裂', '連', '速', '刺', '斬', '流', '破', '吾', '避'],
		loadout: {
			opening:  { nodes: ['N001', 'N001', 'N001'], words: ['速', '刺', '斬'], name: '驚鴻刺' },
			midgame:  { nodes: ['N001', 'N001', 'N001'], words: ['流', '破', '斬'], name: '隨流纏手' },
			finisher: { nodes: ['N001', 'N001', 'N001'], words: ['吾', '震', '破'], name: '斷空崩' },
			defense:  { nodes: ['N001', 'N001', 'N001'], words: ['凝', '流', '避'], name: '沉嶽卸' }
		}
	};

	var TACTICAL_SLOT_DEFS = [
		{ key: 'opening',  label: '起手式 · 搶攻先機', hint: '相剋重點：迅捷搶攻，打亂節奏' },
		{ key: 'midgame',  label: '常態連攜 · 纏鬥守中', hint: '相剋重點：攻守兼備，消耗轉化' },
		{ key: 'finisher', label: '破綻殺招 · 決命一擊', hint: '相剋重點：專攻破綻，破防重創' },
		{ key: 'defense',  label: '守勢身法 · 卸力化勁', hint: '相剋重點：化勁保身，避重就輕' }
	];

	function renderTacticalSection(container) {
		var section = document.createElement('div');
		section.className = 'tactical-loadout-panel';

		// 1. 萬人萬相・心印收納
		var casketHeader = document.createElement('div');
		casketHeader.className = 'starplate-block';
		casketHeader.innerHTML = '<strong>[ 識海字譜 ]</strong>';
		section.appendChild(casketHeader);

		var casketBox = document.createElement('div');
		casketBox.className = 'tactical-casket-box';
		var words = lastTacticalState.casket || [];
		var selectedWordId = null;

		function updateSlotCard(key) {
			var card = section.querySelector('.tactical-slot-card[data-slot="' + key + '"]');
			if (!card) return;
			var cur = (lastTacticalState.loadout && lastTacticalState.loadout[key]) || { nodes: [], words: [], name: '' };
			var preview = card.querySelector('.active-words');
			if (preview) {
				var wordListStr = (cur.words && cur.words.length) ? cur.words.map(function(w){ return '【' + getWordLabel(w) + '】'; }).join(' ') : '（未凝心印）';
				preview.textContent = wordListStr;
			}
			var nodesPreview = card.querySelector('.tactical-nodes-preview');
			if (nodesPreview) {
				var nodeListStr = (cur.nodes && cur.nodes.length) ? cur.nodes.map(formatNodeName).join(' · ') : '天極主樞';
				nodesPreview.textContent = '星樞：' + nodeListStr;
			}
		}

		for (var wi = 0; wi < words.length; wi++) {
			(function (item) {
				var label = getWordLabel(item);
				var wBtn = document.createElement('button');
				wBtn.type = 'button';
				wBtn.className = 'tactical-word-crystal';
				var countBadge = (item.count && item.count > 1) ? '（' + item.count + '枚）' : '';
				var qualStr = item.quality_label ? ' [' + item.quality_label + ']' : '';
				wBtn.textContent = '【' + label + '】' + qualStr + countBadge;
				wBtn.setAttribute('data-word', label);
				wBtn.addEventListener('click', function () {
					selectedWordId = item.id || label;
					selectedWordPlateWordId = item.id || label;
					window.selectedWordPlateWordId = selectedWordPlateWordId;
					casketBox.querySelectorAll('.tactical-word-crystal').forEach(function (b) {
						b.classList.remove('selected');
					});
					wBtn.classList.add('selected');
					if (!lastTacticalState.loadout) lastTacticalState.loadout = {};
					if (!lastTacticalState.loadout[activeTacticalSlot]) {
						lastTacticalState.loadout[activeTacticalSlot] = { nodes: [], words: [], name: '' };
					}
					var cur = lastTacticalState.loadout[activeTacticalSlot];
					if (!Array.isArray(cur.words)) cur.words = [];
					cur.words.push(label);
					if (!Array.isArray(cur.nodes)) cur.nodes = [];
					if (cur.nodes.length < cur.words.length) {
						var nIdx = cur.nodes.length + 1;
						cur.nodes.push('N' + String(nIdx).padStart(3, '0'));
					}
					updateSlotCard(activeTacticalSlot);
				});
				casketBox.appendChild(wBtn);
			})(words[wi]);
		}
		section.appendChild(casketBox);
		var casketActions = document.createElement('div');
		casketActions.className = 'tactical-casket-actions';
		casketActions.style.display = 'flex';
		casketActions.style.gap = '8px';
		casketActions.style.margin = '6px 0 12px 0';

		var stackBtn = document.createElement('button');
		stackBtn.type = 'button';
		stackBtn.className = 'inv-action-btn';
		stackBtn.textContent = '同名疊煉';
		stackBtn.addEventListener('click', function () {
			if (!selectedWordId) {
				alert('請先點選欲疊煉的心印字韻');
				return;
			}
			_send({ type: 'use_item', item_id: selectedWordId, action: 'StackWord' });
		});

		var mintBtn = document.createElement('button');
		mintBtn.type = 'button';
		mintBtn.className = 'inv-action-btn';
		mintBtn.textContent = '鎔鑄化鎂';
		mintBtn.addEventListener('click', function () {
			if (!selectedWordId) {
				alert('請先點選欲化鎂的心印字韻');
				return;
			}
			_send({ type: 'use_item', item_id: selectedWordId, action: 'MintWord' });
		});

		casketActions.appendChild(stackBtn);
		casketActions.appendChild(mintBtn);
		section.appendChild(casketActions);

		// 2. 成相四序・臨陣應變
		var slotHeader = document.createElement('div');
		slotHeader.className = 'starplate-block';
		slotHeader.innerHTML = '<strong>[ 四序星軌 ]</strong>';
		section.appendChild(slotHeader);

		var slotsWrap = document.createElement('div');
		slotsWrap.className = 'tactical-slots-grid';

		for (var si = 0; si < TACTICAL_SLOT_DEFS.length; si++) {
			(function (slotDef) {
				var slotKey = slotDef.key;
				var current = (lastTacticalState.loadout && lastTacticalState.loadout[slotKey]) || { nodes: [], words: [], name: '' };
				var card = document.createElement('div');
				card.className = 'tactical-slot-card' + (slotKey === activeTacticalSlot ? ' tactical-slot-active' : '');
				card.setAttribute('data-slot', slotKey);

				card.addEventListener('click', function (e) {
					if (e.target.tagName === 'INPUT' || e.target.tagName === 'BUTTON') return;
					activeTacticalSlot = slotKey;
					slotsWrap.querySelectorAll('.tactical-slot-card').forEach(function (c) {
						c.classList.remove('tactical-slot-active');
					});
					card.classList.add('tactical-slot-active');
				});

				var cardTitle = document.createElement('div');
				cardTitle.className = 'tactical-slot-title';
				cardTitle.innerHTML = '<strong>' + esc(slotDef.label) + '</strong><div class="tactical-slot-hint">' + esc(slotDef.hint) + '</div>';
				card.appendChild(cardTitle);

				var nameRow = document.createElement('div');
				nameRow.className = 'tactical-name-row';
				nameRow.innerHTML = '<label>招式：</label><span class="tactical-natural-name"><strong>' + esc(current.name || '順脈連式') + '</strong></span>';
				card.appendChild(nameRow);

				// 裝配詞晶與節點
				var equipRow = document.createElement('div');
				equipRow.className = 'tactical-equip-row';
				var wordListStr = (current.words && current.words.length) ? current.words.map(function(w){ return '【' + getWordLabel(w) + '】'; }).join(' ') : '（未凝心印）';
				var nodeListStr = (current.nodes && current.nodes.length) ? current.nodes.map(formatNodeName).join(' · ') : '天極主樞';
				equipRow.innerHTML = '<div class="tactical-words-preview">心印：<span class="active-words">' + esc(wordListStr) + '</span></div>' +
									 '<div class="tactical-nodes-preview">星樞：' + esc(nodeListStr) + '</div>';
				card.appendChild(equipRow);

				var commitBtn = document.createElement('button');
				commitBtn.type = 'button';
				commitBtn.className = 'tactical-commit-btn';
				commitBtn.textContent = '引字入樞（' + slotDef.label.split(' · ')[0] + '）';
				commitBtn.addEventListener('click', function () {
					var nodes = current.nodes && current.nodes.length ? current.nodes.slice() : ['N001'];
					var w = current.words && current.words.length ? current.words.map(getWordLabel) : ['疾'];
					while (nodes.length < w.length) nodes.push(nodes[0] || 'N001');
					if (nodes.length > w.length) nodes = nodes.slice(0, w.length);
					_send({
						type: 'update_tactics',
						slot: slotKey,
						nodes: nodes,
						words: w,
						custom_name: ''
					});
				});
				card.appendChild(commitBtn);
			})(TACTICAL_SLOT_DEFS[si]);
		}
		section.appendChild(slotsWrap);
		container.appendChild(section);
	}

	window.onTacticalState = function (msg) {
		if (!msg) return;
		if (msg.casket) lastTacticalState.casket = msg.casket;
		if (msg.loadout) lastTacticalState.loadout = msg.loadout;
		var me = (window.gameState && window.gameState().me) || (lastStatusMsg && lastStatusMsg.is_self ? lastStatusMsg : null);
		if (me) renderStarplatePane(me);
	};

	window.openTacticsModal = function () {
		var modalOverlay = document.getElementById('player-modal-overlay');
		if (modalOverlay) modalOverlay.hidden = false;
		var tabBtn = document.querySelector('.player-modal-tab[data-tab="starplate"]');
		if (tabBtn) tabBtn.click();
	};

	function renderStarplatePane(me) {
		var wrap = document.getElementById('starplate-content');
		if (!wrap) return;

		wrap.innerHTML = '';

		// 頂部優先渲染：隨身詞匣與星盤四戰術序位配招介面
		renderTacticalSection(wrap);

		if (!me || !me.activated_nodes || !Array.isArray(me.activated_nodes)) {
			var note = document.createElement('div');
			note.className = 'starplate-block text-muted';
			note.textContent = '（登入後可隨意觀覽完整三十六宮與二十主樞展開態）';
			wrap.appendChild(note);
			return;
		}

		var activated = me.activated_nodes;
		var count = activated.length;
		var costs = me.topology_costs || [];

		var header = document.createElement('div');
		header.className = 'starplate-block starplate-paper-header';
		header.innerHTML = '<div class="starplate-paper-title">【 三百六十星軌共鳴 】</div>' +
			'<div class="starplate-paper-sub">以二十主星樞為基，貫通五門邏輯、牽引十二真言成相。點擊主樞展開星位脈絡。</div>';
		wrap.appendChild(header);

		var gridContainer = document.createElement('div');
		gridContainer.className = 'starplate-paper-grid';
		wrap.appendChild(gridContainer);

		for (var i = 0; i < HUB_NAMES.length; i++) {
			(function (hubIndex) {
				var costA = getCostA(costs, hubIndex);
				var adaptStr = costA != null ? costSpan(costA) : '<span class="text-muted">調和中</span>';

				var card = document.createElement('div');
				card.className = 'starplate-hub-card';
				card.setAttribute('data-hub', hubIndex);

				var cardHeader = document.createElement('div');
				cardHeader.className = 'hub-card-header';
				cardHeader.innerHTML = '<span class="hub-name">▸ 【' + HUB_NAMES[hubIndex] + '主樞】</span>' +
					'<span class="hub-adapt">' + adaptStr + '</span>';
				card.appendChild(cardHeader);
				gridContainer.appendChild(card);

				card.addEventListener('click', function () {
					var isExpanded = card.classList.contains('expanded');
					gridContainer.querySelectorAll('.starplate-hub-card.expanded').forEach(function (c) {
						c.classList.remove('expanded');
						var h = c.querySelector('.hub-name');
						if (h) h.innerHTML = h.innerHTML.replace('▾', '▸');
						var exp = c.querySelector('.starplate-hub-expand');
						if (exp) exp.remove();
					});
					if (isExpanded) return;

					card.classList.add('expanded');
					var hName = card.querySelector('.hub-name');
					if (hName) hName.innerHTML = hName.innerHTML.replace('▸', '▾');

					var expandEl = document.createElement('div');
					expandEl.className = 'starplate-hub-expand';
					expandEl.setAttribute('data-hub', hubIndex);
					expandEl.innerHTML = buildHubExpandHTML(hubIndex, costs);
					card.appendChild(expandEl);
				});
			})(i);
		}
	}

	function describeItemWeight(subTotal) {
		var w = Number(subTotal);
		if (isNaN(w) || w <= 0) return '輕巧';
		if (w < 1.0) return '輕巧';
		if (w < 3.0) return '適手';
		if (w < 8.0) return '微沉';
		return '沉重';
	}

	function renderInventoryContent(msg) {
		lastInventoryMsg = msg;
		if (lastStatusMsg && lastStatusMsg.is_self === true) renderStatusPane(lastStatusMsg);
		var weightEl = document.getElementById('inventory-weight');
		var listEl = document.getElementById('inventory-list');
		if (!weightEl || !listEl) return;
		var curW = Number(msg.current_weight) || 0;
		var maxW = Number(msg.max_weight) || 120;
		var ratio = maxW > 0 ? (curW / maxW) : 0;
		var overweight = curW > maxW;
		var burdenDesc = '輕便自如・身輕如燕';
		var burdenClass = 'inventory-weight-light';
		if (overweight || ratio >= 1.0) {
			burdenDesc = '負重累累・步履難移';
			burdenClass = 'inventory-weight-warn';
		} else if (ratio >= 0.8) {
			burdenDesc = '囊橐沉甸・微感吃力';
			burdenClass = 'inventory-weight-heavy';
		} else if (ratio >= 0.4) {
			burdenDesc = '行囊微沉・步履自如';
			burdenClass = 'inventory-weight-mid';
		}
		weightEl.innerHTML = '行囊狀態：<span class="' + burdenClass + '">' + esc(burdenDesc) + '</span>';
		if (!msg.items || msg.items.length === 0) {
			listEl.innerHTML = '<div class="inventory-empty">（背包空空如也）</div>';
			return;
		}
		listEl.innerHTML = '';
		for (var i = 0; i < msg.items.length; i++) {
			(function (it) {
				var itemId = it.item_id || it.id || '';
				var itemName = it.name || '';
				var isMutated = itemId.indexOf('mut_') === 0 || itemName.indexOf('【異變') !== -1;
				var isRaw = !isMutated && (itemId.indexOf('mat_') === 0 || itemId.indexOf('harvest_') === 0 || /礦|砂|原木|蘆草|竹竿|草藥|水苔|紙刀|墨坯|麻繩|袋布|原牛皮|河蝦|水芹/.test(itemName));
				var stageBadge = isMutated ? '[異變奇珍]' : (isRaw ? '[原料]' : '[成品]');
				var qtyStr = it.qty > 1 ? (' \u00d7' + it.qty) : '';
				var wTag = '[' + describeItemWeight(it.sub_total) + ']';
				var nameStr = formatItemName(itemName, itemId);
				var row = document.createElement('div');
				row.className = 'inventory-item';
				row.innerHTML = '<span class="inventory-item-name">\u25b8 ' + esc(nameStr) + qtyStr + '</span>'
					+ '<span class="inventory-item-weight"><span class="item-stage-tag" style="margin-right:6px;opacity:0.85;">' + esc(stageBadge) + '</span>' + esc(wTag) + '</span>';
				listEl.appendChild(row);
				row.addEventListener('click', function () {
					var existing = listEl.querySelector('.inventory-item-expand');
					var wasThisItem = existing && existing.getAttribute('data-item-id') === it.item_id;
					if (existing) {
						var prevRow = existing.previousElementSibling;
						if (prevRow) {
							prevRow.classList.remove('expanded');
							var pn = prevRow.querySelector('.inventory-item-name');
							if (pn) pn.innerHTML = pn.innerHTML.replace('\u25be', '\u25b8');
						}
						existing.remove();
					}
					if (wasThisItem) return;

					listEl.querySelectorAll('.inventory-item.expanded').forEach(function (el) {
						el.classList.remove('expanded');
						var en = el.querySelector('.inventory-item-name');
						if (en) en.innerHTML = en.innerHTML.replace('\u25be', '\u25b8');
					});
					row.classList.add('expanded');
					var nameSpan = row.querySelector('.inventory-item-name');
					if (nameSpan) nameSpan.innerHTML = nameSpan.innerHTML.replace('\u25b8', '\u25be');

					var expand = document.createElement('div');
					expand.className = 'inventory-item-expand';
					expand.setAttribute('data-item-id', it.item_id);
					var desc = it.description || '';
					var descHtml = desc ? '<div class="inventory-item-desc">\u2503 ' + esc(desc) + '</div>' : '';
					var actionsHtml = '<div class="inventory-item-actions">';
					var isCrystal = /^crystal_/.test(it.item_id) || it.item_type === 'word_crystal';
					var isCorpse = /^corpse_/.test(it.item_id) || it.item_type === 'corpse';
					var isManual = /^manual_/.test(it.item_id) || it.item_type === 'manual' || it.item_id === 'blank_manual' || it.item_id === 'wt_blank_manual';
					if (it.slot) {
						if (it.slot === 'hold' || it.slot === 'hold_r' || it.slot === 'hold_l') {
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip" data-target="hold_l">\u5de6\u624b</button>';
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip" data-target="hold_r">\u53f3\u624b</button>';
						} else {
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip">\u7a7f\u6234</button>';
						}
					}
					if (!isCrystal && it.slot) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="inscribe-item">器物刻字</button>';
					}
					if (isCrystal) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="extract-word">碎晶提出</button>';
					}
					if (isCorpse) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="strip-name-whole">整軀剝名</button>';
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="butcher-corpse">肢體分解</button>';
					}
					if (/^manual_/.test(it.item_id)) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="study-manual">研讀手札</button>';
					}
					if (!it.slot && !isCrystal && !isRaw && !isCorpse && !/^manual_/.test(it.item_id)) {
						var isFoodOrDrink = /[餅麵飯肉薯凍花菜蝦蛤魚丸糖湯茶漿飲水酒粥粿]/.test(it.name || '');
						var useLabel = isFoodOrDrink ? '食用' : '使用';
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="use">' + useLabel + '</button>';
					}
					if (!it.slot && !isCrystal && !isCorpse && !isManual) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="strip-name">剝名凝晶</button>';
					}
					if (isRaw) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="craft">加工煉製</button>';
					} else if (!isMutated && !isCrystal && !isCorpse && !isManual) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="mutate">氣脈異變</button>';
					}
					actionsHtml += '<button type="button" class="inv-action-btn" data-action="sell" data-sell-mode="one">售出</button>';
					if (it.qty && it.qty > 1) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="sell" data-sell-mode="all">全部售出</button>';
					}
					actionsHtml += '<button type="button" class="inv-action-btn" data-action="drop" data-drop-mode="one">\u4e1f\u68c4</button>';
					if (it.qty && it.qty > 1) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="drop" data-drop-mode="all">\u5168\u90e8\u4e1f\u68c4</button>';
					}
					actionsHtml += '</div>';
					expand.innerHTML = descHtml + actionsHtml;
					row.after(expand);

					expand.querySelectorAll('.inv-action-btn[data-action="equip"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							var payload = { type: 'equip_item', item_id: it.item_id };
							var target = btn.getAttribute('data-target');
							if (target) payload.target_slot = target;
							_send(payload);
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="use"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'use_item', item_id: it.item_id });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="extract-word"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'use_item', item_id: it.item_id, action: 'ExtractWord' });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="strip-name"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'use_item', item_id: it.item_id, action: 'StripName' });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="strip-name-whole"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'use_item', item_id: it.item_id, action: 'StripNameWhole' });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="butcher-corpse"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'use_item', item_id: it.item_id, action: 'ButcherCorpse' });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="study-manual"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'use_item', item_id: it.item_id, action: 'StudyManual' });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="inscribe-item"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							var wordId = selectedWordPlateWordId || window.selectedWordPlateWordId;
							if (!wordId && lastTacticalState && lastTacticalState.casket && lastTacticalState.casket.length > 0) {
								var first = lastTacticalState.casket[0];
								wordId = (typeof first === 'object' && first !== null) ? (first.id || first.char || first.ch || first.word || first.name) : first;
							}
							if (!wordId) {
								wordId = prompt('請輸入欲銘刻於器物之字韻（如：斬、解、疾、薄）：');
								if (!wordId) return;
							}
							var doSend = (typeof send === 'function') ? send : _send;
							doSend({
								type: 'use_item',
								item_id: it.item_id + '|' + wordId,
								action: 'InscribeItem'
							});
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="craft"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'craft_item', item_id: itemId });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="mutate"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'mutate_item', item_id: itemId });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="sell"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							var mode = btn.getAttribute('data-sell-mode') || 'one';
							_send({ type: 'sell_item', item_id: it.item_id, action: mode === 'all' ? 'all' : 'one' });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="drop"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							var mode = btn.getAttribute('data-drop-mode') || 'one';
							_send({ type: 'drop_item', item_id: it.item_id, action: mode === 'all' ? 'all' : 'one' });
						});
					});
				});
			})(msg.items[i]);
		}
	}
	function applyFontScale(scale) {
		if (scale === 'large') {
			document.documentElement.style.fontSize = '17.5px';
		} else if (scale === 'medium') {
			document.documentElement.style.fontSize = '16px';
		} else {
			document.documentElement.style.fontSize = '15px';
		}
	}

	try {
		var savedFont = localStorage.getItem('sw_font_scale') || 'normal';
		applyFontScale(savedFont);
	} catch (e) {}

	function renderSettingsPane() {
		var wrap = document.getElementById('settings-content') || document.getElementById('player-modal-pane-settings');
		if (!wrap) return;

		var currentFont = localStorage.getItem('sw_font_scale') || 'normal';
		var defaultMapExpand = localStorage.getItem('sw_map_default_expand') !== 'false';

		var html = '<div class="settings-panel">';

		// 1. 介面字級縮放
		html += '<div class="settings-section">';
		html += '<div class="settings-section-title">【 介面字級縮放 】</div>';
		html += '<div class="settings-options font-scale-options">';
		html += '<label class="settings-radio-label"><input type="radio" name="sw_font_scale" value="normal"' + (currentFont === 'normal' ? ' checked' : '') + '> 標準 (100%)</label>';
		html += '<label class="settings-radio-label"><input type="radio" name="sw_font_scale" value="medium"' + (currentFont === 'medium' ? ' checked' : '') + '> 適中 (110%)</label>';
		html += '<label class="settings-radio-label"><input type="radio" name="sw_font_scale" value="large"' + (currentFont === 'large' ? ' checked' : '') + '> 大字 (120%)</label>';
		html += '</div>';
		html += '</div>';

		// 2. 輿圖預設視野
		html += '<div class="settings-section">';
		html += '<div class="settings-section-title">【 輿圖視野偏好 】</div>';
		html += '<label class="settings-checkbox-label">';
		html += '<input type="checkbox" id="setting-map-expand"' + (defaultMapExpand ? ' checked' : '') + '> 登入時保持大地圖展開';
		html += '</label>';
		html += '</div>';

		// 3. 敘事歷程管理
		html += '<div class="settings-section">';
		html += '<div class="settings-section-title">【 敘事動態管理 】</div>';
		html += '<p class="settings-desc">清除當前畫面累積的歷程動態，重整視野：</p>';
		html += '<button type="button" id="setting-btn-clear-log" class="settings-act-btn">清除當前動態紀錄</button>';
		html += '</div>';

		// 4. 角色與帳號
		html += '<div class="settings-section">';
		html += '<div class="settings-section-title">【 角色與帳號 】</div>';
		html += '<p class="settings-desc">暫別此身，返回登入與切換角色介面：</p>';
		html += '<button type="button" id="setting-btn-logout" class="settings-act-btn settings-btn-logout">登出／切換角色</button>';
		html += '</div>';

		html += '</div>';
		wrap.innerHTML = html;

		// 事件綁定
		var radios = wrap.querySelectorAll('input[name="sw_font_scale"]');
		radios.forEach(function (radio) {
			radio.addEventListener('change', function () {
				var val = radio.value;
				localStorage.setItem('sw_font_scale', val);
				applyFontScale(val);
			});
		});

		var mapCheck = wrap.querySelector('#setting-map-expand');
		if (mapCheck) {
			mapCheck.addEventListener('change', function () {
				localStorage.setItem('sw_map_default_expand', mapCheck.checked ? 'true' : 'false');
			});
		}

		var clearBtn = wrap.querySelector('#setting-btn-clear-log');
		if (clearBtn) {
			clearBtn.addEventListener('click', function () {
				var logEl = document.getElementById('sw-log');
				if (logEl) {
					logEl.innerHTML = '';
				}
				if (window.SwLog && window.SwLog.appendLog) {
					window.SwLog.appendLog('敘事動態紀錄已清理，四顧澄清。');
				}
			});
		}

		var logoutBtn = wrap.querySelector('#setting-btn-logout');
		if (logoutBtn) {
			logoutBtn.addEventListener('click', function () {
				if (window.gameWs && window.gameWs.close) {
					try { window.gameWs.close(); } catch (e) {}
				}
				localStorage.removeItem('sw_token');
				sessionStorage.removeItem('sw_token');
				window.location.reload();
			});
		}
	}

	window.SwPanes = {
		renderStatusPane: renderStatusPane,
		renderEquipmentPane: renderEquipmentPane,
		renderSkillPane: renderSkillPane,
		renderStarplatePane: renderStarplatePane,
		renderInventoryContent: renderInventoryContent,
		renderInventoryPane: renderInventoryContent,
		renderSettingsPane: renderSettingsPane,
		applyFontScale: applyFontScale,
		onTacticalState: window.onTacticalState,
		openTacticsModal: window.openTacticsModal,
		renderTacticalSection: renderTacticalSection,
		formatItemName: formatItemName,
		ITEM_NAME_MAP: ITEM_NAME_MAP
	};
})();
