// 最基本：WS 連線 + ping→pong
const { WsClient, assertEq } = require('../helpers');

module.exports = async function test01_ping() {
  const c = new WsClient();
  await c.connect();
  c.send({ type: 'ping' });
  const pong = await c.waitFor('pong');
  assertEq(pong?.type, 'pong', 'ping 應收到 pong');
  c.close();
};
