// 非法移動：方格四向不含「東北」——server 應拒
const { WsClient, randId, assert, sleep } = require('../helpers');

module.exports = async function test12_invalid_move() {
  const id = randId('I');
  const c = new WsClient();
  await c.connect();
  c.send({ type: 'create_character', player_id: id, password: 'pw123456', display_char: '拒', gender: '男' });
  await c.waitFor('me', 5000);
  await sleep(200);

  c.recv.length = 0;
  c.send({ type: 'move', direction: '東北' });  // 四向不存在

  // 可能收到 error 或 blocked；不該當合法移動處理
  await sleep(1000);
  const moved = c.recvOfType('moved');
  assert(moved.length === 0, '非法四向方向不應有 moved 訊息');

  c.close();
};
