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

	function renderStatusPane(msg) {
		var wrap = document.getElementById('status-content');
		if (!wrap) return;
		var isSelf = msg.is_self === true;
		var vit = msg.vit != null ? msg.vit : '—';
		var qi = msg.qi != null ? msg.qi : '—';
		var dex = msg.dex != null ? msg.dex : '—';
		var title = (msg.display_title && msg.display_title.trim()) ? msg.display_title.trim() : '無名之輩';
		var origin = (msg.origin_sentence && msg.origin_sentence.trim()) ? msg.origin_sentence.trim() : '';
		var html = '<dl class="status-dl">';
		html += '<dt>[ 命途 ]</dt><dd>' + esc(title) + '</dd>';
		if (origin) {
			html += '<dt>[ 本源 ]</dt><dd>「' + esc(origin) + '」</dd>';
		}
		html += '<dt>[ 維度 ]</dt><dd>體質 ' + vit + ' ｜ 氣脈 ' + qi + ' ｜ 靈敏 ' + dex + '</dd>';
		html += '<dt>[ 四相 ]</dt><dd class="status-four">'
			+ '<span>氣血 ' + fmt(msg.hp_cur) + '/' + fmt(msg.hp_max) + '</span>'
			+ '<span>內力 ' + fmt(msg.inner_cur) + '/' + fmt(msg.inner_max) + '</span>'
			+ '<span>精神 ' + fmt(msg.spirit_cur) + '/' + fmt(msg.spirit_max) + '</span>'
			+ '<span>體力 ' + fmt(msg.stamina_cur) + '/' + fmt(msg.stamina_max) + '</span>'
			+ '</dd>';
		html += '<dt>[ 持有 ]</dt><dd>鎂：' + (isSelf && msg.magnesium != null ? msg.magnesium : '—') + '</dd>';
		html += '</dl>';
		wrap.innerHTML = html;
	}

	var EQUIP_SLOTS = [
		['head', '【冠盔】'], ['face', '【面甲】'], ['neck', '【護頸】'],
		['undershirt', '【襯衣】'], ['inner_armor', '【內甲】'], ['body', '【掛鎧】'], ['cloak', '【披風】'],
		['shoulder', '【肩鎧】'], ['arm', '【護臂】'], ['wrist', '【護腕】'], ['hand', '【掌套】'],
		['waist', '【腰鎧】'], ['legs', '【襯褲】'], ['leg_armor', '【腿鎧】'], ['feet', '【護靴】'],
		['ring_l', '【左指】'], ['ring_r', '【右指】'], ['trinket', '【佩掛】'],
		['hold_l', '【左持】'], ['hold_r', '【右持】']
	];
	var lastEquipMsg = null;

	function renderEquipmentPane(msg) {
		lastEquipMsg = msg;
		var wrap = document.getElementById('player-modal-pane-equip');
		if (!wrap) return;
		var names = msg.equipment_names || {};
		var slots = msg.equipment_slots || {};
		var descs = msg.equipment_descs || {};
		wrap.innerHTML = '';
		var dl = document.createElement('dl');
		dl.className = 'status-dl equip-dl';
		for (var i = 0; i < EQUIP_SLOTS.length; i++) {
			(function (code, label) {
				var dt = document.createElement('dt');
				dt.textContent = label;
				dl.appendChild(dt);

				var dd = document.createElement('dd');
				var itemName = names[code];
				var itemID = slots[code];
				if (itemName && itemID) {
					dd.className = 'equip-has-item';
					dd.innerHTML = '\u25b8 ' + esc(itemName);
					dd.addEventListener('click', function () {
						var existing = dl.querySelector('.equip-item-expand');
						var wasThis = existing && existing.getAttribute('data-slot') === code;
						if (existing) existing.remove();
						dl.querySelectorAll('dd.expanded').forEach(function (el) {
							el.classList.remove('expanded');
							el.innerHTML = el.innerHTML.replace('\u25be', '\u25b8');
						});
						if (wasThis) return;

						dd.classList.add('expanded');
						dd.innerHTML = dd.innerHTML.replace('\u25b8', '\u25be');
						var expand = document.createElement('div');
						expand.className = 'equip-item-expand';
						expand.setAttribute('data-slot', code);
						var desc = descs[code] || '';
						var descHtml = desc ? '<div class="equip-item-desc">\u2503 ' + esc(desc) + '</div>' : '';
						var actionsHtml = '<div class="inventory-item-actions">';
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="unequip">\u8131\u4e0b</button>';
						actionsHtml += '</div>';
						expand.innerHTML = descHtml + actionsHtml;

						dd.after(expand);

						expand.querySelector('.inv-action-btn[data-action="unequip"]').addEventListener('click', function (e) {
							e.stopPropagation();
							_send({ type: 'unequip_item', slot: code });
						});
					});
				} else {
					dd.innerHTML = '<span class="text-muted">(\u7a7a)</span>';
				}
				dl.appendChild(dd);
			})(EQUIP_SLOTS[i][0], EQUIP_SLOTS[i][1]);
		}
		wrap.appendChild(dl);
	}

	function renderSkillPane(msg) {
		var wrap = document.getElementById('skill-content');
		if (!wrap) return;
		var html = '<div class="skill-section">';
		html += '<div class="skill-section-title">【 運轉功法 】</div>';
		html += '<div class="skill-section-body">';
		html += '<div class="skill-placeholder">▣ 心法：<span class="text-muted">(未習得)</span></div>';
		html += '<div class="skill-divider"></div>';
		html += '<div class="skill-placeholder"><span class="text-muted">(尚無外功)</span></div>';
		html += '</div></div>';

		html += '<div class="skill-section">';
		html += '<div class="skill-section-title">【 實戰招式池 】</div>';
		html += '<div class="skill-section-body">';
		html += '<div class="skill-placeholder"><span class="text-muted">(無招式)</span></div>';
		html += '</div></div>';

		html += '<div class="skill-section">';
		html += '<div class="skill-section-title">【 語境推演 】</div>';
		html += '<div class="skill-section-body">';
		html += '<div class="skill-placeholder"><span class="text-muted">(無推演)</span></div>';
		html += '</div></div>';

		wrap.innerHTML = html;
	}

	// 二十主樞（361 規格 §2.2）：代碼 N001～N020、名稱
	var HUB_NAMES = ['天極', '脈衝', '震淵', '游離', '弦絲', '曜核', '凜晶', '淵流', '萬象', '解離', '鎮閾', '衡定', '穹壁', '重塑', '逆熵', '神淵', '識閾', '坍縮', '無相', '越權'];
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
		var val = cost != null ? ' (' + cost.toFixed(2) + ')' : '';
		return '<span class="' + label.css + '">' + label.text + val + '</span>';
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
	var GREEN_LABELS = ['G01 \u63a2', 'G02 \u89f8', 'G03 \u7d0d', 'G04 \u84c4', 'G05 \u6ffe', 'G06 \u6790', 'G07 \u878d', 'G08 \u884d', 'G09 \u5f8b', 'G10 \u675f', 'G11 \u91cb', 'G12 \u6563'];
	var GREEN_SHORT = ['G01','G02','G03','G04','G05','G06','G07','G08','G09','G10','G11','G12'];
	var GREEN_PER_BLUE = [[0, 1, 2], [2, 3, 4], [4, 5, 6], [7, 8, 9], [9, 10, 11]];
	var GREEN_SHARED = { 2: ' (\u8207[\u627f]\u5171\u7528)', 4: ' (\u8207[\u8f49]\u5171\u7528)', 9: ' (\u8207[\u5408]\u5171\u7528)' };

	function buildHubExpandHTML(hubIndex, costs) {
		var costA = getCostA(costs, hubIndex);
		var typeCCosts = getTypeCCosts(costs, hubIndex);
		var lines = '<div class="starplate-hub-cost">\u2503 \u62b5\u9054\u672c\u4e3b\u6a1e \u2500 ' + costSpan(costA) + '</div>';
		var idx = 0;
		for (var b = 0; b < 5; b++) {
			var costB = getCostB(costs, hubIndex, b);
			var nextBlue = BLUE_NAMES[(b + 1) % 5];
			var ringE = getCostE(costs, hubIndex, b);
			lines += '<div class="starplate-blue">\u2503 \ud83d\udd35 [' + BLUE_NAMES[b] + '] \u908f\u8f2f\u9598 \u2500 ' + costSpan(costB) + ' (\u672a\u8cab\u901a) <span class="ring-cost">\u27f3\u2192[' + nextBlue + '] ' + costSpan(ringE) + '</span></div>';
			for (var g = 0; g < 3; g++) {
				var greenIdx = GREEN_PER_BLUE[b][g];
				var costC = typeCCosts.length > idx ? typeCCosts[idx] : null;
				var ringD = getCostD(costs, hubIndex, greenIdx);
				var nextGreen = GREEN_SHORT[(greenIdx + 1) % 12];
				var shared = GREEN_SHARED[greenIdx] || '';
				lines += '<div class="starplate-green">\u2503 \u3000\u251c\u2500 \ud83d\udfe2 ' + GREEN_LABELS[greenIdx] + ' \u2500 ' + costSpan(costC) + '\uff1a[ \u7a7a ]' + shared + ' <span class="ring-cost">\u27f3\u2192' + nextGreen + ' ' + costSpan(ringD) + '</span></div>';
				idx++;
			}
		}
		return lines;
	}

	function renderStarplatePane(me) {
		var wrap = document.getElementById('starplate-content');
		if (!wrap) return;
		if (!me || !me.activated_nodes || !Array.isArray(me.activated_nodes)) {
			wrap.innerHTML = '<p class="text-muted">\u50c5\u81ea\u5df1\u53ef\u89c0\u770b\u661f\u76e4\u3002\u8acb\u5148\u767b\u5165\u4e26\u9ede\u64ca\u81ea\u5df1\u958b\u555f\u3002</p>';
			return;
		}
		var activated = me.activated_nodes;
		var count = activated.length;
		var costs = me.topology_costs || [];

		wrap.innerHTML = '';
		var header = document.createElement('div');
		header.className = 'starplate-block';
		header.innerHTML = '<strong>[ \u661f\u76e4\u8cab\u901a\u7387 ]</strong> ' + count + ' / 360';
		wrap.appendChild(header);

		var origin = document.createElement('div');
		origin.className = 'starplate-block';
		origin.innerHTML = '<strong>[ \u6e90\u59cb ]</strong> N000 \u751f\u4e4b\u5947\u9ede (\u5df2\u9ede\u4eae)';
		wrap.appendChild(origin);

		var hubTitle = document.createElement('div');
		hubTitle.className = 'starplate-block';
		hubTitle.innerHTML = '<strong>\u4e8c\u5341\u4e3b\u6a1e</strong>';
		wrap.appendChild(hubTitle);

		var list = document.createElement('ul');
		list.className = 'starplate-hub-list';
		wrap.appendChild(list);

		for (var i = 0; i < HUB_NAMES.length; i++) {
			(function (hubIndex) {
				var nodeId = 'N' + String(hubIndex + 1).padStart(3, '0');
				var costA = getCostA(costs, hubIndex);
				var adaptStr = costA != null ? costSpan(costA) : '\u672a\u77e5';

				var li = document.createElement('li');
				li.className = 'starplate-hub-row';
				li.setAttribute('data-hub', hubIndex);
				li.innerHTML = '\u25b8 [' + HUB_NAMES[hubIndex] + '] ' + nodeId + ' \uff5c \u9069\u6027\uff1a' + adaptStr + ' \uff5c \u8cab\u901a\uff1a0/17';
				list.appendChild(li);

				li.addEventListener('click', function () {
					var existing = list.querySelector('.starplate-hub-expand');
					var wasThis = existing && existing.getAttribute('data-hub') === String(hubIndex);
					if (existing) {
						var prevLi = existing.previousElementSibling;
						if (prevLi) {
							prevLi.classList.remove('expanded');
							prevLi.innerHTML = prevLi.innerHTML.replace('\u25be', '\u25b8');
						}
						existing.remove();
					}
					list.querySelectorAll('.starplate-hub-row.expanded').forEach(function (el) {
						el.classList.remove('expanded');
						el.innerHTML = el.innerHTML.replace('\u25be', '\u25b8');
					});
					if (wasThis) return;

					li.classList.add('expanded');
					li.innerHTML = li.innerHTML.replace('\u25b8', '\u25be');
					var expand = document.createElement('li');
					expand.className = 'starplate-hub-expand';
					expand.setAttribute('data-hub', hubIndex);
					expand.innerHTML = buildHubExpandHTML(hubIndex, costs);
					li.after(expand);
				});
			})(i);
		}
	}

	var lastInventoryMsg = null;

	function renderInventoryContent(msg) {
		lastInventoryMsg = msg;
		var weightEl = document.getElementById('inventory-weight');
		var listEl = document.getElementById('inventory-list');
		if (!weightEl || !listEl) return;
		var cur = msg.current_weight != null ? msg.current_weight.toFixed(1) : '0.0';
		var max = msg.max_weight != null ? msg.max_weight.toFixed(1) : '0.0';
		var overweight = msg.current_weight > msg.max_weight;
		weightEl.innerHTML = '負重：<span class="' + (overweight ? 'inventory-weight-warn' : '') + '">' + cur + ' / ' + max + '</span>';
		if (!msg.items || msg.items.length === 0) {
			listEl.innerHTML = '<div class="inventory-empty">（背包空空如也）</div>';
			return;
		}
		listEl.innerHTML = '';
		for (var i = 0; i < msg.items.length; i++) {
			(function (it) {
				var qtyStr = it.qty > 1 ? (' \u00d7' + it.qty) : '';
				var wStr = it.sub_total != null ? it.sub_total.toFixed(2) : '\u2014';
				var row = document.createElement('div');
				row.className = 'inventory-item';
				row.innerHTML = '<span class="inventory-item-name">\u25b8 ' + esc(it.name) + qtyStr + '</span>'
					+ '<span class="inventory-item-weight">(' + wStr + ')</span>';
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
					if (it.item_type === 'equipment' && it.slot) {
						if (it.slot === 'hold') {
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip" data-target="hold_l">\u5de6\u624b</button>';
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip" data-target="hold_r">\u53f3\u624b</button>';
						} else {
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip">\u7a7f\u6234</button>';
						}
					}
					actionsHtml += '<button type="button" class="inv-action-btn" data-action="drop" disabled>\u4e1f\u68c4</button>';
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
				});
			})(msg.items[i]);
		}
	}

	window.SwPanes = {
		renderStatusPane: renderStatusPane,
		renderEquipmentPane: renderEquipmentPane,
		renderSkillPane: renderSkillPane,
		renderStarplatePane: renderStarplatePane,
		renderInventoryContent: renderInventoryContent,
	};
})();
