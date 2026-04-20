// get_inventory 查詢：新玩家空背包
const { WsClient, randId, assert } = require('../helpers');

module.exports = async function test15_inventory() {
  const id = randId('V');
  const c = new WsClient();
  await c.connect();
  c.send({ type: 'create_character', player_id: id, password: 'pw123456', display_char: '囊', gender: '男' });
  await c.waitFor('me', 5000);

  c.recv.length = 0;
  c.send({ type: 'get_inventory' });
  const inv = await c.waitFor('inventory', 3000);
  assert(inv, '應收 inventory');
  assert(Array.isArray(inv.items), 'inventory.items 應為陣列');
  c.close();
};
