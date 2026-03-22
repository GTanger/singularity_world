#!/usr/bin/env node
/**
 * 把 data/rooms 底下每個 .json 一房一檔，全部寫成「有段落的」格式（2 空格縮排），
 * 與 data/rooms/浮生大街/浮生大街一段/青磚民宅/life_home01_bedroom.json 一致。
 * 不處理 data/rooms.json（合併檔維持一房一行）。
 */

const fs = require('fs');
const path = require('path');

const ROOTS_DIR = path.join(__dirname, '..', 'data', 'rooms');

function listJsonFiles(dir, acc = []) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) listJsonFiles(full, acc);
    else if (e.isFile() && e.name.endsWith('.json')) acc.push(full);
  }
  return acc;
}

const files = listJsonFiles(ROOTS_DIR);
let ok = 0;
let err = 0;

for (const f of files) {
  try {
    const raw = fs.readFileSync(f, 'utf8');
    const data = JSON.parse(raw);
    const pretty = JSON.stringify(data, null, 2) + '\n';
    fs.writeFileSync(f, pretty, 'utf8');
    ok++;
  } catch (e) {
    console.error('Skip:', f, e.message);
    err++;
  }
}

console.log('Formatted', ok, 'room files (2-space indent, newlines). Errors:', err);
