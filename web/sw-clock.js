// 遊戲時鐘與開譜曆日晷：gameSecNow / gameDaysNow / formatEpochSeasonDate / updateGameTimeDisplay / ticker / draw。
(function () {
	var GAME_TIME_SCALE = 24;
	var GAME_SEC_PER_DAY = 86400;
	var DAYS_PER_YEAR = 365;
	var DEFAULT_GAME_EPOCH_UNIX = 1772117286;

	var gameTimeTicker = null;

	function gameSecNow() {
		if (window.__swOverrideHour != null && !isNaN(Number(window.__swOverrideHour))) {
			var oh = Number(window.__swOverrideHour) % 24;
			if (oh < 0) oh += 24;
			return oh * 3600;
		}
		var state = window.gameState ? window.gameState() : null;
		var sec = 0;
		if (state && state.server_unix) {
			var elapsed = Math.max(0, (Date.now() / 1000) - state.server_unix);
			sec = state.game_time_sec_since_midnight + elapsed * GAME_TIME_SCALE;
		} else {
			var nowUnix = Date.now() / 1000;
			sec = Math.max(0, (nowUnix - DEFAULT_GAME_EPOCH_UNIX) * GAME_TIME_SCALE);
		}
		sec = sec % GAME_SEC_PER_DAY;
		if (sec < 0) sec += GAME_SEC_PER_DAY;
		return sec;
	}

	function gameDaysNow() {
		var state = window.gameState ? window.gameState() : null;
		var secTotal = 0;
		if (state && state.server_unix) {
			var elapsed = Math.max(0, (Date.now() / 1000) - state.server_unix);
			secTotal = state.game_days_since_epoch * GAME_SEC_PER_DAY + state.game_time_sec_since_midnight + elapsed * GAME_TIME_SCALE;
		} else {
			var nowUnix = Date.now() / 1000;
			secTotal = Math.max(0, (nowUnix - DEFAULT_GAME_EPOCH_UNIX) * GAME_TIME_SCALE);
		}
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

	// 六段天色：拂曉、晝前、正晝、黃昏、夜段、黎明前
	function getSixPhase(hour) {
		var h = Math.floor(hour);
		if (h >= 4 && h < 8) return '拂曉';
		if (h >= 8 && h < 12) return '晝前';
		if (h >= 12 && h < 16) return '正晝';
		if (h >= 16 && h < 20) return '黃昏';
		if (h >= 20 && h < 24) return '夜段';
		return '黎明前';
	}

	// 48 色半小時天光表（每 30 遊戲分鐘 = 現實 75 秒一候；top: 天頂宣色, mid: 中軸宣色, bot: 地平宣色）
	// 破曉（05:00–06:00）地平先透曙光、天頂留黛；黃昏（18:00–19:00）天頂先染夜靛、地平留夕照餘暉。
	var SKY_TIDE_48_PALETTE = [
		{ slot: 0,  h: 0.0,  name: '夜央・玄夜墨',   top: [13, 17, 26],    mid: [18, 24, 34],    bot: [25, 33, 45] },
		{ slot: 1,  h: 0.5,  name: '深宵・深宵黛',   top: [14, 18, 27],    mid: [19, 25, 35],    bot: [26, 34, 46] },
		{ slot: 2,  h: 1.0,  name: '寒更・寒星墨',   top: [14, 19, 28],    mid: [20, 26, 36],    bot: [27, 35, 47] },
		{ slot: 3,  h: 1.5,  name: '沉夜・沉霧青',   top: [15, 20, 30],    mid: [21, 28, 38],    bot: [28, 36, 49] },
		{ slot: 4,  h: 2.0,  name: '更深・鐵紺夜',   top: [16, 21, 32],    mid: [22, 29, 40],    bot: [30, 38, 51] },
		{ slot: 5,  h: 2.5,  name: '蒼暝・蒼暝色',   top: [17, 23, 35],    mid: [24, 31, 43],    bot: [32, 41, 54] },
		{ slot: 6,  h: 3.0,  name: '夜闌・暗夜藍',   top: [19, 25, 38],    mid: [26, 34, 47],    bot: [35, 44, 58] },
		{ slot: 7,  h: 3.5,  name: '曉星・曉星紺',   top: [22, 29, 43],    mid: [30, 38, 52],    bot: [40, 50, 64] },
		{ slot: 8,  h: 4.0,  name: '殘夜・殘夜靛',   top: [26, 34, 50],    mid: [35, 44, 60],    bot: [47, 57, 72] },
		{ slot: 9,  h: 4.5,  name: '將曉・黎明黛',   top: [32, 42, 60],    mid: [43, 53, 70],    bot: [58, 68, 84] },
		{ slot: 10, h: 5.0,  name: '拂曉・魚肚微曙', top: [40, 52, 72],    mid: [54, 66, 84],    bot: [76, 84, 98] },
		{ slot: 11, h: 5.5,  name: '破曉・破曉雲母', top: [192, 208, 218], mid: [210, 220, 224], bot: [228, 224, 214] },
		{ slot: 12, h: 6.0,  name: '晨微・朝霧青宣', top: [202, 216, 224], mid: [218, 228, 232], bot: [234, 236, 232] },
		{ slot: 13, h: 6.5,  name: '朝露・晨露水色', top: [210, 222, 228], mid: [224, 232, 235], bot: [238, 241, 239] },
		{ slot: 14, h: 7.0,  name: '晨光・天青生宣', top: [216, 226, 228], mid: [229, 236, 236], bot: [241, 243, 240] },
		{ slot: 15, h: 7.5,  name: '朝日・薄青竹紙', top: [220, 228, 225], mid: [232, 237, 233], bot: [243, 244, 238] },
		{ slot: 16, h: 8.0,  name: '早潮・晨光月白', top: [224, 229, 222], mid: [235, 238, 230], bot: [245, 244, 236] },
		{ slot: 17, h: 8.5,  name: '朝晴・玉版素宣', top: [227, 230, 219], mid: [237, 238, 228], bot: [246, 244, 235] },
		{ slot: 18, h: 9.0,  name: '晝前・晴光粉宣', top: [229, 229, 216], mid: [239, 238, 226], bot: [247, 245, 235] },
		{ slot: 19, h: 9.5,  name: '晴嵐・暖玉宣色', top: [231, 228, 213], mid: [240, 237, 224], bot: [248, 245, 235] },
		{ slot: 20, h: 10.0, name: '隅中・米湯素宣', top: [232, 227, 210], mid: [241, 237, 223], bot: [248, 245, 235] },
		{ slot: 21, h: 10.5, name: '向午・象牙暖宣', top: [233, 226, 207], mid: [242, 236, 222], bot: [249, 245, 236] },
		{ slot: 22, h: 11.0, name: '日高・日華熟宣', top: [234, 225, 204], mid: [243, 236, 221], bot: [249, 246, 236] },
		{ slot: 23, h: 11.5, name: '照融・澄陽宣色', top: [235, 224, 202], mid: [244, 236, 220], bot: [250, 246, 236] },
		{ slot: 24, h: 12.0, name: '正晝・正晝牙白', top: [235, 224, 200], mid: [244, 235, 218], bot: [250, 245, 235] },
		{ slot: 25, h: 12.5, name: '日盛・日盛麻宣', top: [235, 223, 198], mid: [244, 234, 216], bot: [250, 244, 233] },
		{ slot: 26, h: 13.0, name: '日中・暖日檀宣', top: [234, 221, 196], mid: [243, 233, 214], bot: [249, 243, 231] },
		{ slot: 27, h: 13.5, name: '日斜・微醺麥宣', top: [233, 219, 193], mid: [242, 231, 212], bot: [249, 242, 229] },
		{ slot: 28, h: 14.0, name: '午後・暖茶宣色', top: [232, 217, 190], mid: [241, 229, 209], bot: [248, 240, 226] },
		{ slot: 29, h: 14.5, name: '日昳・淺焙桑皮', top: [231, 214, 187], mid: [240, 227, 206], bot: [247, 238, 223] },
		{ slot: 30, h: 15.0, name: '晚晴・斜陽金宣', top: [230, 211, 183], mid: [239, 225, 203], bot: [247, 237, 220] },
		{ slot: 31, h: 15.5, name: '斜陽・秋香熟宣', top: [229, 208, 180], mid: [238, 222, 199], bot: [246, 235, 217] },
		{ slot: 32, h: 16.0, name: '落照・落暉杏宣', top: [227, 203, 174], mid: [236, 218, 194], bot: [245, 232, 212] },
		{ slot: 33, h: 16.5, name: '夕照・夕霧琥珀', top: [224, 196, 166], mid: [234, 212, 186], bot: [243, 227, 205] },
		{ slot: 34, h: 17.0, name: '夕景・夕照金茶', top: [220, 188, 157], mid: [231, 204, 177], bot: [241, 221, 197] },
		{ slot: 35, h: 17.5, name: '黃昏・晚霞珀宣', top: [215, 179, 148], mid: [227, 196, 167], bot: [238, 214, 188] },
		{ slot: 36, h: 18.0, name: '日暮・茜霞藕荷', top: [206, 168, 140], mid: [220, 186, 158], bot: [233, 204, 178] },
		{ slot: 37, h: 18.5, name: '暮色・餘暉胭脂', top: [194, 156, 134], mid: [210, 174, 148], bot: [225, 192, 166] },
		{ slot: 38, h: 19.0, name: '夕霧・暮靄紫靛', top: [52, 54, 76],    mid: [66, 66, 86],    bot: [88, 76, 90] },
		{ slot: 39, h: 19.5, name: '薄暮・初更青靛', top: [42, 48, 68],    mid: [54, 58, 78],    bot: [72, 70, 84] },
		{ slot: 40, h: 20.0, name: '初夜・夜幕紺青', top: [34, 42, 60],    mid: [45, 52, 69],    bot: [60, 63, 76] },
		{ slot: 41, h: 20.5, name: '夜幕・星河深藍', top: [28, 36, 52],    mid: [38, 45, 61],    bot: [51, 56, 70] },
		{ slot: 42, h: 21.0, name: '星垂・二更松煙', top: [23, 30, 45],    mid: [32, 39, 53],    bot: [44, 50, 63] },
		{ slot: 43, h: 21.5, name: '沉冥・沉夜青墨', top: [20, 26, 39],    mid: [27, 34, 47],    bot: [38, 45, 57] },
		{ slot: 44, h: 22.0, name: '夜靜・三更玄青', top: [17, 23, 34],    mid: [24, 30, 42],    bot: [33, 40, 52] },
		{ slot: 45, h: 22.5, name: '寒夜・寒夜烏金', top: [15, 20, 31],    mid: [21, 27, 38],    bot: [29, 37, 49] },
		{ slot: 46, h: 23.0, name: '夜半・夜半濃墨', top: [14, 18, 28],    mid: [19, 25, 35],    bot: [27, 35, 46] },
		{ slot: 47, h: 23.5, name: '深更・深宵玄黛', top: [13, 17, 26],    mid: [18, 24, 34],    bot: [25, 33, 45] }
	];

	function lerpNum(a, b, t) {
		return a + (b - a) * t;
	}

	function lerpRgb(c0, c1, t) {
		return [
			Math.round(lerpNum(c0[0], c1[0], t)),
			Math.round(lerpNum(c0[1], c1[1], t)),
			Math.round(lerpNum(c0[2], c1[2], t))
		];
	}

	function rgbCss(c, alpha) {
		if (alpha != null && alpha < 1) {
			return 'rgba(' + c[0] + ', ' + c[1] + ', ' + c[2] + ', ' + alpha + ')';
		}
		return 'rgb(' + c[0] + ', ' + c[1] + ', ' + c[2] + ')';
	}

	function resolveEffectiveHourFloat(hourFloat) {
		if (window.__swOverrideHour != null && !isNaN(Number(window.__swOverrideHour))) {
			var oh = Number(window.__swOverrideHour) % 24;
			return oh < 0 ? oh + 24 : oh;
		}
		if (window.__swOverrideTide) {
			var ov = String(window.__swOverrideTide);
			if (!isNaN(Number(ov))) {
				var nh = Number(ov) % 24;
				return nh < 0 ? nh + 24 : nh;
			}
			if (ov === 'dawn') return 6.5;
			if (ov === 'noon') return 12.0;
			if (ov === 'dusk') return 17.5;
			if (ov === 'night') return 22.0;
		}
		var hf = Number(hourFloat);
		if (isNaN(hf)) return 12.0;
		hf = hf % 24;
		return hf < 0 ? hf + 24 : hf;
	}

	function computeSkyPalette(hourFloat) {
		var h = resolveEffectiveHourFloat(hourFloat);
		var slotIdx = Math.floor(h * 2) % 48;
		if (slotIdx < 0) slotIdx += 48;
		var entry = SKY_TIDE_48_PALETTE[slotIdx] || SKY_TIDE_48_PALETTE[24];
		var top = entry.top;
		var mid = entry.mid;
		var bot = entry.bot;
		var lum = (0.2126 * mid[0] + 0.7152 * mid[1] + 0.0722 * mid[2]) / 255;
		var isDark = lum < 0.50;
		var rawDark = Math.max(0, Math.min(1, (0.54 - lum) / 0.08));
		var darkBlend = 0.5 - 0.5 * Math.cos(Math.PI * rawDark);
		var tideKey = 'noon';
		if (isDark) {
			tideKey = 'night';
		} else if (slotIdx >= 11 && slotIdx < 17) {
			tideKey = 'dawn';
		} else if (slotIdx >= 32 && slotIdx <= 37) {
			tideKey = 'dusk';
		}

		var warmBias = (slotIdx >= 30 && slotIdx <= 37) ? Math.min(1, (slotIdx - 30) / 7) : 0;
		var dayInkHeading = lerpRgb([28, 36, 40], [30, 16, 8], warmBias);
		var dayInk = lerpRgb([34, 28, 20], [34, 18, 10], warmBias);
		var dayInkLight = lerpRgb([76, 68, 56], [58, 34, 20], warmBias);
		var dayInkFaint = lerpRgb([112, 102, 88], [82, 52, 34], warmBias);

		var nightDepth = Math.max(0, Math.min(1, (0.42 - lum) / 0.30));
		var nightInkHeading = lerpRgb([246, 239, 227], [232, 225, 212], nightDepth);
		var nightInk = lerpRgb([242, 235, 222], [222, 215, 202], nightDepth);
		var nightInkLight = lerpRgb([212, 210, 206], [174, 182, 190], nightDepth);
		var nightInkFaint = lerpRgb([178, 176, 174], [128, 138, 150], nightDepth);

		var inkHeading = isDark ? nightInkHeading : dayInkHeading;
		var ink = isDark ? nightInk : dayInk;
		var inkLight = isDark ? nightInkLight : dayInkLight;
		var inkFaint = isDark ? nightInkFaint : dayInkFaint;
		var accentInk = isDark ? '#e07a5f' : '#9e2a2b';
		var goldInk = isDark ? '#e0b25c' : '#8b6914';
		var errorInk = isDark ? '#f28b7d' : '#9e2a2b';
		var tokRgb = [
			Math.min(255, Math.round(mid[0] + 10)),
			Math.min(255, Math.round(mid[1] + 9)),
			Math.min(255, Math.round(mid[2] + 8))
		];
		var tokenBg = rgbCss(tokRgb, Number(lerpNum(0.82, 0.86, darkBlend).toFixed(3)));
		var barBg = rgbCss(bot, Number(lerpNum(0.18, 0.24, darkBlend).toFixed(3)));
		var btnRgb = lerpRgb([255, 252, 246], [220, 213, 199], darkBlend);
		var btnBg = rgbCss(btnRgb, Number(lerpNum(0.62, 0.14, darkBlend).toFixed(3)));
		var borderRgb = lerpRgb([110, 88, 62], [220, 213, 199], darkBlend);
		var btnBorder = rgbCss(borderRgb, Number(lerpNum(0.24, 0.26, darkBlend).toFixed(3)));

		var canvasTop = 'rgba(0, 0, 0, 0)';
		var canvasBot = 'rgba(0, 0, 0, 0)';
		return {
			slotIdx: slotIdx,
			slotName: entry.name,
			isDark: isDark,
			tideKey: tideKey,
			paperTop: rgbCss(top),
			paper: rgbCss(mid),
			paperBottom: rgbCss(bot),
			canvasTop: canvasTop,
			canvasBottom: canvasBot,
			barBg: barBg,
			tokenBg: tokenBg,
			inkHeading: rgbCss(inkHeading),
			ink: rgbCss(ink),
			inkLight: rgbCss(inkLight),
			inkFaint: rgbCss(inkFaint),
			btnBg: btnBg,
			btnBorder: btnBorder,
			accentInk: accentInk,
			goldInk: goldInk,
			errorInk: errorInk
		};
	}

	var _lastAppliedPhase = '';
	var _lastAppliedSlotIdx = -1;
	var _lastIsDark = null;
	var _hasSyncedServerTime = false;
	var _tideSwitchTimer = null;
	function setPropIfChanged(el, prop, val) {
		if (el.style.getPropertyValue(prop) !== val) {
			el.style.setProperty(prop, val);
		}
	}

	function applyTideVarsToElement(el, pal) {
		if (!el) return;
		el.setAttribute('data-tide-slot', String(pal.slotIdx));
		el.setAttribute('data-tide-name', pal.slotName);
		setPropIfChanged(el, '--paper-top', pal.paperTop);
		setPropIfChanged(el, '--paper', pal.paper);
		setPropIfChanged(el, '--paper-bg', pal.paper);
		setPropIfChanged(el, '--paper-bottom', pal.paperBottom);
		setPropIfChanged(el, '--paper-light', pal.paperTop);
		setPropIfChanged(el, '--paper-dark', pal.paperBottom);
		setPropIfChanged(el, '--bg', pal.paper);
		setPropIfChanged(el, '--bg-panel', pal.paper);
		setPropIfChanged(el, '--text', pal.ink);
		setPropIfChanged(el, '--text-muted', pal.inkLight);
		setPropIfChanged(el, '--room-name', pal.goldInk);
		setPropIfChanged(el, '--gold', pal.goldInk);
		setPropIfChanged(el, '--border', pal.btnBorder);
		setPropIfChanged(el, '--bar-hp', pal.errorInk);
		setPropIfChanged(el, '--sw-canvas-top', pal.canvasTop);
		setPropIfChanged(el, '--sw-canvas-bottom', pal.canvasBottom);
		setPropIfChanged(el, '--sw-canvas-bg', pal.canvasTop);
		setPropIfChanged(el, '--sw-bar-bg', pal.barBg);
		setPropIfChanged(el, '--sw-token-bg', pal.tokenBg);
		setPropIfChanged(el, '--ink-heading', pal.inkHeading);
		setPropIfChanged(el, '--ink', pal.ink);
		setPropIfChanged(el, '--ink-light', pal.inkLight);
		setPropIfChanged(el, '--ink-muted', pal.inkLight);
		setPropIfChanged(el, '--ink-faint', pal.inkFaint);
		setPropIfChanged(el, '--sw-btn-bg', pal.btnBg);
		setPropIfChanged(el, '--sw-btn-border', pal.btnBorder);
		setPropIfChanged(el, '--sw-accent-ink', pal.accentInk);
	}

	function applyPaperTide(hourFloat, phaseLabel, weatherLabel) {
		var pal = computeSkyPalette(hourFloat);
		var rootEl = document.documentElement;
		var appEl = document.getElementById('app') || document.getElementById('app-grid') || document.querySelector('.sw-app');
		var authEl = document.getElementById('auth-screen');
		var bodyEl = document.body;
		var state = window.gameState ? window.gameState() : null;
		var hasServerTime = !!(state && state.server_unix && window.__swOverrideHour == null && !window.__swOverrideTide);
		var isFirstServerSync = hasServerTime && !_hasSyncedServerTime;
		if (hasServerTime) _hasSyncedServerTime = true;
		var isFirstApply = (_lastAppliedSlotIdx === -1) || isFirstServerSync;
		var customFade = window.__swCustomFadeSec;
		var slotChanged = (pal.slotIdx !== _lastAppliedSlotIdx) || (customFade !== undefined) || isFirstServerSync || (appEl && appEl.getAttribute('data-tide-slot') !== String(pal.slotIdx));
		if ((appEl || bodyEl) && slotChanged) {
			if (_tideSwitchTimer) {
				clearTimeout(_tideSwitchTimer);
				_tideSwitchTimer = null;
			}
			var currentTideAttr = (appEl && appEl.getAttribute('data-tide')) || (bodyEl && bodyEl.getAttribute('data-tide')) || '';
			var tideChanged = (currentTideAttr !== pal.tideKey);
			var inkDelay = 0;
			if (!isFirstApply || customFade !== undefined) {
				var fadeSec = (customFade != null && !isNaN(Number(customFade)))
					? Number(customFade)
					: (isFirstApply ? 0 : 18);
				var isPolarityCross = (_lastIsDark !== null && _lastIsDark !== pal.isDark);
				var inkSec = (customFade != null && !isNaN(Number(customFade)))
					? (isPolarityCross ? Math.max(0, Number((fadeSec * 0.12).toFixed(2))) : fadeSec)
					: (isFirstApply ? 0 : (isPolarityCross ? 2.16 : 18));
				inkDelay = (isPolarityCross && fadeSec > 0)
					? Number((fadeSec * 0.31).toFixed(2))
					: 0;
				[rootEl, appEl, bodyEl, authEl].forEach(function (el) {
					if (!el) return;
					setPropIfChanged(el, '--sw-tide-dur', fadeSec + 's');
					setPropIfChanged(el, '--sw-ink-dur', inkSec + 's');
					setPropIfChanged(el, '--sw-ink-delay', inkDelay + 's');
					el.classList.add('sw-tide-ready');
				});
			}
			applyTideVarsToElement(rootEl, pal);
			applyTideVarsToElement(appEl, pal);
			applyTideVarsToElement(bodyEl, pal);
			applyTideVarsToElement(authEl, pal);
			[rootEl, appEl, bodyEl, authEl].forEach(function (el) {
				if (!el) return;
				el.setAttribute('data-tide-slot', String(pal.slotIdx));
				el.setAttribute('data-tide-name', pal.slotName);
			});
			if (bodyEl && bodyEl.style.backgroundColor !== pal.paper) {
				bodyEl.style.backgroundColor = pal.paper;
			}
			var themeMeta = document.querySelector('meta[name="theme-color"]');
			if (themeMeta) {
				themeMeta.setAttribute('content', pal.paperTop);
			}
			if (isFirstApply && customFade === undefined) {
				if (appEl) void appEl.offsetHeight;
				if (bodyEl) void bodyEl.offsetHeight;
				[rootEl, appEl, bodyEl, authEl].forEach(function (el) {
					if (el) el.classList.add('sw-tide-ready');
				});
			}
			_lastAppliedSlotIdx = pal.slotIdx;
			_lastIsDark = pal.isDark;
			if (tideChanged) {
				var applyTideAttr = function (tk) {
					if (rootEl) rootEl.setAttribute('data-tide', tk);
					if (appEl) appEl.setAttribute('data-tide', tk);
					if (bodyEl) bodyEl.setAttribute('data-tide', tk);
					if (authEl) authEl.setAttribute('data-tide', tk);
					if (window.SwObjectList && typeof window.SwObjectList.renderRoomCanvas === 'function') {
						window.SwObjectList.renderRoomCanvas();
					}
				};
				if (inkDelay > 0) {
					var targetTide = pal.tideKey;
					_tideSwitchTimer = setTimeout(function () {
						_tideSwitchTimer = null;
						applyTideAttr(targetTide);
					}, Math.round((inkDelay + 0.8) * 1000));
				} else {
					applyTideAttr(pal.tideKey);
				}
			}
		}
		if (phaseLabel && _lastAppliedPhase && _lastAppliedPhase !== phaseLabel) {
			if (appEl && !appEl.hidden && typeof window.SwPushBreath === 'function') {
				window.SwPushBreath('天光微移，時序入【' + phaseLabel + '】・' + (weatherLabel || '天朗') + '。', { byPlayer: false });
			}
		}
		if (phaseLabel) _lastAppliedPhase = phaseLabel;
	}
	window.SwSetPaperTide = function (overrideTideOrHour, customFadeSec) {
		if (typeof overrideTideOrHour === 'number') {
			window.__swOverrideHour = overrideTideOrHour;
			window.__swOverrideTide = null;
		} else {
			window.__swOverrideHour = null;
			window.__swOverrideTide = overrideTideOrHour || null;
		}
		window.__swCustomFadeSec = (customFadeSec !== undefined) ? customFadeSec : undefined;
		updateGameTimeDisplay();
		window.__swCustomFadeSec = undefined;
	};
	window.SwGetSkyTidePalette = function () {
		return SKY_TIDE_48_PALETTE.slice();
	};
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
			if (window.__swOverrideHour != null || window.__swOverrideTide != null) {
				applyPaperTide(12.0, '正晝', weatherName);
			}
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

		// 2. 更新頂欄右側六段天色、天象與時間
		setPhasePlaceHtml(phasePlaceEl, sixPhase + ' ' + hourStr + '・' + weatherName);

		applyPaperTide(sec / 3600, sixPhase, weatherName);
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
