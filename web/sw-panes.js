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
		'harvest_crystal': '寒水玉原石',
		'herb_lingzhi': '光葉藥草',
		'ore_black_iron': '青脊重礦',
		'wood_iron_oak': '霜林鐵杉木',
		'ore_copper': '紅土銅原礦',
		'ore_silver': '銀紋輝石',
		'tea_leaf': '霜林嫩茶',
		'wild_fruit': '山野漿果',
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
		'mat_timber': '伐採原木',
		'mat_fresh_shrimp': '水尾寮鮮沼蝦',
		'shanhai_fresh_fish': '深海鮮魚',
		'shanhai_beast_meat': '山海鮮獸肉',
		'mat_salted_fish': '海鹽醃魚乾',
		'mat_grain_sack': '農家精選穀糧袋',
		'mat_pasture_seed_sack': '優質牧草種籽袋',
		'mat_plank_board': '樟松木板料',
		'mat_charcoal_sack': '硬木白炭袋',
		'craft_roasted_shrimp_skewer': '炭炙水尾鮮蝦串',
		'craft_smoked_fish_fillet': '相思木燻鮮魚乾',
		'craft_spiced_beast_jerky': '馬告炭炙獸肉脯',
		'craft_ration_grain_cake': '行旅耐儲米糧餅'
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
				if (key.indexOf('shrimp') !== -1) return '水尾寮鮮沼蝦';
				if (key.indexOf('herb') !== -1) return '野地草藥';
				if (key.indexOf('ore') !== -1) return '百工礦石';
				if (key.indexOf('wood') !== -1) return '堅韌原木';
				if (key.indexOf('fish') !== -1) return '鮮活河魚';
				if (key.indexOf('crystal') !== -1) return '寒水玉原石';
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
		var nameMap = {};
		var nameOrder = [];
		for (var i = 0; i < items.length; i++) {
			var it = items[i];
			if (!it) continue;
			var dName = formatItemName(it.name, it.item_id);
			var q = Number(it.qty) || 1;
			if (nameMap[dName] == null) {
				nameMap[dName] = 0;
				nameOrder.push(dName);
			}
			nameMap[dName] += q;
		}
		var shown = nameOrder.slice(0, 4).map(function (dName) {
			var totalQty = nameMap[dName];
			var qtyStr = totalQty > 1 ? (' ×' + totalQty) : '';
			return dName + qtyStr;
		});
		var summary = shown.join('、');
		if (nameOrder.length > shown.length) summary += ' 等 ' + nameOrder.length + ' 項';
		return summary;
	}

	function toolSummaryText(msg) {
		var names = (msg && msg.equipment_names) || {};
		var held = [];
		if (names.hold_l) held.push(names.hold_l);
		if (names.hold_r) held.push(names.hold_r);
		var zlTools = [];
		var ids = Array.isArray(_lastInventoryItemIds) ? _lastInventoryItemIds : [];
		if (ids.indexOf('zl_item_bamboo_stone_axe') !== -1) zlTools.push('竹柄石斧');
		if (ids.indexOf('zl_item_rhizome_club') !== -1) zlTools.push('竹鞭根槌');
		if (ids.indexOf('zl_item_bamboo_spade') !== -1) zlTools.push('掘土竹鏟');
		if (ids.indexOf('zl_item_fire_hardened_bamboo_knife') !== -1) zlTools.push('烤刃竹片刀');
		if (ids.indexOf('zl_item_sharp_stone_flake') !== -1) zlTools.push('溪床尖石片');
		if (ids.indexOf('zl_item_bamboo_basket') !== -1) zlTools.push('刺竹背簍');
		if (ids.indexOf('zl_item_bamboo_canteen_full') !== -1) zlTools.push('盛水青竹筒');
		else if (ids.indexOf('zl_item_bamboo_canteen') !== -1) zlTools.push('青竹水筒');
		for (var zi = 0; zi < zlTools.length; zi++) {
			if (held.indexOf(zlTools[zi]) === -1) held.push(zlTools[zi]);
		}
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
			vibeCueEl.textContent = shortHp;
		}

		var html = '<dl class="status-dl">';
		html += '<dt>[ 命途 ]</dt><dd>' + esc(title) + '</dd>';
		if (origin) {
			html += '<dt>[ 本源 ]</dt><dd>「' + esc(origin) + '」</dd>';
		}
		html += '<dt>[ 體魄氣脈 ]</dt><dd class="status-qualitative">' + esc(vitDesc) + '、' + esc(qiDesc) + '、' + esc(dexDesc) + '</dd>';
		html += '<dt>[ 身心氣感 ]</dt><dd class="status-four-qualitative">'
			+ '<div class="sw-vibe-row"><span class="sw-vibe-label">氣血：</span><span class="sw-vibe-val">' + esc(hpDesc) + '</span></div>'
			+ '<div class="sw-vibe-row"><span class="sw-vibe-label">氣息：</span><span class="sw-vibe-val">' + esc(innerDesc) + '</span></div>'
			+ '<div class="sw-vibe-row"><span class="sw-vibe-label">心神：</span><span class="sw-vibe-val">' + esc(spiritDesc) + '</span></div>'
			+ '<div class="sw-vibe-row"><span class="sw-vibe-label">行止：</span><span class="sw-vibe-val">' + esc(stamDesc) + '</span></div>'
			+ '</dd>';
		if (isSelf) {
			var breathSkill = null;
			if (Array.isArray(msg.natural_skills)) {
				for (var bi = 0; bi < msg.natural_skills.length; bi++) {
					if (msg.natural_skills[bi] && msg.natural_skills[bi].category === 'inner_breath') {
						breathSkill = msg.natural_skills[bi];
						break;
					}
				}
			}
			if (breathSkill) {
				html += '<dt>[ 內息底色 ]</dt><dd><strong>' + esc(breathSkill.name) + '</strong><div style="font-size:0.86em;color:var(--ink-muted, #5c4b37);margin-top:2px;line-height:1.45;">' + esc(breathSkill.auto_trigger_desc || '') + '</div></dd>';
			}
			html += '<dt>[ 所在 ]</dt><dd>' + esc(locationSummaryText(msg)) + '</dd>';
		}
		// 唯一保留數值：金錢「鎂」
		var magVal = (isSelf && msg.magnesium != null) ? Number(msg.magnesium).toLocaleString('zh-TW') + ' 鎂' : '—';
		html += '<dt>[ 資財 ]</dt><dd class="status-money">' + esc(magVal) + '</dd>';
		if (isSelf) {
			html += '<dt>[ 隨身物 ]</dt><dd>' + esc(inventorySummaryText()) + '</dd>';
			html += '<dt>[ 執握工具 ]</dt><dd>' + esc(toolSummaryText(msg)) + '</dd>';

			var curDirCode = msg.offline_directive || 'inn_rest';
			var curDirLabel = msg.offline_directive_label || '客房歇息養傷';
			var curDirDesc = msg.offline_directive_desc || '投宿歇舍或居室內靜養生息，徐徐調理身心與氣血。';
			var directiveOptions = [
				{ code: 'inn_rest', label: '客房歇息養傷' },
				{ code: 'workshop_craft', label: '駐坊百工代工' },
				{ code: 'field_tend', label: '照料田壟與林緣巡採' },
				{ code: 'road_escort', label: '古道隨行護運' }
			];

			html += '<dt>[ 離線行事 ]</dt><dd class="status-offline-directive">'
				+ '<div class="offline-directive-summary" style="margin-bottom:6px;line-height:1.45;">'
				+ '<strong>' + esc(curDirLabel) + '</strong>'
				+ '<div style="font-size:0.86em;color:var(--ink-muted, #5c4b37);margin-top:2px;">' + esc(curDirDesc) + '</div>'
				+ '</div>'
				+ '<div class="offline-directive-btn-group" style="display:flex;flex-wrap:wrap;gap:6px;margin-top:6px;">';

			for (var di = 0; di < directiveOptions.length; di++) {
				var opt = directiveOptions[di];
				var isActive = opt.code === curDirCode;
				var btnStyle = 'display:inline-block;padding:3px 8px;font-size:0.82em;border-radius:3px;cursor:pointer;user-select:none;transition:all 0.15s ease;'
					+ (isActive
						? 'background:var(--sw-accent-ink, #9e2a2b);color:#f7f2e7;border:1px solid var(--sw-accent-ink, #9e2a2b);font-weight:600;'
						: 'background:var(--sw-btn-bg, rgba(44,30,15,0.04));color:var(--ink-light, #594c3c);border:1px solid var(--sw-btn-border, rgba(73,51,28,0.25));');
				html += '<button type="button" class="sw-offline-dir-btn' + (isActive ? ' is-active' : '') + '" data-directive="' + esc(opt.code) + '" style="' + btnStyle + '">'
					+ esc(opt.label)
					+ '</button>';
			}
			html += '</div></dd>';
		}
		html += '</dl>';
		wrap.innerHTML = html;

		if (isSelf) {
			wrap.querySelectorAll('.sw-offline-dir-btn').forEach(function (btn) {
				btn.addEventListener('click', function (e) {
					e.stopPropagation();
					var code = this.getAttribute('data-directive');
					if (!code) return;
					_send({ type: 'set_offline_directive', action: code });
				});
			});
		}
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
		['boots', '【長靴】'],
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

		var equippedSlots = [];
		var unequippedSlots = [];
		for (var i = 0; i < renderSlots.length; i++) {
			var slotInfo = renderSlots[i];
			var code = slotInfo[0];
			if (names[code] && slots[code]) {
				equippedSlots.push(slotInfo);
			} else {
				unequippedSlots.push(slotInfo);
			}
		}

		if (equippedSlots.length > 0) {
			var dl = document.createElement('dl');
			dl.className = 'status-dl equip-dl';
			for (var ei = 0; ei < equippedSlots.length; ei++) {
				(function (code, label) {
					var dt = document.createElement('dt');
					dt.textContent = label;
					dl.appendChild(dt);

					var dd = document.createElement('dd');
					var itemName = names[code];
					var itemID = slots[code];
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
					dl.appendChild(dd);
				})(equippedSlots[ei][0], equippedSlots[ei][1]);
			}
			sectionEquip.appendChild(dl);
		} else {
			var noneEl = document.createElement('div');
			noneEl.className = 'equip-none-summary';
			noneEl.style.cssText = 'padding: 4px 0 8px; font-size: 0.88em; color: var(--ink-muted, #7c6853);';
			noneEl.textContent = '身著素常衣飾，雙手素常隨行。';
			sectionEquip.appendChild(noneEl);
		}

		if (unequippedSlots.length > 0) {
			var unequipDetails = document.createElement('details');
			unequipDetails.className = 'equip-unequipped-summary';
			unequipDetails.style.cssText = 'margin-top: 6px; margin-bottom: 6px; font-size: 0.82rem; color: var(--ink-muted, #7c6853);';
			var summary = document.createElement('summary');
			summary.style.cssText = 'cursor: pointer; user-select: none; color: var(--ink-muted, #665544); padding: 4px 0; outline: none;';
			summary.textContent = '其餘部位（' + unequippedSlots.length + ' 處素常衣飾／隨行） ▾';
			unequipDetails.appendChild(summary);

			var unequipGrid = document.createElement('div');
			unequipGrid.style.cssText = 'display: flex; flex-wrap: wrap; gap: 4px 8px; padding: 4px 0; color: var(--ink-muted, #887766); line-height: 1.5; font-size: 0.78rem;';
			var unequipLabels = unequippedSlots.map(function (s) {
				var code = s[0];
				var cleanLabel = s[1].replace(/^[【\[]|[\】\]]$/g, '');
				var hint = (code === 'hold_l' || code === 'hold_r') ? '素手' : '素常';
				return cleanLabel + '・' + hint;
			});
			unequipGrid.textContent = unequipLabels.join('、');
			unequipDetails.appendChild(unequipGrid);
			sectionEquip.appendChild(unequipDetails);
		}
		wrap.appendChild(sectionEquip);

		var sectionCraft = document.createElement('div');
		sectionCraft.className = 'equip-section';

		function isRawMaterialItem(it) {
			if (!it) return false;
			var itemId = String(it.item_id || it.id || '');
			if (typeof it.item_stage === 'string' && it.item_stage.length > 0) {
				return it.item_stage === 'raw' || Boolean(it.can_craft);
			}
			var itemName = String(it.name || '');
			var isFinishedOrMutated = itemId.indexOf('mut_') === 0 || itemId.indexOf('craft_') === 0 || itemId.indexOf('manual_') === 0 || itemId.indexOf('item_') === 0 || itemName.indexOf('【異變') !== -1 || itemName.indexOf('精鍛') !== -1;
			if (isFinishedOrMutated) return false;
			return Boolean(it.can_craft) ||
				itemId.indexOf('mat_') === 0 ||
				itemId.indexOf('harvest_') === 0 ||
				itemId === 'wt_beast_bone' ||
				itemId === 'wt_down_feather' ||
				itemId === 'shanhai_fresh_fish' ||
				itemId === 'shanhai_beast_meat' ||
				it.item_type === 'material' ||
				it.item_type === 'raw_material';
		}

		var bagItems = (lastInventoryMsg && Array.isArray(lastInventoryMsg.items)) ? lastInventoryMsg.items : [];
		var rawMaterials = bagItems.filter(isRawMaterialItem);

		var craftBodyHtml = '';
		if (rawMaterials.length > 0) {
			craftBodyHtml += '<div class="craft-materials-list" style="display:flex;flex-direction:column;gap:6px;margin-bottom:8px;">';
			for (var rIdx = 0; rIdx < rawMaterials.length; rIdx++) {
				var rm = rawMaterials[rIdx];
				var rmId = rm.item_id || rm.id || '';
				var rmRawName = rm.name || rmId;
				var rmName = formatItemName(rmRawName, rmId);
				var rmQty = Number(rm.qty) || 1;
				var craftLabel = rm.craft_label || '加工煉製';
				var btnLabel = craftLabel;
				craftBodyHtml += '<div class="craft-material-row" style="display:flex;justify-content:space-between;align-items:center;padding:4px 6px;background:rgba(0,0,0,0.03);border-radius:4px;">' +
					'<span>' + esc(rmName) + (rmQty > 1 ? ' <small style="opacity:0.8;">\u00d7' + rmQty + '</small>' : '') + '</span>' +
					'<button type="button" class="inv-action-btn craft-item-btn" data-action="craft_item" data-item-id="' + esc(rmId) + '">' + btnLabel + '</button>' +
					'</div>';
			}
			craftBodyHtml += '</div>';
		} else {
			craftBodyHtml += '<div style="margin-bottom:8px;font-size:0.9em;opacity:0.75;">行囊中暫無可煉製的原材</div>';
		}

		sectionCraft.innerHTML = '<div class="equip-section-title">【 手作與淬鍊 】</div>' +
			'<div class="equip-section-body craft-summary">' +
			craftBodyHtml +
			'</div>';
		wrap.appendChild(sectionCraft);

		sectionCraft.querySelectorAll('.craft-item-btn').forEach(function (btn) {
			btn.addEventListener('click', function (e) {
				e.stopPropagation();
				var targetItemId = this.getAttribute('data-item-id');
				_send({ type: 'craft_item', item_id: targetItemId });
			});
		});

		var craftAutoBtn = sectionCraft.querySelector('.craft-auto-btn');
		if (craftAutoBtn) {
			craftAutoBtn.addEventListener('click', function (e) {
				e.stopPropagation();
				_send({ type: 'craft_item', item_id: 'auto' });
			});
		}
	}
	var activeStarplateDrawer = 'martial';
	var activeMartialSkillIdx = 0;
	var activeCraftSkillIdx = 0;
	var activeStarplateHubIdx = -1;

	function syncStarplateDrawers() {
		var pane = document.getElementById('player-modal-pane-starplate');
		if (!pane) return;
		pane.querySelectorAll('.sw-accordion-drawer').forEach(function (drawer) {
			var key = drawer.getAttribute('data-drawer');
			var isOpen = key === activeStarplateDrawer;
			drawer.classList.toggle('is-open', isOpen);
			var arrow = drawer.querySelector('.sw-accordion-arrow');
			if (arrow) arrow.textContent = isOpen ? '▾' : '▸';
			var body = drawer.querySelector('.sw-accordion-body');
			if (body) body.hidden = !isOpen;
		});
	}

	function bindDrawerHeader(drawer, key) {
		var hdr = drawer.querySelector('.sw-accordion-header');
		if (!hdr) return;
		hdr.addEventListener('click', function () {
			activeStarplateDrawer = (activeStarplateDrawer === key) ? '' : key;
			syncStarplateDrawers();
		});
	}

	function canTranscribeManual() {
		if (lastInventoryMsg && Array.isArray(lastInventoryMsg.items)) {
			for (var k = 0; k < lastInventoryMsg.items.length; k++) {
				var it = lastInventoryMsg.items[k];
				if (!it) continue;
				if (it.item_id === 'blank_manual' || it.item_id === 'wt_blank_manual' || it.item_id === 'manual_blank' || it.item_type === 'blank_manual' || (it.name && (it.name.indexOf('空白圖譜') !== -1 || it.name.indexOf('無字殘卷') !== -1 || it.name.indexOf('空白書冊') !== -1 || it.name.indexOf('空白手札') !== -1))) {
					return true;
				}
			}
		}
		var st = (window.gameState ? window.gameState() : null) || {};
		var objs = st.objects || [];
		for (var j = 0; j < objs.length; j++) {
			var obj = objs[j];
			if (!obj) continue;
			if (obj.category_tag === 'desk' || obj.tag === 'desk' || (Array.isArray(obj.tags) && obj.tags.indexOf('desk') !== -1)) {
				return true;
			}
			var oName = obj.name || obj.id || '';
			if (oName.indexOf('書案') !== -1 || oName.indexOf('書桌') !== -1 || oName.indexOf('案几') !== -1 || oName.indexOf('筆墨') !== -1) {
				return true;
			}
		}
		return false;
	}

	function renderSkillPane(msg) {
		var wrap = document.getElementById('skill-content');
		if (!wrap) return;
		var skills = (msg && Array.isArray(msg.natural_skills)) ? msg.natural_skills : [];
		var html = '<div class="skill-section">';
		if (!skills.length) {
			html += '<div class="sw-accordion-drawer is-open" data-drawer="martial">';
			html += '<div class="sw-accordion-header"><span class="sw-accordion-title"><span class="sw-accordion-arrow">▾</span>【內息與臨陣身手】</span><span class="sw-accordion-badge">初凝</span></div>';
			html += '<div class="sw-accordion-body"><div class="skill-placeholder">氣脈初凝，隨日常採獵、百工與吐納漸次成形。</div></div>';
			html += '</div>';
		} else {
			var breathItem = skills.find(function (s) {
				return s.category === 'inner_breath' || (s.category_label || '').indexOf('內息底色') !== -1;
			});
			var martialMoves = skills.filter(function (s) { return s.domain === 'martial'; });
			var martial = breathItem ? [breathItem].concat(martialMoves) : martialMoves;
			var prod = skills.filter(function (s) { return s.domain === 'production'; });
			if (martial.length) {
				if (activeMartialSkillIdx >= martial.length) activeMartialSkillIdx = 0;
				var martialMoveCount = martialMoves.length;
				var martialBadge = (breathItem ? esc(breathItem.name) + '・' : '') + martialMoveCount + ' 式';
				var isMartialOpen = activeStarplateDrawer === 'martial';
				html += '<div class="sw-accordion-drawer' + (isMartialOpen ? ' is-open' : '') + '" data-drawer="martial">';
				html += '<div class="sw-accordion-header">';
				html += '<span class="sw-accordion-title"><span class="sw-accordion-arrow">' + (isMartialOpen ? '▾' : '▸') + '</span>【內息與臨陣身手】</span>';
				html += '<span class="sw-accordion-badge">' + martialBadge + '</span>';
				html += '</div>';
				html += '<div class="sw-accordion-body"' + (isMartialOpen ? '' : ' hidden') + '>';
				for (var i = 0; i < martial.length; i++) {
					var ms = martial[i];
					var isBreathHabit = ms.category === 'inner_breath' || (ms.category_label || '').indexOf('內息底色') !== -1;
					var slotKey = (ms.category === 'opening' || ms.category === 'midgame' || ms.category === 'finisher' || ms.category === 'defense') ? ms.category : 'opening';
					var isSubOpen = (i === activeMartialSkillIdx);
					html += '<div class="sw-sub-accordion-card' + (isSubOpen ? ' is-open' : '') + '" data-sub-group="martial" data-sub-idx="' + i + '">';
					html += '<div class="sw-sub-accordion-header">';
					html += '<div class="sw-sub-accordion-main"><span class="sw-sub-arrow">' + (isSubOpen ? '▾' : '▸') + '</span> <strong>◆ ' + esc(ms.name) + '</strong> <span class="text-muted">・' + esc((ms.category_label || '').replace(/^[【\[]|[\】\]]$/g, '')) + '</span></div>';
					var showTranscribe = !isBreathHabit && canTranscribeManual();
					if (showTranscribe) {
						html += '<button type="button" class="transcribe-manual-btn" data-slot="' + slotKey + '">著譜立說</button>';
					}
					html += '</div>';
					html += '<div class="sw-sub-accordion-body"' + (isSubOpen ? '' : ' hidden') + '>';
					html += '<div class="sw-sub-basis">' + esc(ms.origin_basis) + '</div>';
					if (ms.learned_and_familiar) {
						html += '<div class="text-muted sw-sub-familiar">' + esc(ms.learned_and_familiar) + '</div>';
					}
					if (ms.auto_trigger_desc) {
						html += '<div class="sw-sub-effect">' + (isBreathHabit ? '行氣與實效：' : '臨陣實效：') + esc(ms.auto_trigger_desc) + '</div>';
					}
					html += '</div></div>';
				}
				html += '</div></div>';
			}
			if (prod.length) {
				if (activeCraftSkillIdx >= prod.length) activeCraftSkillIdx = 0;
				var isCraftOpen = activeStarplateDrawer === 'craft';
				var craftBadge = '百工 ' + prod.length + ' 藝';
				html += '<div class="sw-accordion-drawer' + (isCraftOpen ? ' is-open' : '') + '" data-drawer="craft">';
				html += '<div class="sw-accordion-header">';
				html += '<span class="sw-accordion-title"><span class="sw-accordion-arrow">' + (isCraftOpen ? '▾' : '▸') + '</span>【百工造物手藝】</span>';
				html += '<span class="sw-accordion-badge">' + craftBadge + '</span>';
				html += '</div>';
				html += '<div class="sw-accordion-body"' + (isCraftOpen ? '' : ' hidden') + '>';
				for (var j = 0; j < prod.length; j++) {
					var ps = prod[j];
					var isProdSubOpen = (j === activeCraftSkillIdx);
					html += '<div class="sw-sub-accordion-card' + (isProdSubOpen ? ' is-open' : '') + '" data-sub-group="craft" data-sub-idx="' + j + '">';
					html += '<div class="sw-sub-accordion-header">';
					html += '<div class="sw-sub-accordion-main"><span class="sw-sub-arrow">' + (isProdSubOpen ? '▾' : '▸') + '</span> <strong>◇ ' + esc(ps.name) + '</strong> <span class="text-muted">・' + esc((ps.category_label || '').replace(/^[【\[]|[\】\]]$/g, '')) + '</span></div>';
					html += '</div>';
					html += '<div class="sw-sub-accordion-body"' + (isProdSubOpen ? '' : ' hidden') + '>';
					html += '<div class="sw-sub-basis">' + esc(ps.origin_basis) + '</div>';
					if (ps.learned_and_familiar) {
						html += '<div class="text-muted sw-sub-familiar">' + esc(ps.learned_and_familiar) + '</div>';
					}
					if (ps.auto_trigger_desc) {
						html += '<div class="sw-sub-effect">百工實效：' + esc(ps.auto_trigger_desc) + '</div>';
					}
					html += '</div></div>';
				}
				html += '</div></div>';
			}
		}
		html += '</div>';
		wrap.innerHTML = html;
		wrap.querySelectorAll('.sw-accordion-drawer').forEach(function (drawer) {
			bindDrawerHeader(drawer, drawer.getAttribute('data-drawer'));
		});
		wrap.querySelectorAll('.sw-sub-accordion-card').forEach(function (card) {
			var hdr = card.querySelector('.sw-sub-accordion-header');
			if (!hdr) return;
			hdr.addEventListener('click', function (e) {
				if (e.target.closest('.transcribe-manual-btn')) return;
				var group = card.getAttribute('data-sub-group');
				var idx = parseInt(card.getAttribute('data-sub-idx'), 10);
				if (group === 'martial') {
					activeMartialSkillIdx = (activeMartialSkillIdx === idx) ? -1 : idx;
				} else if (group === 'craft') {
					activeCraftSkillIdx = (activeCraftSkillIdx === idx) ? -1 : idx;
				}
				var activeIdx = (group === 'martial') ? activeMartialSkillIdx : activeCraftSkillIdx;
				wrap.querySelectorAll('.sw-sub-accordion-card[data-sub-group="' + group + '"]').forEach(function (c) {
					var cIdx = parseInt(c.getAttribute('data-sub-idx'), 10);
					var open = (cIdx === activeIdx);
					c.classList.toggle('is-open', open);
					var arr = c.querySelector('.sw-sub-arrow');
					if (arr) arr.textContent = open ? '▾' : '▸';
					var body = c.querySelector('.sw-sub-accordion-body');
					if (body) body.hidden = !open;
				});
			});
		});
		wrap.querySelectorAll('.transcribe-manual-btn').forEach(function (btn) {
			btn.addEventListener('click', function (e) {
				e.stopPropagation();
				var slot = this.getAttribute('data-slot');
				_send({ type: 'inventory_action', action: 'TranscribeManual', item_id: slot });
			});
		});
	}


	window.onTacticalState = function () {};
	window.openTacticsModal = function () {
		activeStarplateDrawer = 'martial';
		var modalOverlay = document.getElementById('player-modal-overlay');
		if (modalOverlay) modalOverlay.hidden = false;
		var tabBtn = document.querySelector('.player-modal-tab[data-tab="starplate"]');
		if (tabBtn) tabBtn.click();
		syncStarplateDrawers();
	};

	function renderStarplatePane(me) {
		var wrap = document.getElementById('starplate-content');
		if (!wrap) return;

		wrap.innerHTML = '';

		var st = (window.gameState && window.gameState().me) || (lastStatusMsg && lastStatusMsg.is_self ? lastStatusMsg : null);
		var currentMe = me || st || {};
		var skills = (currentMe && Array.isArray(currentMe.natural_skills)) ? currentMe.natural_skills : ((lastStatusMsg && Array.isArray(lastStatusMsg.natural_skills)) ? lastStatusMsg.natural_skills : []);

		var boneSkill = null;
		var masteryCards = [];
		for (var k = 0; k < skills.length; k++) {
			var sk = skills[k];
			if (!sk) continue;
			if (sk.category === 'innate_bone' || (sk.category_label && sk.category_label.indexOf('天生體魄') !== -1)) {
				boneSkill = sk;
			} else if (sk.domain === 'mastery_ledger') {
				masteryCards.push(sk);
			}
		}

		var boneName = (boneSkill ? boneSkill.name : '')
			|| (currentMe && (currentMe.bone_structure || currentMe.innate_phase))
			|| '身架勻稱・發力順暢';

		var boneDesc = (boneSkill && boneSkill.origin_basis)
			|| '身形平實沉著，骨肉勻亭，動靜轉換之間行氣順暢，不偏不倚。';

		var boneFluency = (boneSkill && boneSkill.learned_and_familiar) || '';

		var isBoneOpen = activeStarplateDrawer === 'bone_heritage';
		var drawer = document.createElement('div');
		drawer.className = 'sw-accordion-drawer' + (isBoneOpen ? ' is-open' : '');
		drawer.setAttribute('data-drawer', 'bone_heritage');

		var drawerHeader = document.createElement('div');
		drawerHeader.className = 'sw-accordion-header';
		drawerHeader.innerHTML = '<span class="sw-accordion-title"><span class="sw-accordion-arrow">' + (isBoneOpen ? '▾' : '▸') + '</span>【體魄底蘊與所學所歷】</span>' +
			'<span class="sw-accordion-badge">體魄・所學・所歷</span>';
		drawer.appendChild(drawerHeader);

		var drawerBody = document.createElement('div');
		drawerBody.className = 'sw-accordion-body';
		drawerBody.hidden = !isBoneOpen;

		var cardHtml = '<div class="sw-sub-accordion-card is-open" style="margin-bottom:10px;">' +
			'<div class="sw-sub-accordion-header">' +
			'<div class="sw-sub-accordion-main"><strong>◇ ' + esc(boneName) + '</strong> <span class="text-muted">・天生體魄</span></div>' +
			'</div>' +
			'<div class="sw-sub-accordion-body">' +
			'<div class="sw-sub-basis">' + esc(boneDesc) + '</div>' +
			(boneFluency ? '<div class="text-muted sw-sub-familiar">' + esc(boneFluency) + '</div>' : '') +
			'</div></div>';

		for (var mIdx = 0; mIdx < masteryCards.length; mIdx++) {
			var mc = masteryCards[mIdx];
			cardHtml += '<div class="sw-sub-accordion-card is-open" style="margin-bottom:10px;">' +
				'<div class="sw-sub-accordion-header">' +
				'<div class="sw-sub-accordion-main"><strong>◇ ' + esc(mc.name) + '</strong> <span class="text-muted">・' + esc((mc.category_label || '').replace(/^[【\[]|[\】\]]$/g, '')) + '</span></div>' +
				'</div>' +
				'<div class="sw-sub-accordion-body">' +
				'<div class="sw-sub-basis">' + esc(mc.origin_basis) + '</div>' +
				(mc.learned_and_familiar ? '<div class="text-muted sw-sub-familiar">' + esc(mc.learned_and_familiar) + '</div>' : '') +
				(mc.auto_trigger_desc ? '<div class="sw-sub-effect">' + esc(mc.auto_trigger_desc) + '</div>' : '') +
				'</div></div>';
		}

		cardHtml += '<div style="padding:10px 12px;background:var(--paper-light, rgba(255,255,255,0.65));border:1px dashed var(--sw-btn-border, rgba(112,79,36,0.25));border-radius:6px;font-size:12.5px;line-height:1.6;color:var(--ink-light, #5c4832);">' +
			'「一身技藝皆由平日所學、器物磨合與山海行旅自然養成，臨陣交手與百工勞作時隨手施展。」' +
			'</div>';
		drawerBody.innerHTML = cardHtml;
		drawer.appendChild(drawerBody);
		bindDrawerHeader(drawer, 'bone_heritage');
		wrap.appendChild(drawer);
	}

	function describeItemWeight(subTotal) {
		var w = Number(subTotal);
		if (isNaN(w) || w <= 0) return '輕巧';
		if (w < 1.0) return '輕巧';
		if (w < 3.0) return '適手';
		if (w < 8.0) return '微沉';
		return '沉重';
	}
	var _lastInventoryItemIds = [];
	var _lastInvFeedback = null;

	function showInventoryFeedback(text, isError) {
		var clean = String(text || '').replace(/<[^>]*>/g, '').trim();
		if (!clean) return;
		_lastInvFeedback = { text: clean, isError: !!isError, ts: Date.now() };
		var listEl = document.getElementById('inventory-list');
		if (!listEl) return;
		var existing = document.getElementById('sw-inv-feedback-banner');
		if (!existing) {
			existing = document.createElement('div');
			existing.id = 'sw-inv-feedback-banner';
			existing.className = 'sw-inv-feedback-banner';
			listEl.insertBefore(existing, listEl.firstChild);
		}
		var fbBg = isError ? 'rgba(158, 42, 43, 0.10)' : 'rgba(45, 106, 79, 0.10)';
		var fbBorder = isError ? 'rgba(158, 42, 43, 0.45)' : 'rgba(45, 106, 79, 0.42)';
		var fbColor = isError ? '#8c2324' : '#1b4332';
		existing.style.cssText = 'margin:0 0 10px 0;padding:8px 12px;border-radius:6px;background:' + fbBg + ';border:1px solid ' + fbBorder + ';color:' + fbColor + ';font-size:13.5px;line-height:1.5;';
		existing.textContent = clean;
	}

	function renderInventoryContent(msg) {
		lastInventoryMsg = msg;
		_lastInventoryItemIds = (msg && Array.isArray(msg.items))
			? msg.items.map(function (it) { return it && (it.item_id || it.id) ? String(it.item_id || it.id) : ''; }).filter(Boolean)
			: [];
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
			if (_lastInvFeedback && (Date.now() - _lastInvFeedback.ts < 25000)) {
				showInventoryFeedback(_lastInvFeedback.text, _lastInvFeedback.isError);
			}
			return;
		}
		listEl.innerHTML = '';
		if (_lastInvFeedback && (Date.now() - _lastInvFeedback.ts < 25000)) {
			showInventoryFeedback(_lastInvFeedback.text, _lastInvFeedback.isError);
		}
		for (var i = 0; i < msg.items.length; i++) {
			(function (it) {
				var rawId = String(it.item_id || '');
				if (rawId.indexOf('crystal_') === 0 || it.item_type === 'word_crystal' || /crystal|\u8a5e\u5143\u7d50\u6676/.test(String(it.name || ''))) return;
				var itemId = it.item_id || it.id || '';
				var itemName = it.name || '';
				var itemStage = it.item_stage || (itemId.indexOf('mut_') === 0 ? 'mutated' : (it.item_type === 'material' || it.item_type === 'raw_material' ? 'raw' : 'finished'));
				var isMutated = itemStage === 'mutated';
				var isRaw = itemStage === 'raw';
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
					var isCorpse = it.item_type === 'corpse' || /^corpse_/.test(it.item_id || '');
					var isBlankManual = it.item_id === 'blank_manual' || it.item_id === 'wt_blank_manual';
					var isManual = it.item_type === 'manual' || isBlankManual || /^manual_/.test(it.item_id || '');
					if (it.slot) {
						if (it.slot === 'hold' || it.slot === 'hold_r' || it.slot === 'hold_l') {
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip" data-target="hold_l">\u5de6\u624b</button>';
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip" data-target="hold_r">\u53f3\u624b</button>';
						} else {
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip">\u7a7f\u6234</button>';
						}
					} else if (isCorpse) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="butcher-corpse">肢體分解</button>';
					} else if (isBlankManual) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="transcribe-manual">著譜立說</button>';
					} else if (isManual) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="study-manual">研讀參悟</button>';
					} else if (!isRaw) {
						var isFoodOrDrink = Boolean(it.is_edible) || it.item_type === 'food' || it.item_type === 'consumable';
						var useLabel = isFoodOrDrink ? '食用' : '使用';
						if (itemId === 'zl_item_bamboo_canteen_full' || itemId === 'zl_item_bamboo_tube_soup') {
							useLabel = '飲用';
						} else if (itemId === 'zl_item_bamboo_shoot') {
							useLabel = '生啃';
						}
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="use">' + useLabel + '</button>';
					}
					if (Boolean(it.can_craft) || isRaw) {
						if (Boolean(it.is_edible)) {
							var rawEatLabel = '食用';
							if (itemId === 'zl_item_bamboo_canteen_full' || itemId === 'zl_item_bamboo_tube_soup') {
								rawEatLabel = '飲用';
							} else if (itemId === 'zl_item_bamboo_shoot') {
								rawEatLabel = '生啃';
							}
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="use">' + rawEatLabel + '</button>';
						}
						var craftLabel = it.craft_label || '加工煉製';
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="craft">' + esc(craftLabel) + '</button>';
					}
					var curPlaceName = String((window.SwGrid && window.SwGrid.mapData && window.SwGrid.mapData.currentRoomName) || (document.getElementById('sw-place-name') && document.getElementById('sw-place-name').textContent) || '');
					var isMarketVenue = /(行|鋪|肆|工坊|染坊|織坊|木坊|鐵坊|茶坊|酒坊|磨坊|店|樓|食堂|市集|攤|館|閣|局|場|廠|塢|站|社)/.test(curPlaceName);
					if (isMarketVenue) {
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="sell" data-sell-mode="one">售出</button>';
						if (it.qty && it.qty > 1) {
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="sell" data-sell-mode="all">全部售出</button>';
						}
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
					expand.querySelectorAll('.inv-action-btn[data-action="butcher-corpse"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'use_item', item_id: it.item_id, action: 'ButcherCorpse' });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="transcribe-manual"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'use_item', item_id: 'opening', action: 'TranscribeManual' });
						});
					});
					expand.querySelectorAll('.inv-action-btn[data-action="study-manual"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'use_item', item_id: it.item_id, action: 'StudyManual' });
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
		var savedOrKey = '';
		try {
			savedOrKey = (localStorage.getItem('sw_openrouter_api_key') || '').trim();
		} catch (e) {}
		var hasOrKey = !!(savedOrKey || window.__hasOpenRouterKey);
		var orModel = window.__openRouterModel || 'qwen/qwen3.5-flash-02-23';

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

		// 2. 對話模型與 OpenRouter 金鑰
		html += '<div class="settings-section">';
		html += '<div class="settings-section-title">【 對話模型與 OpenRouter 金鑰 】</div>';
		if (hasOrKey) {
			html += '<p id="setting-openrouter-status" class="settings-desc settings-llm-status is-openrouter">目前生效：<strong>OpenRouter 雲端模型（' + esc(orModel) + '）</strong></p>';
		} else {
			html += '<p id="setting-openrouter-status" class="settings-desc settings-llm-status is-local">目前生效：<strong>本機模型（Gemma-4-26B-A4B）</strong></p>';
		}
		html += '<p class="settings-desc">貼上 OpenRouter API Key（以 *** 遮罩保護）即可將玩家對談切換至雲端推薦模型；未填或清除則使用本機 26B-A4B 模型：</p>';
		html += '<div class="settings-key-row">';
		html += '<input type="password" id="setting-openrouter-key" class="settings-key-input" autocomplete="new-password" spellcheck="false" placeholder="貼上 sk-or-v1-...（以 *** 遮罩顯示）" value="' + (hasOrKey ? '************' : '') + '">';
		html += '</div>';
		html += '<div class="settings-key-actions">';
		html += '<button type="button" id="setting-btn-save-or-key" class="settings-act-btn settings-btn-primary">儲存金鑰</button>';
		html += '<button type="button" id="setting-btn-clear-or-key" class="settings-act-btn">清除並改回本機模型</button>';
		html += '</div>';
		html += '</div>';

		// 3. 輿圖預設視野
		html += '<div class="settings-section">';
		html += '<div class="settings-section-title">【 輿圖視野偏好 】</div>';
		html += '<label class="settings-checkbox-label">';
		html += '<input type="checkbox" id="setting-map-expand"' + (defaultMapExpand ? ' checked' : '') + '> 登入時保持大地圖展開';
		html += '</label>';
		html += '</div>';

		// 4. 敘事歷程管理
		html += '<div class="settings-section">';
		html += '<div class="settings-section-title">【 敘事動態管理 】</div>';
		html += '<p class="settings-desc">清除當前畫面累積的歷程動態，重整視野：</p>';
		html += '<button type="button" id="setting-btn-clear-log" class="settings-act-btn">清除當前動態紀錄</button>';
		html += '</div>';

		// 5. 角色與帳號
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

		var orKeyInput = wrap.querySelector('#setting-openrouter-key');
		var saveOrBtn = wrap.querySelector('#setting-btn-save-or-key');
		var clearOrBtn = wrap.querySelector('#setting-btn-clear-or-key');
		if (orKeyInput) {
			orKeyInput.addEventListener('focus', function () {
				if (orKeyInput.value === '************') {
					orKeyInput.value = '';
				}
			});
			orKeyInput.addEventListener('blur', function () {
				if (!orKeyInput.value.trim() && (window.__hasOpenRouterKey || localStorage.getItem('sw_openrouter_api_key'))) {
					orKeyInput.value = '************';
				}
			});
		}
		if (saveOrBtn) {
			saveOrBtn.addEventListener('click', function () {
				var rawVal = orKeyInput ? orKeyInput.value.trim() : '';
				if (rawVal === '************') {
					var existing = '';
					try { existing = (localStorage.getItem('sw_openrouter_api_key') || '').trim(); } catch (e) {}
					if (existing) {
						_send({ type: 'set_openrouter_key', player_input: existing, action: 'save' });
					}
					return;
				}
				if (!rawVal) {
					try { localStorage.removeItem('sw_openrouter_api_key'); } catch (e) {}
					window.__hasOpenRouterKey = false;
					_send({ type: 'set_openrouter_key', player_input: '', action: 'clear' });
					renderSettingsPane();
					return;
				}
				try { localStorage.setItem('sw_openrouter_api_key', rawVal); } catch (e) {}
				window.__hasOpenRouterKey = true;
				_send({ type: 'set_openrouter_key', player_input: rawVal, action: 'save' });
				renderSettingsPane();
			});
		}
		if (clearOrBtn) {
			clearOrBtn.addEventListener('click', function () {
				try { localStorage.removeItem('sw_openrouter_api_key'); } catch (e) {}
				window.__hasOpenRouterKey = false;
				_send({ type: 'set_openrouter_key', player_input: '', action: 'clear' });
				renderSettingsPane();
			});
		}

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
		updateTacticalState: function () {},
		renderTacticalSection: function () {},
		formatItemName: formatItemName,
		showInventoryFeedback: showInventoryFeedback,
		getInventoryItemIds: function () { return _lastInventoryItemIds.slice(); },
		ITEM_NAME_MAP: ITEM_NAME_MAP
	};
})();
