// 10 個 client 同時 login——應全部成功、無卡住
const { WsClient, randId, assert, sleep } = require('../helpers');

module.exports = async function test09_concurrent_login() {
  // 準備 10 帳號
  const ids = Array.from({ length: 10 }, () => randId('C'));
  for (const id of ids) {
    const c = new WsClient();
    await c.connect();
    c.send({ type: 'create_character', player_id: id, password: 'pw123456', display_char: '多', gender: '男' });
    await c.waitFor('me', 5000);
    c.close();
  }
  await sleep(500);

  // 同時開 10 條 client login
  const t0 = Date.now();
  const results = await Promise.all(ids.map(async (id) => {
    const c = new WsClient();
    await c.connect();
    c.send({ type: 'login', player_id: id, password: 'pw123456' });
    const me = await c.waitFor('me', 5000);
    c.close();
    return !!me;
  }));
  const elapsed = Date.now() - t0;

  const okCount = results.filter(r => r).length;
  assert(okCount === 10, `10 個並發 login 都應成功（實際 ${okCount}/10，${elapsed}ms）`);
};
