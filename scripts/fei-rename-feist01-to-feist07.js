#!/usr/bin/env node
/**
 * 飛霜大街一段～七段改為 feist01_ ～ feist07_ 前綴。
 * 每段僅處理該段目錄內檔案與街道；一段不替換 fei_lake。
 */
const fs = require('fs');
const path = require('path');

const FEI = path.join(__dirname, '..', 'data', 'rooms', '飛霜大街');

function listJson(dir, acc = []) {
  if (!fs.existsSync(dir)) return acc;
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) listJson(full, acc);
    else if (e.name.endsWith('.json')) acc.push(full);
  }
  return acc;
}

// 每段：[[舊前綴, 新前綴], ...]，依建築目錄順序
const SEGMENTS = {
  1: [['fei_shop01', 'feist01_shop01'], ['fei_shop02', 'feist01_shop02'], ['fei_home01', 'feist01_home01'], ['fei_home02', 'feist01_home02']],
  2: [['fei_shop03', 'feist02_shop01'], ['fei_home03', 'feist02_home01']],
  3: [['fei_shop04', 'feist03_shop01'], ['fei_shop05', 'feist03_shop02'], ['fei_home04', 'feist03_home01'], ['fei_home05', 'feist03_home02']],
  4: [['fei_shop04', 'feist04_shop01'], ['fei_shop05', 'feist04_shop02'], ['fei_home01', 'feist04_home01'], ['fei_home02', 'feist04_home02'], ['fei_home03', 'feist04_home03'], ['fei_home04', 'feist04_home04']],
  5: [['fei_shop08', 'feist05_shop01'], ['fei_shop09', 'feist05_shop02'], ['fei_shop10', 'feist05_shop03'], ['fei_home05', 'feist05_home01'], ['fei_home06', 'feist05_home02'], ['fei_home07', 'feist05_home03']],
  6: [['fei_shop11', 'feist06_shop01'], ['fei_shop12', 'feist06_shop02'], ['fei_shop13', 'feist06_shop03'], ['fei_home08', 'feist06_home01'], ['fei_home09', 'feist06_home02'], ['fei_home10', 'feist06_home03'], ['fei_home11', 'feist06_home04']],
  7: [['fei_shop14', 'feist07_shop01'], ['fei_shop15', 'feist07_shop02'], ['fei_shop16', 'feist07_shop03'], ['fei_home11', 'feist07_home01'], ['fei_home12', 'feist07_home02'], ['fei_home13', 'feist07_home03'], ['fei_home14', 'feist07_home04']],
};

const SEG_NAMES = { 1: '一段', 2: '二段', 3: '三段', 4: '四段', 5: '五段', 6: '六段', 7: '七段' };

for (let n = 1; n <= 7; n++) {
  const segDir = path.join(FEI, '飛霜大街' + SEG_NAMES[n]);
  const mapping = SEGMENTS[n];
  const streetFile = path.join(segDir, 'feistreet_st' + n + '.json');

  // 替換內容（建築目錄內所有 json + 街道）
  const files = listJson(segDir);
  for (const f of files) {
    let content = fs.readFileSync(f, 'utf8');
    if (n === 1 && content.includes('fei_lake')) {
      // 一段：只替換 shop01/02, home01/02，不碰 fei_lake
      content = content.split('fei_shop01').join('feist01_shop01');
      content = content.split('fei_shop02').join('feist01_shop02');
      content = content.split('fei_home01').join('feist01_home01');
      content = content.split('fei_home02').join('feist01_home02');
    } else {
      for (const [from, to] of mapping) content = content.split(from).join(to);
    }
    fs.writeFileSync(f, content, 'utf8');
  }

  // 重命名：各建築目錄下 fei_xxx_ 開頭檔名 → feist0N_xxx_
  const dirs = fs.readdirSync(segDir, { withFileTypes: true }).filter((e) => e.isDirectory());
  for (const d of dirs) {
    const buildDir = path.join(segDir, d.name);
    const jsons = fs.readdirSync(buildDir).filter((name) => name.endsWith('.json'));
    for (const name of jsons) {
      for (const [from, to] of mapping) {
        const prefixFrom = from + '_';
        const prefixTo = to + '_';
        if (name.startsWith(prefixFrom)) {
          const newName = prefixTo + name.slice(prefixFrom.length);
          const fromPath = path.join(buildDir, name);
          const toPath = path.join(buildDir, newName);
          if (fs.existsSync(fromPath)) fs.renameSync(fromPath, toPath);
          break;
        }
      }
    }
  }
}

console.log('fei-rename-feist01-to-feist07: 一段～七段 → feist01_ ～ feist07_ done.');
