// 登入後移動：應收到更新後 grid_view + view
const { WsClient, randId, assert, assertEq } = require('../helpers');

module.exports = async function test05_move() {
  const id = randId('M');
  const c = new WsClient();
  await c.connect();
  c.send({ type: 'create_character', player_id: id, password: 'pw123456', display_char: '丁', gender: '男' });
  const initGrid = await c.waitFor('grid_view', 5000);
  assert(initGrid, '初始 grid_view');
  assertEq(initGrid.player_x, 0, '出生格 x=0');
  assertEq(initGrid.player_y, 0, '出生格 y=0');

  // 清 recv，等移動後的訊息
  c.recv.length = 0;
  c.send({ type: 'move', direction: '東' });

  // 等新 grid_view
  const afterMove = await c.waitFor('grid_view', 3000);
  assert(afterMove, '移動後應收新 grid_view');
  // 東 = x+1（coord.rs 定義）
  assertEq(afterMove.player_x, 1, '移動東後 x=1');
  assertEq(afterMove.player_y, 0, '移動東後 y=0');

  c.close();
};
