// 既有帳號登入——先 create 再 login 確保乾淨環境
const { WsClient, randId, assert, assertEq } = require('../helpers');

module.exports = async function test03_login() {
  const id = randId('L');
  // 準備：先創角
  const c1 = new WsClient();
  await c1.connect();
  c1.send({ type: 'create_character', player_id: id, password: 'testpass123', display_char: '乙', gender: '女' });
  await c1.waitFor('me', 5000);
  c1.close();

  // 測試：新 client 登入
  const c2 = new WsClient();
  await c2.connect();
  c2.send({ type: 'login', player_id: id, password: 'testpass123' });

  const view = await c2.waitFor('view', 5000);
  const me = await c2.waitFor('me', 5000);
  assert(view, 'login 應收 view');
  assert(me, 'login 應收 me');
  assertEq(me.player_id, id);

  c2.close();
};
