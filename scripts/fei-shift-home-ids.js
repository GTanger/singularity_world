#!/usr/bin/env node
/**
 * 飛霜大街五段～八段 home ID 順移：五段 04→05,05→06,06→07；六段 07→08…10→11；
 * 七段 10→11…13→14；八段 14→15,15→16。僅處理飛霜大街五/六/七/八段目錄。
 * 從高編號往低替換內容，再批次改檔名。
 */
const fs = require('fs');
const path = require('path');

const FEI_ROOT = path.join(__dirname, '..', 'data', 'rooms', '飛霜大街');
const SEGMENTS = ['飛霜大街五段', '飛霜大街六段', '飛霜大街七段', '飛霜大街八段'];

function listJsonFiles(dir, acc = []) {
  if (!fs.existsSync(dir)) return acc;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) listJsonFiles(full, acc);
    else if (e.isFile() && e.name.endsWith('.json')) acc.push(full);
  }
  return acc;
}

// 從高到低替換，避免重複替換
const REPLACEMENTS = [
  ['fei_home15', 'fei_home16'],
  ['fei_home14', 'fei_home15'],
  ['fei_home13', 'fei_home14'],
  ['fei_home12', 'fei_home13'],
  ['fei_home11', 'fei_home12'],
  ['fei_home10', 'fei_home11'],
  ['fei_home09', 'fei_home10'],
  ['fei_home08', 'fei_home09'],
  ['fei_home07', 'fei_home08'],
  ['fei_home06', 'fei_home07'],
  ['fei_home05', 'fei_home06'],
  ['fei_home04', 'fei_home05'],
];

function pathInSegment(fullPath) {
  const rel = path.relative(FEI_ROOT, fullPath);
  return SEGMENTS.some(seg => rel.startsWith(seg));
}

const allFiles = [];
for (const seg of SEGMENTS) {
  const segPath = path.join(FEI_ROOT, seg);
  listJsonFiles(segPath, allFiles);
}
// 去重（listJsonFiles 可能重複加入）
const files = [...new Set(allFiles)].filter(pathInSegment);

for (const f of files) {
  let content = fs.readFileSync(f, 'utf8');
  for (const [from, to] of REPLACEMENTS) {
    content = content.split(from).join(to);
  }
  fs.writeFileSync(f, content, 'utf8');
}

// 重命名：每個檔名只依「原編號→新編號」對應一次（04→05, 05→06, …, 15→16）
const RENAME_MAP = { fei_home04: 'fei_home05', fei_home05: 'fei_home06', fei_home06: 'fei_home07', fei_home07: 'fei_home08', fei_home08: 'fei_home09', fei_home09: 'fei_home10', fei_home10: 'fei_home11', fei_home11: 'fei_home12', fei_home12: 'fei_home13', fei_home13: 'fei_home14', fei_home14: 'fei_home15', fei_home15: 'fei_home16' };
for (const seg of SEGMENTS) {
  const segPath = path.join(FEI_ROOT, seg);
  const dirs = fs.readdirSync(segPath, { withFileTypes: true }).filter(e => e.isDirectory());
  for (const d of dirs) {
    const dirPath = path.join(segPath, d.name);
    const jsons = fs.readdirSync(dirPath).filter(n => n.endsWith('.json'));
    for (const name of jsons) {
      for (const [from, to] of Object.entries(RENAME_MAP)) {
        if (name.startsWith(from + '_')) {
          const newName = to + name.slice(from.length);
          const fromPath = path.join(dirPath, name);
          const toPath = path.join(dirPath, newName);
          if (fs.existsSync(fromPath)) fs.renameSync(fromPath, toPath);
          break;
        }
      }
    }
  }
}

console.log('fei-shift-home-ids: replaced content and renamed files in 五～八段.');
