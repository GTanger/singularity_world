#!/usr/bin/env node
/**
 * 檢查夜鴞巷一～四巷結構連結閉合：所有 exit.to 與 object.move_to_room_id 是否指向已存在的房間 id。
 * 並抽檢 description 主體（「可經」之前）字數是否 ≥50。
 */

const fs = require('fs');
const path = require('path');

const ROOTS = path.join(__dirname, '..', 'data', 'rooms');
const YAXIAO = path.join(ROOTS, '夜鴞巷');

function listJson(dir, acc = []) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) listJson(full, acc);
    else if (e.isFile() && e.name.endsWith('.json') && !e.name.startsWith('_')) acc.push(full);
  }
  return acc;
}

// 全專案房間 id
const allIds = new Set();
for (const f of listJson(ROOTS)) {
  try {
    const room = JSON.parse(fs.readFileSync(f, 'utf8'));
    if (room && room.id) allIds.add(room.id);
  } catch (_) {}
}

// 夜鴞巷房間與其連結
const targets = [];
const shortBodies = [];
const yaxiaoFiles = listJson(YAXIAO);

for (const f of yaxiaoFiles) {
  let room;
  try {
    room = JSON.parse(fs.readFileSync(f, 'utf8'));
  } catch (e) {
    console.error('Parse error:', f, e.message);
    continue;
  }
  if (!room || !room.id) continue;

  const from = room.id;
  const collect = (tid, kind) => {
    if (tid && typeof tid === 'string') targets.push({ from, to: tid, kind });
  };

  (room.exits || []).forEach((e) => collect(e.to, 'exit'));
  (room.objects || []).forEach((o) => {
    if (o.move_to_room_id) collect(o.move_to_room_id, 'object');
  });

  // 主體字數：取「可經」之前
  const desc = room.description || '';
  const idx = desc.indexOf('可經');
  const body = idx >= 0 ? desc.slice(0, idx).trim() : desc;
  const len = body.length;
  if (len > 0 && len < 50) shortBodies.push({ id: room.id, name: room.name, len, body: body.slice(0, 60) + '…' });
}

// 檢查閉合
const missing = targets.filter(({ to }) => !allIds.has(to));
const byTarget = {};
missing.forEach(({ to }) => { byTarget[to] = (byTarget[to] || 0) + 1; });

console.log('=== 夜鴞巷結構連結閉合 ===');
console.log('全專案房間數:', allIds.size);
console.log('夜鴞巷房間數:', yaxiaoFiles.length);
console.log('夜鴞巷出口/移動目標總數:', targets.length);
if (missing.length === 0) {
  console.log('閉合檢查: 通過，所有目標 id 皆存在。');
} else {
  console.log('閉合檢查: 失敗。以下目標 id 不存在:');
  Object.entries(byTarget).forEach(([id, count]) => console.log('  ', id, '(' + count + ' 處引用)'));
}

console.log('\n=== 主體不足 50 字（「可經」之前）===');
if (shortBodies.length === 0) {
  console.log('無。');
} else {
  shortBodies.forEach(({ id, name, len, body }) => console.log('  ', id, name, '字數:', len, '|', body));
}

process.exit(missing.length > 0 ? 1 : 0);
