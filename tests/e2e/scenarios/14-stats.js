// me 訊息帶 hp/inner/spirit/stamina 四值、都 > 0
const { WsClient, randId, assert } = require('../helpers');

module.exports = async function test14_stats() {
  const id = randId('S');
  const c = new WsClient();
  await c.connect();
  c.send({ type: 'create_character', player_id: id, password: 'pw123456', display_char: '體', gender: '男' });
  const me = await c.waitFor('me', 5000);
  assert(me.hp_cur > 0, 'hp_cur > 0');
  assert(me.hp_max > 0, 'hp_max > 0');
  assert(me.inner_max > 0, 'inner_max > 0');
  assert(me.spirit_max > 0, 'spirit_max > 0');
  assert(me.stamina_max > 0, 'stamina_max > 0');
  assert(me.vit >= 6 && me.vit <= 14, `vit 合理範圍（${me.vit}）`);
  c.close();
};
