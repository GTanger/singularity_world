#!/usr/bin/env node
/**
 * 飛霜大街八段 → feist08_*，九段 → feist09_*
 * 八段：fei_hold01→feist08_hold01, fei_shop17→feist08_shop01, fei_home15→feist08_home01,
 *       霜尾宅/fei_home16→feist08_home02, 街燈宅/fei_home16→feist08_home03
 * 九段：fei_shop19→feist09_shop01, fei_shop20→feist09_shop02, fei_shop21→feist09_shop03,
 *       fei_home17→feist09_home01, fei_home18→feist09_home02
 */
const fs = require('fs');
const path = require('path');

const FEI = path.join(__dirname, '..', 'data', 'rooms', '飛霜大街');
const ST8 = path.join(FEI, '飛霜大街八段');
const ST9 = path.join(FEI, '飛霜大街九段');

function listJson(dir, acc = []) {
  if (!fs.existsSync(dir)) return acc;
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) listJson(full, acc);
    else if (e.name.endsWith('.json')) acc.push(full);
  }
  return acc;
}

// ---------- 八段 ----------
const files8 = listJson(ST8).filter((f) => path.basename(f) !== 'feistreet_st8.json');
for (const f of files8) {
  let content = fs.readFileSync(f, 'utf8');
  const rel = path.relative(ST8, f);
  if (rel.startsWith('霜脊樓' + path.sep)) {
    content = content.split('fei_hold01').join('feist08_hold01');
  } else if (rel.startsWith('街尾鋪' + path.sep)) {
    content = content.split('fei_shop17').join('feist08_shop01');
  } else if (rel.startsWith('簷角宅' + path.sep)) {
    content = content.split('fei_home15').join('feist08_home01');
  } else if (rel.startsWith('霜尾宅' + path.sep)) {
    content = content.split('fei_home16').join('feist08_home02');
  } else if (rel.startsWith('街燈宅' + path.sep)) {
    content = content.split('fei_home16').join('feist08_home03');
  }
  fs.writeFileSync(f, content, 'utf8');
}

// 八段檔名重命名
const renames8 = [
  [path.join(ST8, '霜脊樓'), 'fei_hold01_', 'feist08_hold01_'],
  [path.join(ST8, '街尾鋪'), 'fei_shop17_', 'feist08_shop01_'],
  [path.join(ST8, '簷角宅'), 'fei_home15_', 'feist08_home01_'],
  [path.join(ST8, '霜尾宅'), 'fei_home16_', 'feist08_home02_'],
  [path.join(ST8, '街燈宅'), 'fei_home16_', 'feist08_home03_'],
];
for (const [dir, from, to] of renames8) {
  if (!fs.existsSync(dir)) continue;
  for (const name of fs.readdirSync(dir)) {
    if (!name.endsWith('.json') || !name.startsWith(from)) continue;
    const newName = to + name.slice(from.length);
    fs.renameSync(path.join(dir, name), path.join(dir, newName));
  }
}

// 八段街道：hold/shop/home15 替換；fei_home16_main 第1、3 處→home03（街燈宅），第2、4 處→home02（霜尾宅）
const st8Path = path.join(ST8, 'feistreet_st8.json');
let st8 = fs.readFileSync(st8Path, 'utf8');
st8 = st8.split('fei_hold01').join('feist08_hold01');
st8 = st8.split('fei_shop17').join('feist08_shop01');
st8 = st8.split('fei_home15').join('feist08_home01');
st8 = st8.split('fei_home16_gate').join('feist08_home01_gate');
let n = 0;
st8 = st8.replace(/fei_home16_main/g, () => (++n === 1 || n === 3 ? 'feist08_home03_main' : 'feist08_home02_main'));
fs.writeFileSync(st8Path, st8, 'utf8');

// ---------- 九段 ----------
const files9 = listJson(ST9);
const map9 = [
  ['fei_shop19', 'feist09_shop01'],
  ['fei_shop20', 'feist09_shop02'],
  ['fei_shop21', 'feist09_shop03'],
  ['fei_home17', 'feist09_home01'],
  ['fei_home18', 'feist09_home02'],
];
for (const f of files9) {
  let content = fs.readFileSync(f, 'utf8');
  for (const [from, to] of map9) content = content.split(from).join(to);
  fs.writeFileSync(f, content, 'utf8');
}

const renames9 = [
  [path.join(ST9, '街口鋪'), 'fei_shop19_', 'feist09_shop01_'],
  [path.join(ST9, '匯流閣'), 'fei_shop20_', 'feist09_shop02_'],
  [path.join(ST9, '簷燈鋪'), 'fei_shop21_', 'feist09_shop03_'],
  [path.join(ST9, '街口宅'), 'fei_home17_', 'feist09_home01_'],
  [path.join(ST9, '簷匯宅'), 'fei_home18_', 'feist09_home02_'],
];
for (const [dir, from, to] of renames9) {
  if (!fs.existsSync(dir)) continue;
  for (const name of fs.readdirSync(dir)) {
    if (!name.endsWith('.json') || !name.startsWith(from)) continue;
    const newName = to + name.slice(from.length);
    fs.renameSync(path.join(dir, name), path.join(dir, newName));
  }
}

console.log('fei-rename-feist08-feist09: 八段→feist08_*, 九段→feist09_* done.');
