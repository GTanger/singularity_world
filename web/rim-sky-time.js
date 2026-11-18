// 開譜曆 48 候天色光影與晝夜氛圍系統（與後端 gametime.rs 及 sw-clock.js 完全同源）
(() => {
  'use strict';

  const DEFAULT_EPOCH_UNIX = 1772117286;
  const GAME_SEC_PER_DAY = 86400;
  const DAYS_PER_YEAR = 365;

  // 48 色半小時天光表（top: 天頂, mid: 中軸, bot: 地平）
  const SKY_TIDE_48_PALETTE = [
    { h: 0.0, top: [13, 17, 26], mid: [18, 24, 34], bot: [25, 33, 45] },
    { h: 0.5, top: [14, 18, 27], mid: [19, 25, 35], bot: [26, 34, 46] },
    { h: 1.0, top: [14, 19, 28], mid: [20, 26, 36], bot: [27, 35, 47] },
    { h: 1.5, top: [15, 20, 30], mid: [21, 28, 38], bot: [28, 36, 49] },
    { h: 2.0, top: [16, 21, 32], mid: [22, 29, 40], bot: [30, 38, 51] },
    { h: 2.5, top: [17, 23, 35], mid: [24, 31, 43], bot: [32, 41, 54] },
    { h: 3.0, top: [19, 25, 38], mid: [26, 34, 47], bot: [35, 44, 58] },
    { h: 3.5, top: [22, 29, 43], mid: [30, 38, 52], bot: [40, 50, 64] },
    { h: 4.0, top: [26, 34, 50], mid: [35, 44, 60], bot: [47, 57, 72] },
    { h: 4.5, top: [32, 42, 60], mid: [43, 53, 70], bot: [58, 68, 84] },
    { h: 5.0, top: [40, 52, 72], mid: [54, 66, 84], bot: [76, 84, 98] },
    { h: 5.5, top: [192, 208, 218], mid: [210, 220, 224], bot: [228, 224, 214] },
    { h: 6.0, top: [202, 216, 224], mid: [218, 228, 232], bot: [234, 236, 232] },
    { h: 6.5, top: [210, 222, 228], mid: [224, 232, 235], bot: [238, 241, 239] },
    { h: 7.0, top: [216, 226, 228], mid: [229, 236, 236], bot: [241, 243, 240] },
    { h: 7.5, top: [220, 228, 225], mid: [232, 237, 233], bot: [243, 244, 238] },
    { h: 8.0, top: [224, 229, 222], mid: [235, 238, 230], bot: [245, 244, 236] },
    { h: 8.5, top: [227, 230, 219], mid: [237, 238, 228], bot: [246, 244, 235] },
    { h: 9.0, top: [229, 229, 216], mid: [239, 238, 226], bot: [247, 245, 235] },
    { h: 9.5, top: [231, 228, 213], mid: [240, 237, 224], bot: [248, 245, 235] },
    { h: 10.0, top: [232, 227, 210], mid: [241, 237, 223], bot: [248, 245, 235] },
    { h: 10.5, top: [233, 226, 207], mid: [242, 236, 222], bot: [249, 245, 236] },
    { h: 11.0, top: [234, 225, 204], mid: [243, 236, 221], bot: [249, 246, 236] },
    { h: 11.5, top: [235, 224, 202], mid: [244, 236, 220], bot: [250, 246, 236] },
    { h: 12.0, top: [235, 224, 200], mid: [244, 235, 218], bot: [250, 245, 235] },
    { h: 12.5, top: [235, 223, 198], mid: [244, 234, 216], bot: [250, 244, 233] },
    { h: 13.0, top: [234, 221, 196], mid: [243, 233, 214], bot: [249, 243, 231] },
    { h: 13.5, top: [233, 219, 193], mid: [242, 231, 212], bot: [249, 242, 229] },
    { h: 14.0, top: [232, 217, 190], mid: [241, 229, 209], bot: [248, 240, 226] },
    { h: 14.5, top: [231, 214, 187], mid: [240, 227, 206], bot: [247, 238, 223] },
    { h: 15.0, top: [230, 211, 183], mid: [239, 225, 203], bot: [247, 237, 220] },
    { h: 15.5, top: [229, 208, 180], mid: [238, 222, 199], bot: [246, 235, 217] },
    { h: 16.0, top: [227, 203, 174], mid: [236, 218, 194], bot: [245, 232, 212] },
    { h: 16.5, top: [224, 196, 166], mid: [234, 212, 186], bot: [243, 227, 205] },
    { h: 17.0, top: [220, 188, 157], mid: [231, 204, 177], bot: [241, 221, 197] },
    { h: 17.5, top: [215, 179, 148], mid: [227, 196, 167], bot: [238, 214, 188] },
    { h: 18.0, top: [206, 168, 140], mid: [220, 186, 158], bot: [233, 204, 178] },
    { h: 18.5, top: [194, 156, 134], mid: [210, 174, 148], bot: [225, 192, 166] },
    { h: 19.0, top: [52, 54, 76], mid: [66, 66, 86], bot: [88, 76, 90] },
    { h: 19.5, top: [42, 48, 68], mid: [54, 58, 78], bot: [72, 70, 84] },
    { h: 20.0, top: [34, 42, 60], mid: [45, 52, 69], bot: [60, 63, 76] },
    { h: 20.5, top: [28, 36, 52], mid: [38, 45, 61], bot: [51, 56, 70] },
    { h: 21.0, top: [23, 30, 45], mid: [32, 39, 53], bot: [44, 50, 63] },
    { h: 21.5, top: [20, 26, 39], mid: [27, 34, 47], bot: [38, 45, 57] },
    { h: 22.0, top: [17, 23, 34], mid: [24, 30, 42], bot: [33, 40, 52] },
    { h: 22.5, top: [15, 20, 31], mid: [21, 27, 38], bot: [29, 37, 49] },
    { h: 23.0, top: [14, 18, 28], mid: [19, 25, 35], bot: [27, 35, 46] },
    { h: 23.5, top: [13, 17, 26], mid: [18, 24, 34], bot: [25, 33, 45] },
  ];

  let anchor = null; // { localMs, secSinceMidnight, scale, dateLabel }

  function lerp(a, b, t) {
    return a + (b - a) * t;
  }

  function lerpRgb(c0, c1, t) {
    return [
      Math.round(lerp(c0[0], c1[0], t)),
      Math.round(lerp(c0[1], c1[1], t)),
      Math.round(lerp(c0[2], c1[2], t)),
    ];
  }

  function rgbCss(c, alpha) {
    if (alpha != null && alpha < 1) {
      return `rgba(${c[0]}, ${c[1]}, ${c[2]}, ${alpha})`;
    }
    return `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
  }

  function getPeriodLabel(hour) {
    const h = ((Math.floor(hour) % 24) + 24) % 24;
    if (h >= 4 && h <= 7) return '拂曉';
    if (h >= 8 && h <= 10) return '晝前';
    if (h >= 11 && h <= 14) return '正晝';
    if (h >= 15 && h <= 17) return '黃昏';
    if (h >= 18 && h <= 22) return '夜段';
    return '黎明前';
  }

  function formatFallbackDateLabel() {
    const nowSec = Date.now() / 1000;
    const totalGameSec = Math.max(0, (nowSec - DEFAULT_EPOCH_UNIX) * 24);
    const days = Math.floor(totalGameSec / GAME_SEC_PER_DAY);
    const year = Math.floor(days / DAYS_PER_YEAR) + 1;
    const doy = days % DAYS_PER_YEAR;
    let season = '春融';
    let day = doy + 1;
    if (doy >= 273) {
      season = '冬凝';
      day = doy - 273 + 1;
    } else if (doy >= 182) {
      season = '秋狩';
      day = doy - 182 + 1;
    } else if (doy >= 91) {
      season = '夏漲';
      day = doy - 91 + 1;
    }
    return `開譜曆 ${year === 1 ? '元' : year} 年・${season}・${day} 日`;
  }

  function syncFromServer(skyTime) {
    if (!skyTime || typeof skyTime.sec_since_midnight !== 'number') return;
    anchor = {
      localMs: performance.now(),
      secSinceMidnight: skyTime.sec_since_midnight,
      scale: Number(skyTime.scale) || 24,
      dateLabel: skyTime.date_label || formatFallbackDateLabel(),
    };
  }

  function getClockInfo() {
    let hourFloat = 12.0;
    let dateLabel = anchor ? anchor.dateLabel : formatFallbackDateLabel();

    if (window.__swOverrideHour != null && !isNaN(Number(window.__swOverrideHour))) {
      hourFloat = ((Number(window.__swOverrideHour) % 24) + 24) % 24;
    } else if (anchor) {
      const elapsedSec = Math.max(0, (performance.now() - anchor.localMs) / 1000);
      const sec = (anchor.secSinceMidnight + elapsedSec * anchor.scale) % GAME_SEC_PER_DAY;
      hourFloat = sec / 3600;
    } else {
      const nowSec = Date.now() / 1000;
      const sec = (Math.max(0, (nowSec - DEFAULT_EPOCH_UNIX) * 24)) % GAME_SEC_PER_DAY;
      hourFloat = sec / 3600;
    }

    const hour = Math.floor(hourFloat);
    const min = Math.floor((hourFloat - hour) * 60);
    const period = getPeriodLabel(hour);
    return {
      hourFloat,
      hour,
      min,
      period,
      dateLabel,
      clockText: `${dateLabel}・${period}`,
    };
  }

  function computeSkyTide(hourFloat) {
    const hf = ((hourFloat % 24) + 24) % 24;
    const pos = hf * 2;
    const i0 = Math.floor(pos) % 48;
    const i1 = (i0 + 1) % 48;
    const t = pos - Math.floor(pos);
    const c0 = SKY_TIDE_48_PALETTE[i0] || SKY_TIDE_48_PALETTE[24];
    const c1 = SKY_TIDE_48_PALETTE[i1] || SKY_TIDE_48_PALETTE[24];
    const top = lerpRgb(c0.top, c1.top, t);
    const mid = lerpRgb(c0.mid, c1.mid, t);
    const bot = lerpRgb(c0.bot, c1.bot, t);
    const lum = (0.2126 * mid[0] + 0.7152 * mid[1] + 0.0722 * mid[2]) / 255;
    const isNight = lum < 0.48;
    return { top, mid, bot, lum, isNight };
  }

  let lastAppliedSig = '';
  function applyDocumentSkyCss(hourFloat) {
    const info = getClockInfo();
    const h = hourFloat != null ? hourFloat : info.hourFloat;
    const tide = computeSkyTide(h);
    const sig = `${tide.top.join(',')}|${tide.mid.join(',')}|${tide.bot.join(',')}`;
    if (sig !== lastAppliedSig) {
      lastAppliedSig = sig;
      const root = document.documentElement;
      root.style.setProperty('--sky-top', rgbCss(tide.top));
      root.style.setProperty('--sky-mid', rgbCss(tide.mid));
      root.style.setProperty('--sky-bot', rgbCss(tide.bot));
      // 嚴守宣紙卡片高對比鐵律：夜間僅微暖紙面如燭光燈籠，絕不將紙張反黑、墨字反白
      if (tide.isNight) {
        root.style.setProperty('--paper', '#eadbc4');
        root.style.setProperty('--paper-light', '#f2e5d0');
        root.style.setProperty('--paper-dark', '#dbc8aa');
      } else {
        root.style.setProperty('--paper', '#f0e6d3');
        root.style.setProperty('--paper-light', '#f7f0e3');
        root.style.setProperty('--paper-dark', '#e0d4c0');
      }
    }
    return info;
  }

  let skyCanvas = null;
  let skyCtx = null;

  function ensureSkyCanvas(w, h) {
    if (!skyCanvas) {
      skyCanvas = document.createElement('canvas');
      skyCtx = skyCanvas.getContext('2d');
    }
    const iw = Math.max(1, Math.round(w));
    const ih = Math.max(1, Math.round(h));
    if (skyCanvas.width !== iw || skyCanvas.height !== ih) {
      skyCanvas.width = iw;
      skyCanvas.height = ih;
    }
    return skyCtx;
  }

  function computeAtmosphereMask(hourFloat) {
    const h = ((hourFloat % 24) + 24) % 24;
    if (h >= 8.0 && h < 15.0) {
      return { kind: 'day', nightAlpha: 0, tintColor: null };
    }
    if (h >= 15.0 && h < 18.5) {
      const k = Math.sin(Math.min(1, (h - 15.0) / 3.5) * Math.PI);
      const duskTint = `rgba(216, 148, 86, ${(0.18 * k).toFixed(3)})`;
      return { kind: 'dusk', nightAlpha: 0, tintColor: duskTint };
    }
    if (h >= 5.2 && h < 8.0) {
      const k = 1 - (h - 5.2) / 2.8;
      const dawnAlpha = Math.max(0, 0.16 * k);
      const nightAlpha = h < 5.75 ? 0.24 * (1 - (h - 5.2) / 0.55) : 0;
      return {
        kind: 'dawn',
        nightAlpha: Math.max(0, nightAlpha),
        tintColor: dawnAlpha > 0.01 ? `rgba(176, 198, 214, ${dawnAlpha.toFixed(3)})` : null,
      };
    }
    // 18.5 .. 24 .. 5.2 進入夜間松煙青墨（平順淡入，最高 0.48，保留月下墨線清晰度）
    let fade = 1.0;
    if (h >= 18.5 && h < 19.3) {
      fade = (h - 18.5) / 0.8;
    } else if (h >= 4.5 && h < 5.2) {
      fade = 1 - (h - 4.5) / 0.7;
    }
    return {
      kind: 'night',
      nightAlpha: Math.max(0, Math.min(0.48, 0.48 * fade)),
      tintColor: null,
    };
  }

  function drawCanvasAtmosphere(ctx, w, h, lights, hourFloat, nowMs) {
    const hf = hourFloat != null ? hourFloat : getClockInfo().hourFloat;
    const mask = computeAtmosphereMask(hf);

    if (mask.tintColor) {
      ctx.save();
      ctx.fillStyle = mask.tintColor;
      ctx.fillRect(0, 0, w, h);
      ctx.restore();
    }

    if (mask.nightAlpha > 0.02) {
      const sctx = ensureSkyCanvas(w, h);
      sctx.save();
      sctx.globalCompositeOperation = 'source-over';
      sctx.clearRect(0, 0, w, h);
      sctx.fillStyle = `rgba(18, 28, 44, ${mask.nightAlpha.toFixed(3)})`;
      sctx.fillRect(0, 0, w, h);

      if (Array.isArray(lights) && lights.length > 0) {
        sctx.globalCompositeOperation = 'destination-out';
        for (const l of lights) {
          const pulse = l.kind === 'campfire'
            ? 1 + Math.sin((nowMs || 0) * 0.008 + (l.x + l.y) * 0.03) * 0.05
            : 1;
          const r = Math.max(12, l.radius * pulse);
          const g = sctx.createRadialGradient(l.x, l.y, r * 0.14, l.x, l.y, r);
          g.addColorStop(0, 'rgba(0, 0, 0, 0.96)');
          g.addColorStop(0.55, 'rgba(0, 0, 0, 0.56)');
          g.addColorStop(1, 'rgba(0, 0, 0, 0)');
          sctx.fillStyle = g;
          sctx.beginPath();
          sctx.arc(l.x, l.y, r, 0, Math.PI * 2);
          sctx.fill();
        }
      }
      sctx.restore();

      ctx.save();
      ctx.drawImage(skyCanvas, 0, 0, w, h);

      // 疊一層溫潤燭火／恆光街燈暖琥珀暈（街燈恆亮抗雨）
      if (Array.isArray(lights)) {
        for (const l of lights) {
          if (l.kind !== 'campfire' && l.kind !== 'workbench' && l.kind !== 'lamp') continue;
          const warmR = l.radius * (l.kind === 'lamp' ? 0.78 : (l.kind === 'campfire' ? 0.72 : 0.65));
          const alpha = (l.kind === 'campfire' ? 0.24 : (l.kind === 'lamp' ? 0.19 : 0.12)) * (mask.nightAlpha / 0.48);
          const g = ctx.createRadialGradient(l.x, l.y, 2, l.x, l.y, warmR);
          g.addColorStop(0, l.kind === 'lamp'
            ? `rgba(255, 224, 146, ${alpha.toFixed(3)})`
            : `rgba(246, 178, 86, ${alpha.toFixed(3)})`);
          g.addColorStop(1, 'rgba(246, 178, 86, 0)');
          ctx.fillStyle = g;
          ctx.beginPath();
          ctx.arc(l.x, l.y, warmR, 0, Math.PI * 2);
          ctx.fill();
        }
      }
      ctx.restore();
    }
  }

  window.SwSkyTime = {
    syncFromServer,
    getClockInfo,
    applyDocumentSkyCss,
    drawCanvasAtmosphere,
    computeSkyTide,
  };
})();
