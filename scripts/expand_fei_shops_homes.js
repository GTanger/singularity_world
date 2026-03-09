#!/usr/bin/env node
/**
 * 飛霜大街 商／宅 擴充為多室：
 * 商：main + 儲物間 + 後間（共 3 室）
 * 宅：main(門廳) + 廊道 + 生息室 + 灶間 + 小院 + 臥房（共 6 室）
 * 依 data/rooms 目錄內現有 fei_shop*_main / fei_home*_main 所在目錄寫入新檔並更新 main。
 */
const fs = require('fs');
const path = require('path');

const ROOTS = [path.join(__dirname, '..', 'data', 'rooms', '飛霜大街')];

function* walkDir(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) yield* walkDir(full);
    else if (e.isFile() && e.name.endsWith('.json')) yield full;
  }
}

function readJson(p) {
  return JSON.parse(fs.readFileSync(p, 'utf8'));
}
function writeJson(p, obj) {
  fs.writeFileSync(p, JSON.stringify(obj, null, 2) + '\n', 'utf8');
}

// --- 商：main + 儲物間 + 後間 ---
const shopMainPaths = [];
for (const root of ROOTS) {
  if (!fs.existsSync(root)) continue;
  for (const f of walkDir(root)) {
    const name = path.basename(f);
    if (/^fei_shop\d+_main\.json$/.test(name)) shopMainPaths.push(f);
  }
}

for (const mainPath of shopMainPaths.sort()) {
  const dir = path.dirname(mainPath);
  const idMatch = path.basename(mainPath).match(/^fei_shop(\d+)_main\.json$/);
  if (!idMatch) continue;
  const num = idMatch[1];
  const prefix = `fei_shop${num}`;
  const main = readJson(mainPath);
  const streetId = main.exits.find(e => e.to && e.to.startsWith('feistreet_'))?.to || 'feistreet_st1';

  // 更新 main：加 儲物間、後間
  if (!main.exits.some(e => e.to === `${prefix}_storage`)) {
    main.exits.push({ direction: `${prefix}_storage`, to: `${prefix}_storage`, ui_hidden: true });
    main.exits.push({ direction: `${prefix}_back`, to: `${prefix}_back`, ui_hidden: true });
    main.objects = main.objects || [];
    main.objects.push({
      id: `${prefix}_main_exit_storage`,
      name: '儲物間',
      sockets: ['Move'],
      move_to_room_id: `${prefix}_storage`,
      responses: { Move: '你朝後方通道走去，進入儲物間。' }
    });
    main.objects.push({
      id: `${prefix}_main_exit_back`,
      name: '後間',
      sockets: ['Move'],
      move_to_room_id: `${prefix}_back`,
      responses: { Move: '你往後間走去。' }
    });
    writeJson(mainPath, main);
  }

  const storagePath = path.join(dir, `${prefix}_storage.json`);
  if (!fs.existsSync(storagePath)) {
    writeJson(storagePath, {
      id: `${prefix}_storage`,
      name: '儲物間',
      description: '簷角積霜、格架與牆面沁出玉質脈絡，堆放雜貨與陶罐。冷調天光自通道漏入。此處通〔店頭〕與〔後間〕。',
      tags: [],
      zone: 'feistreet',
      exits: [
        { direction: `${prefix}_main`, to: `${prefix}_main`, ui_hidden: true },
        { direction: `${prefix}_back`, to: `${prefix}_back`, ui_hidden: true }
      ],
      objects: [
        { id: `${prefix}_storage_exit_main`, name: '店頭', sockets: ['Move'], move_to_room_id: `${prefix}_main`, responses: { Move: '你循來路回到店頭。' } },
        { id: `${prefix}_storage_exit_back`, name: '後間', sockets: ['Move'], move_to_room_id: `${prefix}_back`, responses: { Move: '你往後間走去。' } }
      ]
    });
  }

  const backPath = path.join(dir, `${prefix}_back.json`);
  if (!fs.existsSync(backPath)) {
    writeJson(backPath, {
      id: `${prefix}_back`,
      name: '後間',
      description: '後間冷調、簷下霜晶隱約，牆面玉質脈絡沁出。此處僅通〔儲物間〕。',
      tags: [],
      zone: 'feistreet',
      exits: [{ direction: `${prefix}_storage`, to: `${prefix}_storage`, ui_hidden: true }],
      objects: [
        { id: `${prefix}_back_exit_storage`, name: '儲物間', sockets: ['Move'], move_to_room_id: `${prefix}_storage`, responses: { Move: '你回到儲物間。' } }
      ]
    });
  }
}

// --- 宅：main + 廊道 + 生息室 + 灶間 + 小院 + 臥房 ---
const homeMainPaths = [];
for (const root of ROOTS) {
  if (!fs.existsSync(root)) continue;
  for (const f of walkDir(root)) {
    const name = path.basename(f);
    if (/^fei_home\d+_main\.json$/.test(name)) homeMainPaths.push(f);
  }
}

for (const mainPath of homeMainPaths.sort()) {
  const dir = path.dirname(mainPath);
  const idMatch = path.basename(mainPath).match(/^fei_home(\d+)_main\.json$/);
  if (!idMatch) continue;
  const num = idMatch[1];
  const prefix = `fei_home${num}`;

  const main = readJson(mainPath);
  if (!main.exits.some(e => e.to === `${prefix}_hallway`)) {
    main.exits.push({ direction: `${prefix}_hallway`, to: `${prefix}_hallway`, ui_hidden: true });
    main.objects = main.objects || [];
    main.objects.push({
      id: `${prefix}_main_exit_hallway`,
      name: '廊道',
      sockets: ['Move'],
      move_to_room_id: `${prefix}_hallway`,
      responses: { Move: '你朝內側走去，進入廊道。' }
    });
    writeJson(mainPath, main);
  }

  const rooms = [
    {
      file: `${prefix}_hallway.json`,
      id: `${prefix}_hallway`,
      name: '廊道',
      description: '廊道冷調、簷角積霜，牆面玉質脈絡隱約。此處通〔門廳〕、〔生息室〕與〔小院〕。',
      exits: [{ direction: `${prefix}_main`, to: `${prefix}_main`, ui_hidden: true }, { direction: `${prefix}_living`, to: `${prefix}_living`, ui_hidden: true }, { direction: `${prefix}_courtyard`, to: `${prefix}_courtyard`, ui_hidden: true }],
      objects: [
        { id: `${prefix}_hallway_exit_main`, name: '門廳', sockets: ['Move'], move_to_room_id: `${prefix}_main`, responses: { Move: '你循來路回到門廳。' } },
        { id: `${prefix}_hallway_exit_living`, name: '生息室', sockets: ['Move'], move_to_room_id: `${prefix}_living`, responses: { Move: '你朝通道深處走去，進入生息室。' } },
        { id: `${prefix}_hallway_exit_courtyard`, name: '小院', sockets: ['Move'], move_to_room_id: `${prefix}_courtyard`, responses: { Move: '你踏入小院。' } }
      ]
    },
    {
      file: `${prefix}_living.json`,
      id: `${prefix}_living`,
      name: '生息室',
      description: '生息室冷調、石几貼地，簷下霜晶隱約、牆縫玉質脈絡。此處通〔廊道〕、〔灶間〕與〔內臥房〕。',
      exits: [{ direction: `${prefix}_hallway`, to: `${prefix}_hallway`, ui_hidden: true }, { direction: `${prefix}_kitchen`, to: `${prefix}_kitchen`, ui_hidden: true }, { direction: `${prefix}_bedroom`, to: `${prefix}_bedroom`, ui_hidden: true }],
      objects: [
        { id: `${prefix}_living_exit_hallway`, name: '廊道', sockets: ['Move'], move_to_room_id: `${prefix}_hallway`, responses: { Move: '你回到廊道。' } },
        { id: `${prefix}_living_exit_kitchen`, name: '灶間', sockets: ['Move'], move_to_room_id: `${prefix}_kitchen`, responses: { Move: '你步入灶間。' } },
        { id: `${prefix}_living_exit_bedroom`, name: '內臥房', sockets: ['Move'], move_to_room_id: `${prefix}_bedroom`, responses: { Move: '你步入內臥房。' } }
      ]
    },
    {
      file: `${prefix}_kitchen.json`,
      id: `${prefix}_kitchen`,
      name: '灶間',
      description: '灶間冷調、石灶與簷角積霜，牆根玉質脈絡隱約。此處僅通〔生息室〕。',
      exits: [{ direction: `${prefix}_living`, to: `${prefix}_living`, ui_hidden: true }],
      objects: [{ id: `${prefix}_kitchen_exit_living`, name: '生息室', sockets: ['Move'], move_to_room_id: `${prefix}_living`, responses: { Move: '你回到生息室。' } }]
    },
    {
      file: `${prefix}_courtyard.json`,
      id: `${prefix}_courtyard`,
      name: '小院',
      description: '小院簷下、泥地苔衣與霜晶並存，牆根苔綠。此處僅通〔廊道〕。',
      exits: [{ direction: `${prefix}_hallway`, to: `${prefix}_hallway`, ui_hidden: true }],
      objects: [{ id: `${prefix}_courtyard_exit_hallway`, name: '廊道', sockets: ['Move'], move_to_room_id: `${prefix}_hallway`, responses: { Move: '你回到廊道。' } }]
    },
    {
      file: `${prefix}_bedroom.json`,
      id: `${prefix}_bedroom`,
      name: '內臥房',
      description: '臥房冷調、簷下霜晶隱約，牆角玉質微光。此處僅通〔生息室〕。',
      exits: [{ direction: `${prefix}_living`, to: `${prefix}_living`, ui_hidden: true }],
      objects: [{ id: `${prefix}_bedroom_exit_living`, name: '生息室', sockets: ['Move'], move_to_room_id: `${prefix}_living`, responses: { Move: '你離開臥房，回到生息室。' } }]
    }
  ];

  for (const r of rooms) {
    const outPath = path.join(dir, r.file);
    if (fs.existsSync(outPath)) continue;
    const { file, ...rest } = r;
    writeJson(outPath, { ...rest, tags: [], zone: 'feistreet' });
  }
}

console.log('Done: 商 儲物間+後間、宅 廊道+生息室+灶間+小院+臥房 已擴充。');
