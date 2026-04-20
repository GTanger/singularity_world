// 測試 helpers——WS client + PG 驗證 + server 假設在 localhost:1721 跑
const WebSocket = require('ws');
const { spawnSync } = require('child_process');

const SERVER_URL = process.env.SW_TEST_URL || 'http://localhost:1721';
const WS_URL = SERVER_URL.replace(/^http/, 'ws') + '/ws';
const PG_CMD = ['docker', 'exec', '-i', 'postgres-singularity', 'psql', '-U', 'postgres', '-d', 'singularity', '-tAc'];

// 連線 + 收送 helper
class WsClient {
  constructor() {
    this.ws = null;
    this.recv = [];
    this.sent = [];
    this.closed = false;
    this.closeCode = null;
  }
  connect() {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(WS_URL);
      this.ws.on('open', () => resolve());
      this.ws.on('message', (d) => {
        try { this.recv.push(JSON.parse(d.toString())); }
        catch { this.recv.push({ _raw: d.toString() }); }
      });
      this.ws.on('close', (code) => { this.closed = true; this.closeCode = code; });
      this.ws.on('error', (e) => { if (!this.closed) reject(e); });
      setTimeout(() => reject(new Error('ws connect timeout')), 5000);
    });
  }
  send(obj) {
    this.sent.push(obj);
    this.ws.send(JSON.stringify(obj));
  }
  // 等到 recv 裡出現某個 type 的訊息；回傳該訊息或 null（超時）
  async waitFor(msgType, timeoutMs = 3000) {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      const m = this.recv.find(m => m.type === msgType);
      if (m) return m;
      await sleep(50);
    }
    return null;
  }
  recvOfType(msgType) {
    return this.recv.filter(m => m.type === msgType);
  }
  close() {
    try { this.ws?.close(); } catch {}
  }
}

function sleep(ms) {
  return new Promise(r => setTimeout(r, ms));
}

// PG query helper
function pgQuery(sql) {
  const r = spawnSync(PG_CMD[0], [...PG_CMD.slice(1), sql], { encoding: 'utf8' });
  if (r.status !== 0) throw new Error(`pg query failed: ${r.stderr}`);
  return r.stdout.trim();
}

function pgCount(table, where = '') {
  const w = where ? ` WHERE ${where}` : '';
  return parseInt(pgQuery(`SELECT COUNT(*) FROM ${table}${w}`), 10);
}

// 生成獨立的 test ID（避免 test 之間撞帳號）
function randId(prefix = 'T') {
  return prefix + Date.now().toString(36) + Math.floor(Math.random() * 1000);
}

// 驗收斷言
function assert(cond, msg) {
  if (!cond) throw new Error('ASSERT FAIL: ' + msg);
}
function assertEq(a, b, msg) {
  if (a !== b) throw new Error(`ASSERT EQ FAIL: ${msg} (got ${JSON.stringify(a)}, expected ${JSON.stringify(b)})`);
}

module.exports = { WsClient, sleep, pgQuery, pgCount, randId, assert, assertEq, SERVER_URL, WS_URL };
