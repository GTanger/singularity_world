#!/usr/bin/env node
/**
 * 把 data/rooms 底下所有 .json 合併成一個檔，一房一行，方便捲動比對相似句／用詞。
 * 輸出：data/rooms.json（JSON 陣列，每個元素一行）
 */

const fs = require('fs');
const path = require('path');

// 專案根目錄（本檔位於 tools/js/）
const ROOTS_DIR = path.join(__dirname, '..', '..', 'data', 'rooms');
const OUT_FILE = path.join(__dirname, '..', '..', 'data', 'rooms.json');

function listJsonFiles(dir, acc = []) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) listJsonFiles(full, acc);
    else if (e.isFile() && e.name.endsWith('.json')) acc.push(full);
  }
  return acc;
}

const files = listJsonFiles(ROOTS_DIR).sort();
const rooms = [];

for (const f of files) {
  try {
    const raw = fs.readFileSync(f, 'utf8');
    const room = JSON.parse(raw);
    if (room && typeof room.id === 'string') rooms.push(room);
  } catch (err) {
    console.error('Skip:', f, err.message);
  }
}

const lines = ['['];
rooms.forEach((r, i) => {
  const oneLine = JSON.stringify(r);
  lines.push((i < rooms.length - 1 ? '  ' + oneLine + ',' : '  ' + oneLine));
});
lines.push(']');

fs.writeFileSync(OUT_FILE, lines.join('\n'), 'utf8');
console.log('Wrote', OUT_FILE, 'with', rooms.length, 'rooms (one room per line).');
