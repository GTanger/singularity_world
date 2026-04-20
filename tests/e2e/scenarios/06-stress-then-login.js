// 回歸測試：先壓測大量 move、再用另一帳號 login——
// 本來的 bug：壓測後 login 會卡 10s（writer 霸佔 store lock）
// 這個 test 驗收方向四重構有沒有解
const { WsClient, randId, assert, sleep } = require('../helpers');

module.exports = async function test06_stress_then_login() {
  // Client A：創角後狂 move 10 秒
  const idA = randId('SA');
  const a = new WsClient();
  await a.connect();
  a.send({ type: 'create_character', player_id: idA, password: 'pw123456', display_char: '戊', gender: '男' });
  await a.waitFor('me', 5000);

  const dirs = ['東', '西', '北', '南'];
  const stressStart = Date.now();
  let moves = 0;
  while (Date.now() - stressStart < 10000) {
    a.send({ type: 'move', direction: dirs[moves % 4] });
    await sleep(50);
    moves++;
  }

  // Client B：在 A 還活著時嘗試創角 + login（模擬卡住情境）
  const idB = randId('SB');
  const b = new WsClient();
  await b.connect();
  b.send({ type: 'create_character', player_id: idB, password: 'pw123456', display_char: '己', gender: '女' });
  const t0 = Date.now();
  const bMe = await b.waitFor('me', 5000);
  const bElapsed = Date.now() - t0;
  assert(bMe, `B 創角應成功（卡 ${bElapsed}ms）`);
  assert(bElapsed < 3000, `B 創角應在 3s 內完成（實際 ${bElapsed}ms）`);

  a.close();
  b.close();
};
