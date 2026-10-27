// 遊戲時鐘與開譜曆日晷：gameSecNow / gameDaysNow / formatEpochSeasonDate / updateGameTimeDisplay / ticker / draw。
(function () {
	var GAME_TIME_SCALE = 24;
	var GAME_SEC_PER_DAY = 86400;
	var DAYS_PER_YEAR = 365;

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

	// 開譜曆與四季日期（春融、夏漲、秋狩、冬凝）
	function formatEpochSeasonDate(days) {
		var state = window.gameState ? window.gameState() : null;
		if (state && state.season_date_label) {
			return String(state.season_date_label);
		}
		days = Math.max(0, Math.floor(days || 0));
		var year = Math.floor(days / DAYS_PER_YEAR) + 1;
		var dayInYear = days % DAYS_PER_YEAR;
		var seasonName = '春融';
		var dayInSeason = dayInYear + 1;
		if (dayInYear < 91) {
			seasonName = '春融';
			dayInSeason = dayInYear + 1;
		} else if (dayInYear < 182) {
			seasonName = '夏漲';
			dayInSeason = dayInYear - 91 + 1;
		} else if (dayInYear < 273) {
			seasonName = '秋狩';
			dayInSeason = dayInYear - 182 + 1;
		} else {
			seasonName = '冬凝';
			dayInSeason = dayInYear - 273 + 1;
		}
		var yearStr = year === 1 ? '元' : String(year);
		return '開譜曆 ' + yearStr + ' 年・' + seasonName + '・' + dayInSeason + ' 日';
	}

	// 六段時辰：拂曉、晝前、正晝、黃昏、夜段、黎明前
	function getSixPhase(hour) {
		var h = Math.floor(hour);
		if (h >= 4 && h < 8) return '拂曉';
		if (h >= 8 && h < 12) return '晝前';
		if (h >= 12 && h < 16) return '正晝';
		if (h >= 16 && h < 20) return '黃昏';
		if (h >= 20 && h < 24) return '夜段';
		return '黎明前';
	}

	var WEATHER_NAMES = {
		'clear': '天朗',
		'rain': '微雨',
		'snow': '初雪',
		'wind': '勁風',
		'plum_rain': '梅雨',
		'fog': '薄霧',
		'storm': '雷雨'
	};

	function getCurrentWeatherName() {
		var state = window.gameState ? window.gameState() : null;
		var wt = (state && (state.weather || (state.room_view && state.room_view.weather))) ||
			(window.SwGrid && window.SwGrid.mapData && window.SwGrid.mapData.weather);
		if (!wt) return '天朗';
		if (typeof wt === 'string') return WEATHER_NAMES[wt] || wt;
		var k = wt.event_type || wt.icon_key || wt.name || 'clear';
		return WEATHER_NAMES[k] || wt.desc || '天朗';
	}

	function updateGameTimeDisplay() {
		var sec = gameSecNow();
		var days = gameDaysNow();
		var state = window.gameState ? window.gameState() : null;

		var calendarEl = document.getElementById('sw-top-calendar');
		var phasePlaceEl = document.getElementById('sw-top-phase-place');
		var weatherName = getCurrentWeatherName();

		function setPhasePlaceHtml(el, phaseWeatherText) {
			if (!el) return;
			el.textContent = '';
			var mainSpan = document.createElement('span');
			mainSpan.className = 'sw-top-phase-weather';
			mainSpan.textContent = phaseWeatherText;
			el.appendChild(mainSpan);
		}

		if (sec == null || days == null) {
			if (calendarEl) calendarEl.textContent = '開譜曆 元年・春融・1 日';
			setPhasePlaceHtml(phasePlaceEl, '正晝 12:00・' + weatherName);
			if (window.SwRoomDesc && window.SwRoomDesc.renderL1) {
				window.SwRoomDesc.renderL1({ hour_str: '12:00', period: '正晝' });
			}
			return;
		}

		var h = Math.floor(sec / 3600) % 24;
		var m = Math.floor((sec % 3600) / 60);
		var hourStr = (h < 10 ? '0' : '') + h + ':' + (m < 10 ? '0' : '') + m;
		var sixPhase = getSixPhase(h);

		// 1. 更新頂欄左側開譜曆日期
		var calText = formatEpochSeasonDate(days);
		if (calendarEl) {
			calendarEl.textContent = calText;
		}

		// 2. 更新頂欄右側六段時辰、天象與時間
		setPhasePlaceHtml(phasePlaceEl, sixPhase + ' ' + hourStr + '・' + weatherName);

		// 相容 L1 描述層
		if (window.SwRoomDesc && window.SwRoomDesc.renderL1) {
			window.SwRoomDesc.renderL1({ hour_str: hourStr, period: sixPhase });
		}
	}

	function startTicker() {
		if (gameTimeTicker) return;
		updateGameTimeDisplay();
		gameTimeTicker = setInterval(updateGameTimeDisplay, 500);
	}

	function stopTicker() {
		if (gameTimeTicker) {
			clearInterval(gameTimeTicker);
			gameTimeTicker = null;
		}
	}

	// 四條狀態欄（若有需要保留內部邏輯，但前台數值條已隱藏）
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

	// draw：刷新姓名與時鐘顯示
	function draw() {
		var state = window.gameState ? window.gameState() : null;
		if (!state) return;
		var nameEl = document.getElementById('player-name');
		if (nameEl) nameEl.textContent = (state.me && state.me.player_id) ? state.me.player_id : '姓名';
		updateGameTimeDisplay();
	}

	window.SwClock = {
		startTicker: startTicker,
		stopTicker: stopTicker,
		updateStatusBars: updateStatusBars,
		draw: draw,
		updateDisplay: updateGameTimeDisplay,
		formatDate: formatEpochSeasonDate,
		formatEpochSeasonDate: formatEpochSeasonDate,
		getSixPhase: getSixPhase,
		gameSecNow: gameSecNow,
		gameDaysNow: gameDaysNow
	};
	window.gameStartTimeTicker = startTicker;
	window.gameUpdateTime = updateGameTimeDisplay;
})();
