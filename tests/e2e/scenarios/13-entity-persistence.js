// 資料持久化：創角後 entity 寫入 PG 的實際欄位正確
const { WsClient, randId, assertEq, pgQuery } = require('../helpers');

module.exports = async function test13_entity_persistence() {
  const id = randId('P');
  const c = new WsClient();
  await c.connect();
  c.send({ type: 'create_character', player_id: id, password: 'pw123456', display_char: '持', gender: '女' });
  await c.waitFor('me', 5000);
  c.close();

  const row = pgQuery(`SELECT display_char, gender, hex_q, hex_r FROM entities WHERE id = '${id}'`);
  assertEq(row.length > 0, true, 'PG 應找到該 entity');
  // 格式：display_char|gender|hex_q|hex_r
  const parts = row.split('|');
  assertEq(parts[0], '持', 'display_char 正確');
  // 性別 M/F
  assertEq(parts[1], 'F', 'gender=F（女→F）');
  // 出生 hex (0,0)
  assertEq(parts[2], '0', 'hex_q=0');
  assertEq(parts[3], '0', 'hex_r=0');
};
