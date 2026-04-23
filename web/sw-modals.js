// 背包 Modal + 角色 Modal 開關邏輯；自包含，依賴 window.gameState / window.gameSend / window.gameIsConnected。v0.20.44
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
		if (!overlay || !modal || !playerName) return;

		function openModal(displayName, entityId) {
			if (titleEl) titleEl.textContent = (displayName && displayName.trim()) ? displayName.trim() : '角色';
			var st = window.gameState ? window.gameState() : {};
			var id = (entityId && entityId.trim()) ? entityId.trim() : (st.me && st.me.player_id ? st.me.player_id : '');
			var starplateWrap = document.getElementById('starplate-content');
			if (starplateWrap && id && st.me && st.me.player_id && id !== st.me.player_id) {
				starplateWrap.innerHTML = '<p class="text-muted">僅自己可觀看星盤。</p>';
			}
			if (id && window.gameIsConnected && window.gameIsConnected() && window.gameSend) {
				window.gameSend({ type: 'get_entity_status', entity_id: id });
			} else {
				var wrap = document.getElementById('status-content');
				if (wrap) wrap.innerHTML = '<p class="text-muted">請先登入</p>';
			}
			if (logEl) {
				var h = logEl.clientHeight;
				if (h > 0) modal.style.height = h + 'px';
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
		playerName.addEventListener('click', function () {
			var st = window.gameState ? window.gameState() : {};
			var myId = st.me && st.me.player_id;
			openModal(playerName.textContent || myId || '', myId || '');
		});
		playerName.addEventListener('keydown', function (e) {
			if (e.key === 'Enter' || e.key === ' ') {
				e.preventDefault();
				var st = window.gameState ? window.gameState() : {};
				var myId = st.me && st.me.player_id;
				openModal(playerName.textContent || myId || '', myId || '');
			}
		});
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
			});
		});
		window.openCharacterModal = openModal;
	}

	window.initInventoryModal = initInventoryModal;
	window.initPlayerModal = initPlayerModal;
})();
