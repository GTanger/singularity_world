// 遊戲時鐘與狀態條：gameSecNow / gameDaysNow / formatSingularityDate / updateGameTimeDisplay / ticker / updateStatusBars / draw。v0.20.46
(function () {
	var GAME_TIME_SCALE = 24;
	var GAME_SEC_PER_DAY = 86400;
	var DAYS_PER_YEAR = 365;
	var DAYS_PER_MONTH = 30;

	var gameTimeTicker = null;

	function gameSecNow() {
		var state = window.gameState ? window.gameState() : null;
		if (!state || !state.server_unix) return null;
		var elapsed = Math.max(0, (Date.now() / 1000) - state.server_unix);
		var sec = state.game_time_sec_since_midnight + elapsed * GAME_TIME_SCALE;
		sec = sec % GAME_SEC_PER_DAY;
		if (sec < 0) sec += GAME_SEC_PER_DAY;
		return sec;
	}

	function gameDaysNow() {
		var state = window.gameState ? window.gameState() : null;
		if (!state || !state.server_unix) return null;
		var elapsed = Math.max(0, (Date.now() / 1000) - state.server_unix);
		var secTotal = state.game_days_since_epoch * GAME_SEC_PER_DAY + state.game_time_sec_since_midnight + elapsed * GAME_TIME_SCALE;
		return Math.max(0, Math.floor(secTotal / GAME_SEC_PER_DAY));
	}

	function formatSingularityDate(days) {
		days = Math.max(0, Math.floor(days));
		var dayInYear = days % DAYS_PER_YEAR;
		var year = Math.floor(days / DAYS_PER_YEAR) + 1;
		var month = Math.min(12, Math.floor(dayInYear / DAYS_PER_MONTH) + 1);
		var day = (dayInYear % DAYS_PER_MONTH) + 1;
		if (day < 1) day = 1;
		if (day > 30) day = 30;
		var yearStr = year === 1 ? '元' : (year + '');
		return '奇點曆 ' + yearStr + '年' + month + '月' + day + '日';
	}

	function updateGameTimeDisplay() {
		var sec = gameSecNow();
		var days = gameDaysNow();
		var handEl = document.getElementById('game-time-hand');
		var clockEl = document.getElementById('game-time-clock');
		var labelEl = document.getElementById('game-time-label');
		var dateEl = document.getElementById('game-time-date');
		if (sec == null || days == null) {
			if (dateEl) dateEl.textContent = '奇點曆 —';
			if (clockEl) clockEl.textContent = '--:--';
			if (labelEl) labelEl.textContent = '';
			if (handEl) handEl.setAttribute('transform', 'rotate(0 16 16)');
			return;
		}
		if (handEl) {
			var hourCont = sec / 3600;
			var angle = (hourCont - 12) * 15;
			handEl.setAttribute('transform', 'rotate(' + angle + ' 16 16)');
		}
		if (dateEl) dateEl.textContent = formatSingularityDate(days);
		var h = Math.floor(sec / 3600) % 24;
		var m = Math.floor((sec % 3600) / 60);
		if (clockEl) clockEl.textContent = (h < 10 ? '0' : '') + h + ':' + (m < 10 ? '0' : '') + m;
		var phases = ['深夜','凌晨','破曉','清晨','上午','正午','午後','下午','傍晚','入夜','晚間','半夜'];
		var phase = phases[Math.floor(h / 2)] || '深夜';
		if (labelEl) labelEl.textContent = phase;
	}

	function startTicker() {
		if (gameTimeTicker) return;
		gameTimeTicker = setInterval(updateGameTimeDisplay, 500);
	}

	function stopTicker() {
		if (gameTimeTicker) {
			clearInterval(gameTimeTicker);
			gameTimeTicker = null;
		}
	}

	// 四條狀態欄：滿條＝該屬性最大值，條寬＝當前值/最大值*100%
	function updateStatusBars(hpCur, hpMax, innerCur, innerMax, spiritCur, spiritMax, staminaCur, staminaMax) {
		var pct = function (cur, max) {
			if (max == null || max <= 0) return 100;
			var c = Number(cur);
			var m = Number(max);
			return m <= 0 ? 100 : Math.min(100, Math.round((c / m) * 100));
		};
		var barHp = document.getElementById('bar-hp');
		var barSpirit = document.getElementById('bar-spirit');
		var barInner = document.getElementById('bar-inner');
		var barStamina = document.getElementById('bar-stamina');
		if (barHp) barHp.style.width = pct(hpCur, hpMax) + '%';
		if (barSpirit) barSpirit.style.width = pct(spiritCur, spiritMax) + '%';
		if (barInner) barInner.style.width = pct(innerCur, innerMax) + '%';
		if (barStamina) barStamina.style.width = pct(staminaCur, staminaMax) + '%';
	}

	// draw：刷新地圖視圖 + 姓名 + 狀態條（讀 window.gameState()）
	function draw() {
		var state = window.gameState ? window.gameState() : null;
		if (!state) return;
		if (window.mudUpdateRoomView) {
			window.mudUpdateRoomView(state.room_name, state.description, state.exits, state.entities, state.me, state.objects);
		}
		if (window.mudRenderExitButtons && document.getElementById('exits-buttons')) {
			window.mudRenderExitButtons('exits-buttons', state.exits, function (direction) {
				if (window.gameSendMoveDirection) window.gameSendMoveDirection(direction);
			});
		}
		var nameEl = document.getElementById('player-name');
		if (nameEl) nameEl.textContent = (state.me && state.me.player_id) ? state.me.player_id : '姓名';
		if (state.me && state.me.hp_max != null) {
			updateStatusBars(state.me.hp_cur, state.me.hp_max, state.me.inner_cur, state.me.inner_max, state.me.spirit_cur, state.me.spirit_max, state.me.stamina_cur, state.me.stamina_max);
		}
	}

	window.SwClock = {
		startTicker: startTicker,
		stopTicker: stopTicker,
		updateStatusBars: updateStatusBars,
		draw: draw,
		updateDisplay: updateGameTimeDisplay,
		formatDate: formatSingularityDate,
		gameSecNow: gameSecNow,
		gameDaysNow: gameDaysNow
	};
	// 向後相容 alias
	window.gameStartTimeTicker = startTicker;
	window.gameUpdateTime = updateGameTimeDisplay;
})();
