#!/usr/bin/env node
/**
 * 產生夜鴞巷 5 商鋪 + 10 住宅的 exits，接在 rooms.json 的 exits 陣列中。
 * 夜鴞巷僅 1 格 (yaxiaolane)，所有建築首格與逃生口皆連 yaxiaolane。
 */
const fs = require('fs');
const path = require('path');

const ROOMS_JSON = path.join(__dirname, '../data/rooms.json');

// 從 rooms 讀取 yaxiao_ 的 name，用於 direction
const data = JSON.parse(fs.readFileSync(ROOMS_JSON, 'utf8'));
const nameById = {};
data.rooms.forEach((r) => { if (r.id.startsWith('yaxiao_')) nameById[r.id] = r.name; });

const exits = data.exits;
const newExits = [];

// 15 棟建築首格 ID（夜鴞巷 ↔ 這些格）
const ENTRANCE_IDS = [
  'yaxiao_01', 'yaxiao_26', 'yaxiao_44', 'yaxiao_74', 'yaxiao_96',
  'yaxiao_116', 'yaxiao_128', 'yaxiao_138', 'yaxiao_153', 'yaxiao_166',
  'yaxiao_174', 'yaxiao_185', 'yaxiao_199', 'yaxiao_208', 'yaxiao_220'
];
// 15 棟建築逃生口（這些格 → 夜鴞巷）
const ESCAPE_IDS = [
  'yaxiao_25', 'yaxiao_43', 'yaxiao_73', 'yaxiao_95', 'yaxiao_115',
  'yaxiao_127', 'yaxiao_137', 'yaxiao_152', 'yaxiao_165', 'yaxiao_173',
  'yaxiao_184', 'yaxiao_198', 'yaxiao_207', 'yaxiao_219', 'yaxiao_229'
];

// 1) 夜鴞巷 → 各建築首格（direction 用目標房間 name）
ENTRANCE_IDS.forEach((id) => {
  newExits.push({ from: 'yaxiaolane', direction: nameById[id] || id, to: id });
});
// 2) 各建築首格 → 夜鴞巷
ENTRANCE_IDS.forEach((id) => {
  newExits.push({ from: id, direction: '大街', to: 'yaxiaolane' });
});
// 3) 各逃生口 → 夜鴞巷
ESCAPE_IDS.forEach((id) => {
  newExits.push({ from: id, direction: '大街', to: 'yaxiaolane' });
});

// 4) 建築內部鏈：依 doc 連接節點，線性鏈 + 少數分支
// 商鋪1: 01-02-03-04-05-06-07-08-09-10, 10-11, 10-12-13-14-15-16-17-18-19-20-21-22-23-24-25
function pad(n) { return n < 10 ? '0' + n : String(n); }
function linkIds(ida, idb) {
  const a = 'yaxiao_' + pad(ida);
  const b = 'yaxiao_' + pad(idb);
  if (nameById[a] && nameById[b]) {
    newExits.push({ from: a, direction: nameById[b], to: b });
    newExits.push({ from: b, direction: nameById[a], to: a });
  }
}

// 商鋪1: 01~25
for (let i = 1; i <= 24; i++) linkIds(i, i + 1);
linkIds(10, 12); // 10 還連 12（10-11 已由上面 loop 產生）

// 商鋪2: 26~43
for (let i = 26; i <= 42; i++) linkIds(i, i + 1);
linkIds(33, 34);
linkIds(33, 35);
linkIds(35, 36);

// 商鋪3: 44~73
for (let i = 44; i <= 72; i++) linkIds(i, i + 1);
linkIds(54, 55);
linkIds(54, 56);
linkIds(62, 63);
linkIds(62, 66);
linkIds(64, 65);
linkIds(66, 67);

// 商鋪4: 74~95
for (let i = 74; i <= 94; i++) linkIds(i, i + 1);
linkIds(82, 83);
linkIds(82, 85);
linkIds(83, 84);
linkIds(85, 86);

// 商鋪5: 96~115
for (let i = 96; i <= 114; i++) linkIds(i, i + 1);
linkIds(102, 103);
linkIds(102, 105);
linkIds(105, 104);
linkIds(105, 106);

// 住宅1 巷口六號: 116~127
for (let i = 116; i <= 126; i++) linkIds(i, i + 1);
linkIds(117, 118);
linkIds(117, 119);
linkIds(117, 120);
linkIds(119, 120);
linkIds(120, 121);

// 住宅2 雜物間: 128~137
for (let i = 128; i <= 136; i++) linkIds(i, i + 1);

// 住宅3 張氏寓所: 138~152
for (let i = 138; i <= 151; i++) linkIds(i, i + 1);
linkIds(139, 140);
linkIds(139, 141);
linkIds(139, 142);

// 住宅4 管道維護站: 153~165
for (let i = 153; i <= 164; i++) linkIds(i, i + 1);
linkIds(154, 155);
linkIds(156, 157);
linkIds(157, 158);

// 住宅5 空置房: 166~173
for (let i = 166; i <= 172; i++) linkIds(i, i + 1);

// 住宅6 舊瓦舍: 174~184
for (let i = 174; i <= 183; i++) linkIds(i, i + 1);
linkIds(175, 176);
linkIds(176, 177);

// 住宅7 街角一樓: 185~198
for (let i = 185; i <= 197; i++) linkIds(i, i + 1);
linkIds(186, 187);
linkIds(187, 188);
linkIds(188, 189);
linkIds(187, 190);
linkIds(189, 190);
linkIds(190, 191);

// 住宅8 後巷平房: 199~207
for (let i = 199; i <= 206; i++) linkIds(i, i + 1);

// 住宅9 石牆側室: 208~219
for (let i = 208; i <= 218; i++) linkIds(i, i + 1);
linkIds(209, 210);
linkIds(210, 211);

// 住宅10 窄門寓所: 220~229
for (let i = 220; i <= 228; i++) linkIds(i, i + 1);
linkIds(221, 222);
linkIds(222, 223);

// 寫回：在 "yaxiaolane" -> "lifestreet" 那一行後面插入 newExits
const raw = fs.readFileSync(ROOMS_JSON, 'utf8');
const marker = '    { "from": "yaxiaolane",       "direction": "浮生", "to": "lifestreet" }\n  ]';
const lines = newExits.map((e) => '    { "from": "' + e.from + '", "direction": "' + e.direction.replace(/"/g, '\\"') + '", "to": "' + e.to + '" }');
const insert = lines.join(',\n');
const newRaw = raw.replace(marker, '    { "from": "yaxiaolane",       "direction": "浮生", "to": "lifestreet" },\n' + insert + '\n  ]');
fs.writeFileSync(ROOMS_JSON, newRaw);
console.log('Added', newExits.length, 'exits.');
console.log('Sample:', JSON.stringify(newExits.slice(0, 4), null, 2));
