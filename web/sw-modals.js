// 背包 Modal + 角色 Modal 啟閉邏輯；自包含，依賴 window.gameState / window.gameSend / window.gameIsConnected。v0.20.44
(function () {
	function initInventoryModal() {
		var overlay = document.getElementById('inventory-modal-overlay');
		var closeBtn = document.getElementById('inventory-modal-close');
		var openBtn = document.getElementById('btn-inventory');
		if (!overlay || !openBtn) return;

		function openInventory() {
			var st = window.gameState ? window.gameState() : {};
			if (!st.me) return;
			overlay.removeAttribute('hidden');
			overlay.setAttribute('aria-hidden', 'false');
			document.body.style.overflow = 'hidden';
			if (closeBtn) closeBtn.focus();
			document.addEventListener('keydown', onInvKeydown);
			if (window.gameIsConnected && window.gameIsConnected() && window.gameSend) {
				window.gameSend({ type: 'get_inventory' });
			}
		}
		function closeInventory() {
			overlay.setAttribute('hidden', '');
			overlay.setAttribute('aria-hidden', 'true');
			document.body.style.overflow = '';
			document.removeEventListener('keydown', onInvKeydown);
		}
		function onInvKeydown(e) {
			if (e.key === 'Escape') closeInventory();
		}
		openBtn.addEventListener('click', openInventory);
		if (closeBtn) closeBtn.addEventListener('click', closeInventory);
		overlay.addEventListener('click', function (e) {
			if (e.target === overlay) closeInventory();
		});
	}

	function initPlayerModal() {
		var overlay = document.getElementById('player-modal-overlay');
		var modal = document.getElementById('player-modal');
		var titleEl = document.getElementById('player-modal-title');
		var playerName = document.getElementById('player-name');
		var closeBtn = document.getElementById('player-modal-close');
		var logEl = document.querySelector('.log-content');
		var tabs = document.querySelectorAll('.player-modal-tab');
		var panes = document.querySelectorAll('.player-modal-pane');
		var currentEntityId = '';
		var currentIsSelf = false;
		if (!overlay || !modal || !playerName) return;

		function openModal(displayName, entityId, defaultTab) {
			var activeTabName = defaultTab || 'status';
			if (titleEl) titleEl.textContent = (displayName && displayName.trim()) ? displayName.trim() : '角色';
			var st = window.gameState ? window.gameState() : {};
			var id = (entityId && entityId.trim()) ? entityId.trim() : (st.me && st.me.player_id ? st.me.player_id : '');
			currentEntityId = id;
			currentIsSelf = !!(id && st.me && st.me.player_id && id === st.me.player_id);
			var starplateWrap = document.getElementById('starplate-content');
			if (starplateWrap && id && !currentIsSelf) {
				starplateWrap.innerHTML = '<p class="text-muted">僅自己可查看自身技能與歷練。</p>';
			}
			var skillWrap = document.getElementById('skill-content');
			if (skillWrap && !currentIsSelf) {
				skillWrap.innerHTML = '';
			}
			var invWeight = document.getElementById('inventory-weight');
			var invList = document.getElementById('inventory-list');
			if (!currentIsSelf) {
				if (invWeight) invWeight.innerHTML = '';
				if (invList) invList.innerHTML = '<div class="inventory-empty">僅自己可觀看背包。</div>';
			}
			if (id && window.gameIsConnected && window.gameIsConnected() && window.gameSend) {
				window.gameSend({ type: 'get_entity_status', entity_id: id });
				if (currentIsSelf && (activeTabName === 'inventory' || activeTabName === 'status')) {
					window.gameSend({ type: 'get_inventory' });
				}
			} else {
				var wrap = document.getElementById('status-content');
				if (wrap) wrap.innerHTML = '<p class="text-muted">請先登入</p>';
			}

			tabs.forEach(function (x) {
				var on = x.getAttribute('data-tab') === activeTabName;
				x.classList.toggle('active', on);
				x.setAttribute('aria-selected', on ? 'true' : 'false');
			});
			panes.forEach(function (p) {
				var on = p.getAttribute('data-tab') === activeTabName;
				p.classList.toggle('active', on);
				p.hidden = !on;
			});

			if (activeTabName === 'settings' && window.SwPanes && window.SwPanes.renderSettingsPane) {
				window.SwPanes.renderSettingsPane();
			} else if (activeTabName === 'equip' && window.SwPanes && window.SwPanes.renderEquipmentPane) {
				var eqMsg = window.lastEquipMsg || {};
				window.SwPanes.renderEquipmentPane(eqMsg);
			} else if (activeTabName === 'starplate' && window.SwPanes && window.SwPanes.renderStarplatePane) {
				window.SwPanes.renderStarplatePane(st.me || {});
			}

			overlay.removeAttribute('hidden');
			overlay.setAttribute('aria-hidden', 'false');
			document.body.style.overflow = 'hidden';
			closeBtn.focus();
			document.addEventListener('keydown', onModalKeydown);
		}
		function closeModal() {
			overlay.setAttribute('hidden', '');
			overlay.setAttribute('aria-hidden', 'true');
			document.body.style.overflow = '';
			document.removeEventListener('keydown', onModalKeydown);
			if (playerName) playerName.focus();
		}
		function onModalKeydown(e) {
			if (e.key === 'Escape') closeModal();
		}
		function handleSelfClick(e) {
			if (e && e.type === 'keydown' && e.key !== 'Enter' && e.key !== ' ') return;
			if (e && e.type === 'keydown') e.preventDefault();
			var st = window.gameState ? window.gameState() : {};
			var myId = st.me && st.me.player_id;
			openModal(playerName.textContent || myId || '', myId || '');
		}

		playerName.addEventListener('click', handleSelfClick);
		playerName.addEventListener('keydown', handleSelfClick);
		var playerVibe = document.getElementById('player-vibe');
		if (playerVibe) {
			playerVibe.addEventListener('click', handleSelfClick);
		}
		var openInvBtn = document.getElementById('btn-open-inventory');
		if (openInvBtn) {
			openInvBtn.addEventListener('click', function () {
				var st = window.gameState ? window.gameState() : {};
				var myId = st.me && st.me.player_id;
				openModal(playerName.textContent || myId || '', myId || '', 'inventory');
			});
		}
		var playerCore = document.querySelector('.grid-player-core');
		if (playerCore) {
			playerCore.addEventListener('click', function (e) {
				if (e.target === playerCore) {
					handleSelfClick(e);
				}
			});
		}
		closeBtn.addEventListener('click', closeModal);
		overlay.addEventListener('click', function (e) {
			if (e.target === overlay) closeModal();
		});
		tabs.forEach(function (tab) {
			tab.addEventListener('click', function () {
				var t = tab.getAttribute('data-tab');
				tabs.forEach(function (x) {
					x.classList.toggle('active', x === tab);
					x.setAttribute('aria-selected', x === tab ? 'true' : 'false');
				});
				panes.forEach(function (p) {
					var on = p.getAttribute('data-tab') === t;
					p.classList.toggle('active', on);
					p.hidden = !on;
				});
				if (t === 'inventory' && currentIsSelf && window.gameIsConnected && window.gameIsConnected() && window.gameSend) {
					window.gameSend({ type: 'get_inventory' });
				} else if (t === 'settings') {
					if (window.SwPanes && window.SwPanes.renderSettingsPane) {
						window.SwPanes.renderSettingsPane();
					}
				} else if (t === 'equip') {
					if (window.SwPanes && window.SwPanes.renderEquipmentPane) {
						var eqMsg = window.lastEquipMsg || {};
						window.SwPanes.renderEquipmentPane(eqMsg);
					}
				} else if (t === 'starplate') {
					if (window.SwPanes && window.SwPanes.renderStarplatePane) {
						var st = window.gameState ? window.gameState() : {};
						window.SwPanes.renderStarplatePane(st.me || {});
					}
				}
			});
		});
		window.openCharacterModal = openModal;
	}

	window.initInventoryModal = initInventoryModal;
	window.initPlayerModal = initPlayerModal;
})();
