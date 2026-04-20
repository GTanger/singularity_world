// 錯誤密碼應被拒
const { WsClient, randId, assert, assertEq } = require('../helpers');

module.exports = async function test04_wrong_password() {
  const id = randId('W');
  const c1 = new WsClient();
  await c1.connect();
  c1.send({ type: 'create_character', player_id: id, password: 'correct_pass', display_char: '丙', gender: '男' });
  await c1.waitFor('me', 5000);
  c1.close();

  const c2 = new WsClient();
  await c2.connect();
  c2.send({ type: 'login', player_id: id, password: 'wrong_pass' });
  const err = await c2.waitFor('error', 3000);
  assert(err, '錯誤密碼應收 error');
  const me = await c2.waitFor('me', 500);
  assertEq(me, null, '錯誤密碼不該收 me');
  c2.close();
};
