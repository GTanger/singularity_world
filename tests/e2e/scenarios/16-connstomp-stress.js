// Stress scenario：先探索一小塊區域，再在已揭露格之間連戳移動 20 次
// 驗證：已揭露格的反覆移動不卡 server（save_square_grid_to_pg 只在新格觸發）
const { WsClient, randId, assert, assertEq, sleep } = require('../helpers');

const MOVE_TIMEOUT = 3000; // 每次 move 後等 GridView 的上限（ms）
const STOMPS = 20;          // 已揭露格來回次數

module.exports = async function test16_connstomp_stress() {
  const id = randId('CS');
  const c = new WsClient();
  await c.connect();

  // ── 建角色 / 進入世界 ──────────────────────────────────────────
  c.send({ type: 'create_character', player_id: id, password: 'pw123456', display_char: '壓', gender: '男' });
  const init = await c.waitFor('grid_view', 6000);
  assert(init, '初始 grid_view 未收到（create_character 失敗）');
  assertEq(init.player_x, 0, '出生格 x 應為 0');
  assertEq(init.player_y, 0, '出生格 y 應為 0');

  // ── 探索相鄰格（往四個方向各走一步再回中心）──────────────────────
  // 目的是讓 (0,0),(1,0),(-1,0),(0,1),(0,-1) 都進 explored 狀態
  const exploreSteps = [
    { dir: '東',  ex: 1,  ey: 0  },
    { dir: '西',  ex: 0,  ey: 0  },
    { dir: '西',  ex: -1, ey: 0  },
    { dir: '東',  ex: 0,  ey: 0  },
    { dir: '南',  ex: 0,  ey: -1 },
    { dir: '北',  ex: 0,  ey: 0  },
    { dir: '北',  ex: 0,  ey: 1  },
    { dir: '南',  ex: 0,  ey: 0  },
  ];

  for (const step of exploreSteps) {
    c.recv.length = 0;
    c.send({ type: 'move', direction: step.dir });
    const gv = await c.waitFor('grid_view', MOVE_TIMEOUT);
    assert(gv, `探索段：方向 ${step.dir} 後未收到 grid_view`);
    assertEq(gv.player_x, step.ex, `探索段：${step.dir} 後 x 應為 ${step.ex}`);
    assertEq(gv.player_y, step.ey, `探索段：${step.dir} 後 y 應為 ${step.ey}`);
  }

  // 回到 (0,0) 後確認 explored 格數 >= 5（四鄰 + 出生格）
  const exploreView = c.recv.find(m => m.type === 'grid_view');
  const exploredCount = (exploreView?.cells || []).filter(x => x.explored).length;
  assert(exploredCount >= 5, `探索後 explored 格數應 ≥ 5（實際 ${exploredCount}）`);

  // ── 連戳已揭露格 20 次（東西交替）────────────────────────────────
  const pattern = ['東', '西']; // (0,0)↔(1,0)，兩格都已揭露
  let expectedX = 0;
  const timings = [];

  for (let i = 0; i < STOMPS; i++) {
    const dir = pattern[i % 2];
    expectedX = dir === '東' ? expectedX + 1 : expectedX - 1;

    c.recv.length = 0;
    const t0 = Date.now();
    c.send({ type: 'move', direction: dir });
    const gv = await c.waitFor('grid_view', MOVE_TIMEOUT);
    const dt = Date.now() - t0;
    timings.push(dt);

    assert(gv, `stomp ${i + 1}/${STOMPS}：方向 ${dir} 後 ${MOVE_TIMEOUT}ms 內未收到 grid_view（server 可能卡死）`);
    assertEq(gv.player_x, expectedX, `stomp ${i + 1}：x 應為 ${expectedX}`);
    assertEq(gv.player_y, 0, `stomp ${i + 1}：y 應為 0`);

    // 若有 object_list / inventory_update 類訊息也記錄下來（不強制斷言有，但印出）
    const objMsg = c.recv.find(m => m.type === 'object_list' || m.type === 'inventory_update' || m.type === 'inventory');
    if (i === 0 && objMsg) {
      // 第一次有就算有 object_list 支援，不再重複印
      console.log(`    [stomp 1] 收到 ${objMsg.type} message`);
    }
  }

  // 統計 RTT
  const avg = Math.round(timings.reduce((a, b) => a + b, 0) / timings.length);
  const max = Math.max(...timings);
  console.log(`    [stomp stats] avg=${avg}ms  max=${max}ms  n=${STOMPS}`);
  assert(max < MOVE_TIMEOUT, `最慢一次 ${max}ms 超過 ${MOVE_TIMEOUT}ms 上限`);

  // ── 最終存活確認：ping ────────────────────────────────────────────
  c.recv.length = 0;
  c.send({ type: 'ping' });
  const pong = await c.waitFor('pong', 3000);
  assert(pong, 'stomp 完成後 ping 無回應（server 已死）');

  c.close();
};
