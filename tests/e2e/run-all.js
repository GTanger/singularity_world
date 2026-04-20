// 跑所有 scenario 並報告
const fs = require('fs');
const path = require('path');
const { SERVER_URL } = require('./helpers');

const SCENARIOS_DIR = path.join(__dirname, 'scenarios');

async function main() {
  console.log(`E2E baseline against ${SERVER_URL}`);
  console.log('='.repeat(60));

  const files = fs.readdirSync(SCENARIOS_DIR).filter(f => f.endsWith('.js')).sort();
  const results = [];

  for (const f of files) {
    const scenario = require(path.join(SCENARIOS_DIR, f));
    const t0 = Date.now();
    let status = 'PASS';
    let err = null;
    try {
      await scenario();
    } catch (e) {
      status = 'FAIL';
      err = e.message;
    }
    const dt = Date.now() - t0;
    const flag = status === 'PASS' ? '✓' : '✗';
    console.log(`${flag} ${f.padEnd(45)} ${dt}ms  ${status}${err ? ' — ' + err : ''}`);
    results.push({ file: f, status, elapsed: dt, err });
  }

  console.log('='.repeat(60));
  const pass = results.filter(r => r.status === 'PASS').length;
  const fail = results.filter(r => r.status === 'FAIL').length;
  console.log(`${pass} passed, ${fail} failed, total ${results.length}`);
  process.exit(fail > 0 ? 1 : 0);
}

main().catch(e => { console.error('RUNNER ERROR:', e); process.exit(2); });
