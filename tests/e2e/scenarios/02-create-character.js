// 創角流程：新 id → create_character → 回 view + grid_view + me → PG entities 表新增一條
const { WsClient, randId, assert, assertEq, pgQuery, pgCount } = require('../helpers');

module.exports = async function test02_create_character() {
  const id = randId('E2E');
  const before = pgCount('entities', `id = '${id}'`);
  assertEq(before, 0, '新 id 事前在 PG 應不存在');

  const c = new WsClient();
  await c.connect();
  c.send({
    type: 'create_character',
    player_id: id,
    password: 'testpass123',
    display_char: '甲',
    gender: '男',
  });

  const view = await c.waitFor('view', 5000);
  const grid = await c.waitFor('grid_view', 5000);
  const me = await c.waitFor('me', 5000);

  assert(view, '應收到 view');
  assert(grid, '應收到 grid_view');
  assert(me, '應收到 me');
  assertEq(me.player_id, id, 'me.player_id 應匹配');

  const after = pgCount('entities', `id = '${id}'`);
  assertEq(after, 1, '創角後 PG entities 應新增');

  const authCount = pgCount('auth', `entity_id = '${id}'`);
  assertEq(authCount, 1, '創角後 auth 表應新增');

  c.close();
};
