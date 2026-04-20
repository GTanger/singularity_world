// 關鍵回歸：A 壓測後斷線 → B 登入——就是用戶說的「一開始能動、後來卡、重連登不上」那個 bug
const { WsClient, randId, assert, sleep } = require('../helpers');

module.exports = async function test07_disconnect_stress_login() {
  const idA = randId('DA');
  const a = new WsClient();
  await a.connect();
  a.send({ type: 'create_character', player_id: idA, password: 'pw123456', display_char: '庚', gender: '男' });
  await a.waitFor('me', 5000);

  const dirs = ['東', '西', '北', '南'];
  for (let i = 0; i < 100; i++) {
    a.send({ type: 'move', direction: dirs[i % 4] });
    await sleep(40);
  }
  a.close();
  await sleep(500);  // 等 server cleanup

  // 新 client login 既有帳號
  const idB = randId('DB');
  const prep = new WsClient();
  await prep.connect();
  prep.send({ type: 'create_character', player_id: idB, password: 'pw123456', display_char: '辛', gender: '女' });
  await prep.waitFor('me', 5000);
  prep.close();
  await sleep(300);

  const b = new WsClient();
  await b.connect();
  const t0 = Date.now();
  b.send({ type: 'login', player_id: idB, password: 'pw123456' });
  const me = await b.waitFor('me', 5000);
  const elapsed = Date.now() - t0;
  assert(me, `login 應回應（卡 ${elapsed}ms）`);
  assert(elapsed < 3000, `login 應 < 3s（實際 ${elapsed}ms）——這是核心 bug 回歸點`);
  b.close();
};
