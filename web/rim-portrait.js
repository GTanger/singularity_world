// 《奇點世界》MSXotto+ 即時紙娃娃肖像模組（零延遲本地疊層渲染，完全取代靜態 AI 肖像）
(() => {
  'use strict';

  function drawSlateFrame(ctx, w, h, glyph, isFemale) {
    const grad = ctx.createLinearGradient(0, 0, 0, h);
    grad.addColorStop(0, '#ebe2d0');
    grad.addColorStop(1, '#dfd2ba');
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, w, h);

    // 底部柔和草石地台，襯托紙娃娃站姿
    ctx.fillStyle = 'rgba(72, 92, 64, 0.22)';
    ctx.fillRect(4, h * 0.66, w - 8, h * 0.34 - 4);

    ctx.strokeStyle = '#a89880';
    ctx.lineWidth = 1;
    ctx.strokeRect(0.5, 0.5, w - 1, h - 1);
    ctx.strokeStyle = 'rgba(168, 152, 128, 0.42)';
    ctx.strokeRect(3.5, 3.5, w - 7, h - 7);

    if (glyph) {
      const sz = w < 110 ? 15 : 18;
      const sx = w - sz - 6;
      const sy = 6;
      ctx.fillStyle = isFemale ? '#9a322e' : '#8b2020';
      ctx.beginPath();
      ctx.roundRect(sx, sy, sz, sz, 2.5);
      ctx.fill();
      ctx.fillStyle = '#f7f0e3';
      ctx.font = `700 ${sz - 5}px "LXGW WenKai Mono TC", serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String(glyph).slice(0, 1), sx + sz / 2, sy + sz / 2 + 0.5);
    }
  }

  function render(canvas, opts = {}) {
    if (!canvas) return;
    const mode = opts.mode === 'bust' ? 'bust' : 'full';
    const cssW = opts.width || (mode === 'bust' ? 84 : 136);
    const cssH = opts.height || (mode === 'bust' ? 98 : 206);
    const dpr = Math.min(window.devicePixelRatio || 1, 3);
    canvas.width = Math.round(cssW * dpr);
    canvas.height = Math.round(cssH * dpr);
    canvas.style.width = `${cssW}px`;
    canvas.style.height = `${cssH}px`;

    const isFemale = opts.gender === 'F' || opts.gender === '女';
    const paint = (fromAsync) => {
      if (fromAsync && !canvas.isConnected) return;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      drawSlateFrame(ctx, cssW, cssH, opts.glyph, isFemale);

      if (window.CddaTiles && window.CddaTiles.isReady()) {
        // 採整數倍 32px 放大（3x=96px 或 2x=64px）確保像素銳利無模糊
        const intScale = mode === 'bust'
          ? Math.max(2, Math.floor(Math.min(cssW, cssH) / 36))
          : Math.max(3, Math.floor(cssW / 38));
        const tileSize = 32 * intScale;
        const cx = Math.round(cssW * 0.5);
        const cy = mode === 'bust' ? Math.round(cssH * 0.68) : Math.round(cssH * 0.66);
        window.CddaTiles.drawHumanPaperDoll(ctx, opts, cx, cy, tileSize, {
          wearHat: opts.wearHat !== false,
          showPack: Boolean(opts.showPack),
          tool: opts.tool || null,
          noFlip: true,
          noDownedRotate: true,
        });
        canvas.dataset.portraitStatus = 'ready';
      } else {
        canvas.dataset.portraitStatus = 'loading';
      }
    };

    paint(false);
    if (!window.CddaTiles || !window.CddaTiles.isReady()) {
      window.CddaTiles?.onReady(() => paint(true));
    }
  }

  function clearCache() {}
  function pickTraits(opts = {}) {
    return { isFemale: opts.gender === 'F' || opts.gender === '女' };
  }
  function whenReady() {
    return new Promise((resolve) => {
      if (window.CddaTiles && window.CddaTiles.isReady()) resolve();
      else if (window.CddaTiles) window.CddaTiles.onReady(resolve);
      else resolve();
    });
  }

  window.SwPortrait = { render, clearCache, pickTraits, whenReady };
})();
