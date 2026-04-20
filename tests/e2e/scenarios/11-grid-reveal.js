// 方格揭露：移動一格、新格 explored=true
const { WsClient, randId, assert, assertEq, sleep } = require('../helpers');

module.exports = async function test11_grid_reveal() {
  const id = randId('G');
  const c = new WsClient();
  await c.connect();
  c.send({ type: 'create_character', player_id: id, password: 'pw123456', display_char: '揭', gender: '男' });
  const init = await c.waitFor('grid_view', 5000);
  assert(init, '初始 grid_view');
  const initExplored = (init.cells || []).filter(x => x.explored).length;
  assert(initExplored > 0, '出生應至少一格 explored');

  // 移動探索
  c.recv.length = 0;
  c.send({ type: 'move', direction: '東' });
  const afterMove = await c.waitFor('grid_view', 3000);
  assert(afterMove, '移動後 grid_view');
  const newExplored = (afterMove.cells || []).filter(x => x.explored).length;
  assert(newExplored >= initExplored, `探索格數不減少（${initExplored}→${newExplored}）`);

  c.close();
};
