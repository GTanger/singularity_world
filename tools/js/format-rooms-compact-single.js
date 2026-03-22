#!/usr/bin/env node
/**
 * 將 data/rooms 底下每個 .json 格式化：
 * - 多個項目的陣列/物件：維持多行
 * - 僅單一項目（或空）：壓成單行，例如 "sockets": ["Move"]、 "responses": { "Move": "..." }
 * 不處理 data/rooms.json（合併檔）。
 */

const fs = require('fs');
const path = require('path');

const ROOTS_DIR = path.join(__dirname, '..', '..', 'data', 'rooms');

function listJsonFiles(dir, acc = []) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) listJsonFiles(full, acc);
    else if (e.isFile() && e.name.endsWith('.json')) acc.push(full);
  }
  return acc;
}

// 先 JSON.stringify 再對「單一項」sockets / responses 壓成單行
function formatRoomFinal(data) {
  let raw = JSON.stringify(data, null, 2);
  raw = raw.replace(/"sockets": \[\s*\n\s*"([^"]+)"\s*\n\s*\]/g, '"sockets": ["$1"]');
  // 單鍵 responses，value 可能含 \" 等跳脫
  raw = raw.replace(/"responses": \{\s*\n\s*"([^"]+)": "((?:[^"\\]|\\.)*)"\s*\n\s*\}/g, (_, k, v) => '"responses": { "' + k + '": "' + v.replace(/\\/g, '\\\\').replace(/"/g, '\\"') + '" }');
  return raw + '\n';
}

const files = listJsonFiles(ROOTS_DIR);
let ok = 0;
let err = 0;

for (const f of files) {
  try {
    const raw = fs.readFileSync(f, 'utf8');
    const data = JSON.parse(raw);
    const formatted = formatRoomFinal(data);
    fs.writeFileSync(f, formatted, 'utf8');
    ok++;
  } catch (e) {
    console.error('Skip:', f, e.message);
    err++;
  }
}

console.log('Formatted', ok, 'room files (single-line sockets/responses compacted). Errors:', err);
