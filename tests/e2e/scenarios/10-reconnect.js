// 斷線重連：同帳號 login → 斷線 → 立即再 login 不應卡
const { WsClient, randId, assert, sleep } = require('../helpers');

module.exports = async function test10_reconnect() {
  const id = randId('R');
  const c0 = new WsClient();
  await c0.connect();
  c0.send({ type: 'create_character', player_id: id, password: 'pw123456', display_char: '重', gender: '男' });
  await c0.waitFor('me', 5000);
  c0.close();
  await sleep(200);

  // 連續 5 次 login + disconnect
  for (let i = 0; i < 5; i++) {
    const c = new WsClient();
    await c.connect();
    const t0 = Date.now();
    c.send({ type: 'login', player_id: id, password: 'pw123456' });
    const me = await c.waitFor('me', 3000);
    const elapsed = Date.now() - t0;
    assert(me, `第 ${i + 1} 次重連 login 應成功`);
    assert(elapsed < 1000, `第 ${i + 1} 次 login 應 < 1s（${elapsed}ms）`);
    c.close();
    await sleep(100);
  }
};
