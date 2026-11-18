// 《奇點世界》人物靜態立繪渲染模組
(() => {
  'use strict';

  const ASSET_VER = '20261118a17';
  const imgCache = new Map();       // queryKey -> { img, status }
  const inflightMap = new Map();    // queryKey -> Promise
  const retryTimers = new WeakMap();
  const renderTokens = new WeakMap();
  const readyCallbacks = [];

  function drawPaperFrame(ctx, w, h, glyph, isFemale) {
    const grad = ctx.createLinearGradient(0, 0, 0, h);
    grad.addColorStop(0, '#f7f1e5');
    grad.addColorStop(1, '#ede3cf');
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, w, h);

    ctx.strokeStyle = '#a89880';
    ctx.lineWidth = 1;
    ctx.strokeRect(0.5, 0.5, w - 1, h - 1);
    ctx.strokeStyle = 'rgba(168, 152, 128, 0.40)';
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

  function drawStamp(ctx, w, h, glyph, isFemale) {
    if (!glyph) return;
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

  function drawBadge(ctx, w, h, text) {
    ctx.save();
    ctx.fillStyle = 'rgba(247, 240, 227, 0.88)';
    ctx.fillRect(5, h - 22, w - 10, 17);
    ctx.fillStyle = '#6b5a47';
    ctx.font = '600 10.5px "Noto Sans TC", sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(text, w * 0.5, h - 13.5);
    ctx.restore();
  }

  function buildQueryKey(opts = {}) {
    const p = new URLSearchParams();
    p.set('id', String(opts.id || 1));
    p.set('name', String(opts.name || '角色'));
    p.set('gender', opts.gender === 'F' || opts.gender === '女' ? 'F' : 'M');
    p.set('soul_seed', String(opts.soul_seed || '0'));
    if (opts.role) p.set('role', String(opts.role));
    p.set('wear_hat', opts.wearHat === false ? 'false' : 'true');
    p.set('show_pack', opts.showPack ? 'true' : 'false');
    p.set('tool', opts.tool ? String(opts.tool) : '');
    p.set('hp_ratio', String(typeof opts.hpRatio === 'number' ? opts.hpRatio : 1.0));
    return p.toString();
  }

  function paintStaticPortrait(canvas, cssW, cssH, mode, glyph, isFemale, entry) {
    const ctx = canvas.getContext('2d');
    const dpr = Math.min(window.devicePixelRatio || 1, 3);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';

    drawPaperFrame(ctx, cssW, cssH, null, isFemale);
    if (entry && entry.img && entry.img.naturalWidth > 0) {
      const iw = entry.img.naturalWidth;
      const ih = entry.img.naturalHeight;
      const pad = 4;
      const dw = cssW - pad * 2;
      const dh = cssH - pad * 2;
      if (mode === 'bust') {
        // 同一張全身立繪裁切上半身胸像（涵蓋頭笠、五官、背心雙肩與舉起手持器物）
        const sx = Math.round(iw * 0.04);
        const sy = 0;
        const sw = Math.round(iw * 0.92);
        const sh = Math.round(ih * 0.54);
        const fit = Math.min(dw / sw, dh / sh);
        const w = sw * fit;
        const h = sh * fit;
        ctx.drawImage(entry.img, sx, sy, sw, sh, (cssW - w) * 0.5, (cssH - h) * 0.5, w, h);
      } else {
        const fit = Math.min(dw / iw, dh / ih);
        const w = iw * fit;
        const h = ih * fit;
        ctx.drawImage(entry.img, 0, 0, iw, ih, (cssW - w) * 0.5, (cssH - h) * 0.5, w, h);
      }
      if (entry.status === 'updating') {
        drawBadge(ctx, cssW, cssH, '換裝描繪中…');
      }
    } else {
      ctx.save();
      ctx.fillStyle = '#6b5a47';
      ctx.font = '600 11.5px "Noto Sans TC", sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('丹青描繪中…', cssW * 0.5, cssH * 0.5);
      ctx.restore();
    }
    drawStamp(ctx, cssW, cssH, glyph, isFemale);
  }

  async function fetchPortrait(queryKey) {
    if (inflightMap.has(queryKey)) return inflightMap.get(queryKey);
    const job = (async () => {
      try {
        const url = `/api/rim/portrait/image?${queryKey}&v=${ASSET_VER}`;
        const res = await fetch(url, { credentials: 'same-origin', cache: 'no-store' });
        if (res.status === 202) {
          return { img: null, status: 'pending' };
        }
        if (!res.ok) {
          return { img: null, status: 'pending' };
        }
        const status = res.headers.get('X-Portrait-Status') || 'ready';
        const blob = await res.blob();
        const objUrl = URL.createObjectURL(blob);
        const img = await new Promise((resolve, reject) => {
          const im = new Image();
          im.onload = () => { URL.revokeObjectURL(objUrl); resolve(im); };
          im.onerror = (err) => { URL.revokeObjectURL(objUrl); reject(err); };
          im.src = objUrl;
        });
        const entry = { img, status };
        if (status === 'ready') {
          imgCache.set(queryKey, entry);
        }
        return entry;
      } catch (_) {
        return { img: null, status: 'pending' };
      } finally {
        inflightMap.delete(queryKey);
      }
    })();
    inflightMap.set(queryKey, job);
    return job;
  }

  function clearCache() {
    imgCache.clear();
  }

  function render(canvas, opts = {}) {
    if (!canvas) return;
    const prevTimer = retryTimers.get(canvas);
    if (prevTimer) {
      clearTimeout(prevTimer);
      retryTimers.delete(canvas);
    }

    const mode = opts.mode === 'bust' ? 'bust' : 'full';
    const cssW = opts.width || (mode === 'bust' ? 92 : 144);
    const cssH = opts.height || (mode === 'bust' ? 108 : 220);
    const dpr = Math.min(window.devicePixelRatio || 1, 3);
    canvas.width = Math.round(cssW * dpr);
    canvas.height = Math.round(cssH * dpr);
    canvas.style.width = `${cssW}px`;
    canvas.style.height = `${cssH}px`;

    const isFemale = opts.gender === 'F' || opts.gender === '女';
    const queryKey = buildQueryKey(opts);
    const token = (renderTokens.get(canvas) || 0) + 1;
    renderTokens.set(canvas, token);

    const cached = imgCache.get(queryKey);
    if (cached && cached.status === 'ready') {
      canvas.dataset.portraitStatus = 'ready';
      paintStaticPortrait(canvas, cssW, cssH, mode, opts.glyph, isFemale, cached);
      while (readyCallbacks.length > 0) {
        const fn = readyCallbacks.shift();
        try { fn(); } catch (_) {}
      }
      return;
    }

    canvas.dataset.portraitStatus = 'loading';
    paintStaticPortrait(canvas, cssW, cssH, mode, opts.glyph, isFemale, null);

    fetchPortrait(queryKey).then((entry) => {
      if (!canvas.isConnected || renderTokens.get(canvas) !== token) return;
      canvas.dataset.portraitStatus = entry.status;
      paintStaticPortrait(canvas, cssW, cssH, mode, opts.glyph, isFemale, entry);
      if (entry.status === 'ready') {
        while (readyCallbacks.length > 0) {
          const fn = readyCallbacks.shift();
          try { fn(); } catch (_) {}
        }
        return;
      }
      const tid = setTimeout(() => {
        if (canvas.isConnected && renderTokens.get(canvas) === token) {
          render(canvas, opts);
        }
      }, 1500);
      retryTimers.set(canvas, tid);
    });
  }

  function pickTraits(opts = {}) {
    return { isFemale: opts.gender === 'F' || opts.gender === '女' };
  }

  function whenReady() {
    if (inflightMap.size === 0) return Promise.resolve();
    return new Promise((resolve) => {
      readyCallbacks.push(resolve);
      setTimeout(resolve, 6000);
    });
  }

  window.SwPortrait = { render, clearCache, pickTraits, whenReady, buildQueryKey };
})();
