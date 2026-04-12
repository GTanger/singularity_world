(function () {
    const canvas = document.getElementById('hex-canvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');

    const HEX_R = 202;
    const ROLE_RADIUS = 14;
    const SQRT3 = Math.sqrt(3);

    // 相機：決定畫面中心在哪個像素座標
    let cameraX = 0;
    let cameraY = 0;

    // 拖曳相關
    let dragging = false;
    let dragStartX = 0;
    let dragStartY = 0;
    let cameraStartX = 0;
    let cameraStartY = 0;
    let justDragged = false;

    // 當前視角中心的像素 (Canvas 中點)
    let centerX = 0;
    let centerY = 0;

    // 資料快取
    let currentCells = [];
    let currentEntities = [];
    let playerCoord = { q: 0, r: 0 };

    // 移動動畫 (跨格)
    let isMoving = false;
    let moveStartTime = 0;
    let moveDuration = 0;
    let moveStartPx = { x: 0, y: 0 };
    let moveEndPx = { x: 0, y: 0 };
    let moveCallback = null;

    // 格內平滑移動 (Intra-hex)
    let playerOffsetX = 0;
    let playerOffsetY = 0;
    let targetOffsetX = 0;
    let targetOffsetY = 0;
    let isIntraMoving = false;
    let moveSpeed = 0; // px/s
    let exploreProgress = 0; // 0.0 -> 1.0
    let scoutingHexes = new Set();
    let myPlayerId = '';
    let lastPlayerQ = null;
    let lastPlayerR = null;
    let lastFrameTime = 0;

    function hexToPixel(q, r) {
        const x = HEX_R * SQRT3 * (q + r / 2);
        const y = HEX_R * 3 / 2 * r;
        return { x: x, y: y };
    }

    function pixelToHex(x, y) {
        const q = (SQRT3 / 3 * x - 1 / 3 * y) / HEX_R;
        const r = (2 / 3 * y) / HEX_R;
        return axialRound(q, r);
    }

    function axialRound(q, r) {
        let x = q;
        let z = r;
        let y = -x - z;

        let rx = Math.round(x);
        let ry = Math.round(y);
        let rz = Math.round(z);

        const x_diff = Math.abs(rx - x);
        const y_diff = Math.abs(ry - y);
        const z_diff = Math.abs(rz - z);

        if (x_diff > y_diff && x_diff > z_diff) {
            rx = -ry - rz;
        } else if (y_diff > z_diff) {
            ry = -rx - rz;
        } else {
            rz = -rx - ry;
        }
        return { q: rx, r: rz };
    }

    function ensureCanvasSize() {
        const w = window.innerWidth;
        const h = window.innerHeight;
        const dpr = window.devicePixelRatio || 1;
        if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
            canvas.width = w * dpr;
            canvas.height = h * dpr;
            ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        }
        centerX = w / 2;
        centerY = h / 2;
    }

    function drawHex(x, y, radius, color, text, textColor, textFont, isFogged) {
        ctx.beginPath();
        for (let i = 0; i < 6; i++) {
            const angle = 2 * Math.PI / 6 * (i + 0.5); // pointy-top
            const px = x + radius * Math.cos(angle);
            const py = y + radius * Math.sin(angle);
            if (i === 0) ctx.moveTo(px, py);
            else ctx.lineTo(px, py);
        }
        ctx.closePath();
        ctx.fillStyle = color;
        ctx.fill();
        ctx.strokeStyle = '#1a1a1a';
        ctx.lineWidth = 1;
        ctx.stroke();

        if (text) {
            ctx.fillStyle = textColor || (isLight(color) ? '#000' : '#fff');
            ctx.font = "bold 48px '" + (textFont || 'Chiron GoRound TC') + "', sans-serif";
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText(text, x, y);
        }

        if (isFogged) {
            ctx.beginPath();
            for (let i = 0; i < 6; i++) {
                const angle = 2 * Math.PI / 6 * (i + 0.5);
                const px = x + radius * Math.cos(angle);
                const py = y + radius * Math.sin(angle);
                if (i === 0) ctx.moveTo(px, py);
                else ctx.lineTo(px, py);
            }
            ctx.closePath();
            ctx.fillStyle = 'rgba(20, 20, 20, 0.45)';
            ctx.fill();
        }
    }

    function isLight(color) {
        const hex = color.replace('#', '');
        const r = parseInt(hex.substr(0, 2), 16);
        const g = parseInt(hex.substr(2, 2), 16);
        const b = parseInt(hex.substr(4, 2), 16);
        const brightness = (r * 299 + g * 587 + b * 114) / 1000;
        return brightness > 128;
    }

    function drawEntity(x, y, char, color) {
        ctx.beginPath();
        ctx.arc(x, y, ROLE_RADIUS, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.fill();
        ctx.strokeStyle = '#333';
        ctx.lineWidth = 1;
        ctx.stroke();

        ctx.fillStyle = '#000';
        ctx.font = "12px 'Chiron GoRound TC', sans-serif";
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(char, x, y);
    }

    function checkCollision(nx, ny) {
        // 用 pixelToHex 判定字圓下一步落在哪個格
        const currentPx = hexToPixel(playerCoord.q, playerCoord.r);
        const nextHex = pixelToHex(currentPx.x + nx, currentPx.y + ny);

        if (nextHex.q === playerCoord.q && nextHex.r === playerCoord.r) {
            // 仍在本格內：只檢查 NPC 碰撞
            const others = currentEntities.filter(e => e.q === playerCoord.q && e.r === playerCoord.r && (e.id || e.ID) !== myPlayerId);
            for (let i = 0; i < others.length; i++) {
                const ex = (i - (others.length - 1) / 2) * (ROLE_RADIUS * 2 * 1.1);
                const ey = 0;
                const d = Math.sqrt((nx - ex) ** 2 + (ny - ey) ** 2);
                if (d < ROLE_RADIUS * 2) return true;
            }
            return false;
        }

        // 跨格：檢查鄰格是否可進入
        const neighbor = currentCells.find(c => c.q === nextHex.q && c.r === nextHex.r);
        if (!neighbor) {
            // 黑格：擋，並觸發偵查
            if (window.onBlackHexBoundaryStop) {
                window.onBlackHexBoundaryStop(nextHex.q, nextHex.r);
            }
            return true;
        }
        if (!neighbor.walkable) {
            return true; // 不可走地形
        }
        // 可走的彩格或灰霧格：放行跨格
        return false;
    }

    function render(timestamp) {
        if (!timestamp) timestamp = performance.now();
        if (!lastFrameTime) lastFrameTime = timestamp;
        const dt = (timestamp - lastFrameTime) / 1000;
        lastFrameTime = timestamp;

        if (isIntraMoving) {
            const dx = targetOffsetX - playerOffsetX;
            const dy = targetOffsetY - playerOffsetY;
            const dist = Math.sqrt(dx * dx + dy * dy);

            if (dist < 2) {
                playerOffsetX = targetOffsetX;
                playerOffsetY = targetOffsetY;
                isIntraMoving = false;
                // P2: 停止移動時若在灰霧格中心，啟動探索 Render Loop
                const cell = currentCells.find(c => c.q === playerCoord.q && c.r === playerCoord.r);
                if (cell && cell.fogged) {
                    requestAnimationFrame(render);
                }
            } else {
                const angle = Math.atan2(dy, dx);
                const step = moveSpeed * dt;
                const nextX = playerOffsetX + Math.cos(angle) * step;
                const nextY = playerOffsetY + Math.sin(angle) * step;

                if (checkCollision(nextX, nextY)) {
                    isIntraMoving = false;
                } else {
                    // 偵測跨格同步 (P1: 立即換算座標與 offset)
                    const currentPx = hexToPixel(playerCoord.q, playerCoord.r);
                    const nextCoord = pixelToHex(currentPx.x + nextX, currentPx.y + nextY);
                    if (nextCoord.q !== playerCoord.q || nextCoord.r !== playerCoord.r) {
                        const newPx = hexToPixel(nextCoord.q, nextCoord.r);
                        const moveDX = currentPx.x - newPx.x;
                        const moveDY = currentPx.y - newPx.y;

                        playerCoord.q = nextCoord.q;
                        playerCoord.r = nextCoord.r;
                        lastPlayerQ = playerCoord.q;
                        lastPlayerR = playerCoord.r;

                        playerOffsetX = nextX + moveDX;
                        playerOffsetY = nextY + moveDY;
                        targetOffsetX += moveDX;
                        targetOffsetY += moveDY;

                        if (window.onHexCrossBoundary) {
                            window.onHexCrossBoundary(nextCoord.q, nextCoord.r);
                        }
                    } else {
                        playerOffsetX = nextX;
                        playerOffsetY = nextY;
                    }
                }
            }
            // 重置探索進度（移動中不探索）
            exploreProgress = 0;
            // 讓相機跟隨字圓
            if (!isMoving) {
                const px = hexToPixel(playerCoord.q, playerCoord.r);
                cameraX = px.x + playerOffsetX;
                cameraY = px.y + playerOffsetY;
            }
        } else {
            // 靜止狀態：檢查是否在灰霧格中心進行探索
            const cell = currentCells.find(c => c.q === playerCoord.q && c.r === playerCoord.r);
            if (cell && cell.fogged) {
                const distToCenter = Math.sqrt(playerOffsetX * playerOffsetX + playerOffsetY * playerOffsetY);
                if (distToCenter <= 30) {
                    exploreProgress += dt / 3; // 3 秒
                    if (exploreProgress >= 1.0) {
                        if (window.onHexExploreComplete) {
                            window.onHexExploreComplete(playerCoord.q, playerCoord.r);
                        }
                        exploreProgress = 0;
                    }
                } else {
                    exploreProgress = 0;
                }
            } else {
                exploreProgress = 0;
            }
        }

        if (isMoving) {
            const now = Date.now();
            const elapsed = now - moveStartTime;
            const progress = Math.min(1, elapsed / moveDuration);

            cameraX = moveStartPx.x + (moveEndPx.x - moveStartPx.x) * progress;
            cameraY = moveStartPx.y + (moveEndPx.y - moveStartPx.y) * progress;

            if (progress >= 1) {
                isMoving = false;
                if (moveCallback) moveCallback();
            }
        }

        ensureCanvasSize();
        ctx.clearRect(0, 0, canvas.width, canvas.height);

        // 繪製格子
        currentCells.forEach(cell => {
            const px = hexToPixel(cell.q, cell.r);
            const screenX = px.x - cameraX + centerX;
            const screenY = px.y - cameraY + centerY;
            
            // 可視範圍檢測：margin 須 ≥ hex 外接圓半徑，否則邊角被裁
            const cullMargin = HEX_R * 2;
            if (screenX < -cullMargin || screenX > window.innerWidth + cullMargin || screenY < -cullMargin || screenY > window.innerHeight + cullMargin) return;

            drawHex(screenX, screenY, HEX_R, cell.color, cell.display_char, cell.text_color, cell.text_font, cell.fogged);
        });

        // 繪製實體（玩家獨立繪製，不依賴 currentEntities 位置）
        let playerDisplayChar = null;
        const entityGroups = {};
        currentEntities.forEach(e => {
            if ((e.id || e.ID) === myPlayerId) {
                playerDisplayChar = e.display_char;
                return; // 玩家不進 group，由下方獨立繪製
            }
            const key = `${e.q},${e.r}`;
            if (!entityGroups[key]) entityGroups[key] = [];
            entityGroups[key].push(e);
        });

        for (const key in entityGroups) {
            const group = entityGroups[key];
            const [q, r] = key.split(',').map(Number);
            const px = hexToPixel(q, r);
            const baseScreenX = px.x - cameraX + centerX;
            const baseScreenY = px.y - cameraY + centerY;
            group.forEach((e, i) => {
                const offsetX = (i - (group.length - 1) / 2) * (ROLE_RADIUS * 2 * 1.1);
                drawEntity(baseScreenX + offsetX, baseScreenY, e.display_char, '#c4b8a8');
            });
        }

        // 玩家字圓：永遠以 playerCoord + offset 繪製，不受 currentEntities 延遲影響
        if (playerDisplayChar) {
            const pPx = hexToPixel(playerCoord.q, playerCoord.r);
            const pScreenX = pPx.x - cameraX + centerX + playerOffsetX;
            const pScreenY = pPx.y - cameraY + centerY + playerOffsetY;
            drawEntity(pScreenX, pScreenY, playerDisplayChar, '#7ec8e3');

            // 探索進度環
            if (exploreProgress > 0) {
                ctx.beginPath();
                ctx.arc(pScreenX, pScreenY, 20, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * exploreProgress);
                ctx.strokeStyle = 'rgba(255, 220, 120, 0.8)';
                ctx.lineWidth = 3;
                ctx.stroke();
            }
        }

        if (isMoving || isIntraMoving || exploreProgress > 0) {
            requestAnimationFrame(render);
        }
    }

    function updateView(view, myId) {
        if (myId) myPlayerId = myId;
        currentCells = view.cells;
        currentEntities = view.entities;

        const serverCoord = view.center;

        if (lastPlayerQ === null) {
            // 初次載入：從伺服器同步座標
            playerCoord = serverCoord;
            playerOffsetX = 0;
            playerOffsetY = 0;
            targetOffsetX = 0;
            targetOffsetY = 0;
            isIntraMoving = false;
            lastPlayerQ = playerCoord.q;
            lastPlayerR = playerCoord.r;
        } else if (!isIntraMoving && !isMoving) {
            // 靜止時才從伺服器同步（避免移動中被 race condition 覆蓋）
            if (serverCoord.q !== playerCoord.q || serverCoord.r !== playerCoord.r) {
                playerCoord = serverCoord;
                playerOffsetX = 0;
                playerOffsetY = 0;
                targetOffsetX = 0;
                targetOffsetY = 0;
                lastPlayerQ = playerCoord.q;
                lastPlayerR = playerCoord.r;
            }
        }
        // 移動中不覆蓋 playerCoord，render loop 跨格偵測是權威

        if (!isMoving && !isIntraMoving) {
            const px = hexToPixel(playerCoord.q, playerCoord.r);
            cameraX = px.x + playerOffsetX;
            cameraY = px.y + playerOffsetY;
        }
        render();
    }

    function animateTo(targetQ, targetR, duration, callback) {
        const startPx = hexToPixel(playerCoord.q, playerCoord.r);
        const endPx = hexToPixel(targetQ, targetR);
        
        isMoving = true;
        moveStartTime = Date.now();
        moveDuration = duration * 1000;
        moveStartPx = startPx;
        moveEndPx = endPx;
        moveCallback = callback;
        
        requestAnimationFrame(render);
    }

    // 事件
    canvas.addEventListener('mousedown', e => {
        if (isMoving) return;
        dragging = true;
        dragStartX = e.clientX;
        dragStartY = e.clientY;
        cameraStartX = cameraX;
        cameraStartY = cameraY;
        justDragged = false;
    });

    window.addEventListener('mousemove', e => {
        if (!dragging) return;
        const dx = e.clientX - dragStartX;
        const dy = e.clientY - dragStartY;
        if (Math.abs(dx) > 5 || Math.abs(dy) > 5) {
            justDragged = true;
        }
        cameraX = cameraStartX - dx;
        cameraY = cameraStartY - dy;
        render();
    });

    window.addEventListener('mouseup', e => {
        if (!dragging) return;
        dragging = false;
        if (!justDragged) {
            // 點擊移動
            const rect = canvas.getBoundingClientRect();
            const clickX = e.clientX - rect.left;
            const clickY = e.clientY - rect.top;
            
            // 轉換為地圖絕對像素
            const mapX = clickX - centerX + cameraX;
            const mapY = clickY - centerY + cameraY;
            const targetHex = pixelToHex(mapX, mapY);
            const hexPx = hexToPixel(targetHex.q, targetHex.r);
            
            if (window.gameOnHexClick) {
                window.gameOnHexClick(targetHex.q, targetHex.r, mapX - hexPx.x, mapY - hexPx.y);
            }
        }
    });

    // 手勢支援
    canvas.addEventListener('touchstart', e => {
        if (isMoving || e.touches.length !== 1) return;
        const t = e.touches[0];
        dragging = true;
        dragStartX = t.clientX;
        dragStartY = t.clientY;
        cameraStartX = cameraX;
        cameraStartY = cameraY;
        justDragged = false;
    }, { passive: true });

    window.addEventListener('touchmove', e => {
        if (!dragging || e.touches.length !== 1) return;
        const t = e.touches[0];
        const dx = t.clientX - dragStartX;
        const dy = t.clientY - dragStartY;
        if (Math.abs(dx) > 5 || Math.abs(dy) > 5) {
            justDragged = true;
        }
        cameraX = cameraStartX - dx;
        cameraY = cameraStartY - dy;
        render();
    }, { passive: true });

    window.addEventListener('touchend', e => {
        if (!dragging) return;
        dragging = false;
        e.preventDefault();   // 阻止瀏覽器合成 mouseup，否則單觸被判雙擊
        if (!justDragged && e.changedTouches[0]) {
            const t = e.changedTouches[0];
            const rect = canvas.getBoundingClientRect();
            const clickX = t.clientX - rect.left;
            const clickY = t.clientY - rect.top;
            const mapX = clickX - centerX + cameraX;
            const mapY = clickY - centerY + cameraY;
            const targetHex = pixelToHex(mapX, mapY);
            const hexPx = hexToPixel(targetHex.q, targetHex.r);

            if (window.gameOnHexClick) {
                window.gameOnHexClick(targetHex.q, targetHex.r, mapX - hexPx.x, mapY - hexPx.y);
            }
        }
    });

    window.addEventListener('resize', () => {
        ensureCanvasSize();
        render();
    });

    // 主動觸發字型載入（canvas 不觸發 CSS @font-face）並重繪
    if (document.fonts) {
        Promise.all([
            document.fonts.load("bold 48px 'Chiron GoRound TC'"),
            document.fonts.load("bold 48px 'FZJiaGuWen'")
        ]).then(() => render());
    }

    // 公開 API
    window.hexCanvas = {
        updateView,
        animateTo,
        startIntraHexMove: (tx, ty, isRun) => {
            targetOffsetX = tx;
            targetOffsetY = ty;
            const currentCell = currentCells.find(c => c.q === playerCoord.q && c.r === playerCoord.r);
            if (currentCell && currentCell.fogged) {
                moveSpeed = 30; // 灰霧格固定速
            } else {
                moveSpeed = isRun ? 100 : 50;
            }
            isIntraMoving = true;
            lastFrameTime = performance.now();
            requestAnimationFrame(render);
        },
        render,
        getDist: (a, b) => {
            return (Math.abs(a.q - b.q) + Math.abs(a.q + a.r - b.q - b.r) + Math.abs(a.r - b.r)) / 2;
        },
        getDirection: (from, to) => {
            const dq = to.q - from.q;
            const dr = to.r - from.r;
            // 定義六方向向量 (pointy-top)
            const dirs = [
                { q: 1, r: 0, label: "東" },
                { q: 1, r: -1, label: "東北" },
                { q: 0, r: -1, label: "西北" },
                { q: -1, r: 0, label: "西" },
                { q: -1, r: 1, label: "西南" },
                { q: 0, r: 1, label: "東南" }
            ];
            
            let best = null;
            let bestDist = Infinity;
            for (const d of dirs) {
                const dist = Math.pow(dq - d.q, 2) + Math.pow(dr - d.r, 2);
                if (dist < bestDist) {
                    bestDist = dist;
                    best = d.label;
                }
            }
            return best;
        }
    };
})();
