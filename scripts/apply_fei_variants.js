#!/usr/bin/env node
/**
 * 依 fei_building_variants.json 對飛霜大街每棟商/宅套用「結構＋描述」變體，
 * 避免整條街複製貼上。會覆寫既有房間 JSON 的 description 與結構（exits/objects）。
 */
const fs = require('fs');
const path = require('path');

const ROOT = path.join(__dirname, '..', 'data', 'rooms', '飛霜大街');
const VARIANTS_PATH = path.join(ROOT, 'fei_building_variants.json');

const variants = JSON.parse(fs.readFileSync(VARIANTS_PATH, 'utf8'));

function* walkDir(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) yield* walkDir(full);
    else if (e.isFile() && e.name.endsWith('.json') && !e.name.includes('variants')) yield full;
  }
}

function readJson(p) {
  return JSON.parse(fs.readFileSync(p, 'utf8'));
}
function writeJson(p, obj) {
  fs.writeFileSync(p, JSON.stringify(obj, null, 2) + '\n', 'utf8');
}

// --- 找出各 main 路徑與對應的 streetId ---
const shopDirs = new Map();
const homeDirs = new Map();
for (const f of walkDir(ROOT)) {
  const name = path.basename(f);
  const dir = path.dirname(f);
  if (/^fei_shop(\d+)_main\.json$/.test(name)) {
    const n = parseInt(RegExp.$1, 10);
    const main = readJson(f);
    const streetId = main.exits.find(e => e.to && e.to.startsWith('feistreet_'))?.to || 'feistreet_st1';
    shopDirs.set(n, { dir, streetId, main });
  } else if (/^fei_home(\d+)_main\.json$/.test(name)) {
    const n = parseInt(RegExp.$1, 10);
    const main = readJson(f);
    const streetId = main.exits.find(e => e.to && e.to.startsWith('feistreet_'))?.to || 'feistreet_st1';
    homeDirs.set(n, { dir, streetId, main });
  }
}

// --- 商：依 structure 與描述變體覆寫 ---
const shopStruct = variants.shop.structure;
const shopMainDesc = variants.shop.mainDesc;
const shopStorageDesc = variants.shop.storageDesc;
const shopBackDesc = variants.shop.backDesc;
const shopWorkshopDesc = variants.shop.workshopDesc;

for (const [num, { dir, streetId, main }] of shopDirs) {
  const prefix = `fei_shop${String(num).padStart(2, '0')}`;
  const i = num - 1;
  const struct = shopStruct[i] || 'main_storage_back';

  const seg = (streetId.match(/feistreet_st(\d+)/) || [])[1];
  const streetLabel = seg ? `飛霜大街${['','一','二','三','四','五','六','七','八','九'][seg] || seg}段` : '飛霜大街';
  main.description = (shopMainDesc[i] || shopMainDesc[0]).replace('此處連接外部的〔飛霜大街〕。', `此處連接外部的〔${streetLabel}〕。`);

  main.exits = main.exits.filter(e => e.to === streetId || e.to === `${prefix}_storage` || (struct.includes('back') && e.to === `${prefix}_back`) || (struct.includes('workshop') && e.to === `${prefix}_workshop`));
  main.objects = main.objects.filter(o => !o.move_to_room_id || o.move_to_room_id === streetId || o.move_to_room_id === `${prefix}_storage` || (struct.includes('back') && o.move_to_room_id === `${prefix}_back`) || (struct.includes('workshop') && o.move_to_room_id === `${prefix}_workshop`));

  if (!main.objects.some(o => o.move_to_room_id === `${prefix}_storage`)) {
    main.objects.push({ id: `${prefix}_main_exit_storage`, name: '儲物間', sockets: ['Move'], move_to_room_id: `${prefix}_storage`, responses: { Move: '你朝後方通道走去，進入儲物間。' } });
    main.exits.push({ direction: `${prefix}_storage`, to: `${prefix}_storage`, ui_hidden: true });
  }
  if (struct.includes('back') && !main.objects.some(o => o.move_to_room_id === `${prefix}_back`)) {
    main.objects.push({ id: `${prefix}_main_exit_back`, name: '後間', sockets: ['Move'], move_to_room_id: `${prefix}_back`, responses: { Move: '你往後間走去。' } });
    main.exits.push({ direction: `${prefix}_back`, to: `${prefix}_back`, ui_hidden: true });
  }
  if (struct.includes('workshop') && !main.objects.some(o => o.move_to_room_id === `${prefix}_workshop`)) {
    main.objects.push({ id: `${prefix}_main_exit_workshop`, name: '工間', sockets: ['Move'], move_to_room_id: `${prefix}_workshop`, responses: { Move: '你往工間走去。' } });
    main.exits.push({ direction: `${prefix}_workshop`, to: `${prefix}_workshop`, ui_hidden: true });
  }

  writeJson(path.join(dir, `${prefix}_main.json`), main);

  const storagePath = path.join(dir, `${prefix}_storage.json`);
  const storageDesc = shopStorageDesc[i] || shopStorageDesc[0];
  const storageExits = [{ direction: `${prefix}_main`, to: `${prefix}_main`, ui_hidden: true }];
  const storageObjs = [{ id: `${prefix}_storage_exit_main`, name: '店頭', sockets: ['Move'], move_to_room_id: `${prefix}_main`, responses: { Move: '你循來路回到店頭。' } }];
  if (struct.includes('back')) {
    storageExits.push({ direction: `${prefix}_back`, to: `${prefix}_back`, ui_hidden: true });
    storageObjs.push({ id: `${prefix}_storage_exit_back`, name: '後間', sockets: ['Move'], move_to_room_id: `${prefix}_back`, responses: { Move: '你往後間走去。' } });
  }
  if (struct.includes('workshop')) {
    storageExits.push({ direction: `${prefix}_workshop`, to: `${prefix}_workshop`, ui_hidden: true });
    storageObjs.push({ id: `${prefix}_storage_exit_workshop`, name: '工間', sockets: ['Move'], move_to_room_id: `${prefix}_workshop`, responses: { Move: '你往工間走去。' } });
  }
  writeJson(storagePath, {
    id: `${prefix}_storage`,
    name: '儲物間',
    description: storageDesc + (struct.includes('back') ? ' 此處通〔店頭〕與〔後間〕。' : struct.includes('workshop') ? ' 此處通〔店頭〕與〔工間〕。' : ' 此處僅通〔店頭〕。'),
    tags: [],
    zone: 'feistreet',
    exits: storageExits,
    objects: storageObjs
  });

  const backPath = path.join(dir, `${prefix}_back.json`);
  const workshopPath = path.join(dir, `${prefix}_workshop.json`);
  if (struct.includes('back')) {
    writeJson(backPath, {
      id: `${prefix}_back`,
      name: '後間',
      description: (shopBackDesc[i % shopBackDesc.length] || shopBackDesc[0]),
      tags: [],
      zone: 'feistreet',
      exits: [{ direction: `${prefix}_storage`, to: `${prefix}_storage`, ui_hidden: true }],
      objects: [{ id: `${prefix}_back_exit_storage`, name: '儲物間', sockets: ['Move'], move_to_room_id: `${prefix}_storage`, responses: { Move: '你回到儲物間。' } }]
    });
  } else if (fs.existsSync(backPath)) fs.unlinkSync(backPath);

  if (struct.includes('workshop')) {
    writeJson(workshopPath, {
      id: `${prefix}_workshop`,
      name: '工間',
      description: (num === 3 ? shopWorkshopDesc[0] : num === 5 || num === 15 ? shopWorkshopDesc[1] : shopWorkshopDesc[0]),
      tags: [],
      zone: 'feistreet',
      exits: [{ direction: `${prefix}_storage`, to: `${prefix}_storage`, ui_hidden: true }],
      objects: [{ id: `${prefix}_workshop_exit_storage`, name: '儲物間', sockets: ['Move'], move_to_room_id: `${prefix}_storage`, responses: { Move: '你回到儲物間。' } }]
    });
  } else if (fs.existsSync(workshopPath)) fs.unlinkSync(workshopPath);
}

// --- 宅：依 structure 與描述變體覆寫 ---
const homeStruct = variants.home.structure;
const homeMainDesc = variants.home.mainDesc;
const homeHallwayDesc = variants.home.hallwayDesc;
const homeLivingDesc = variants.home.livingDesc;
const homeKitchenDesc = variants.home.kitchenDesc;
const homeCourtyardDesc = variants.home.courtyardDesc;
const homeBedroomDesc = variants.home.bedroomDesc;

for (const [num, { dir, streetId, main }] of homeDirs) {
  const prefix = `fei_home${String(num).padStart(2, '0')}`;
  const i = num - 1;
  const struct = homeStruct[i] || '6_standard';

  main.description = homeMainDesc[i] || homeMainDesc[0];
  const streetLabel = streetId.replace('feistreet_', '飛霜大街');

  if (struct === '5_no_hallway') {
    main.exits = main.exits.filter(e => e.to === streetId || e.to === `${prefix}_living` || e.to === `${prefix}_courtyard`);
    main.objects = main.objects.filter(o => !o.move_to_room_id || o.move_to_room_id === streetId || o.move_to_room_id === `${prefix}_living` || o.move_to_room_id === `${prefix}_courtyard`);
    if (!main.objects.some(o => o.move_to_room_id === `${prefix}_living`)) {
      main.objects.push({ id: `${prefix}_main_exit_living`, name: '生息室', sockets: ['Move'], move_to_room_id: `${prefix}_living`, responses: { Move: '你朝內側走去，進入生息室。' } });
      main.objects.push({ id: `${prefix}_main_exit_courtyard`, name: '小院', sockets: ['Move'], move_to_room_id: `${prefix}_courtyard`, responses: { Move: '你踏入小院。' } });
      main.exits.push({ direction: `${prefix}_living`, to: `${prefix}_living`, ui_hidden: true }, { direction: `${prefix}_courtyard`, to: `${prefix}_courtyard`, ui_hidden: true });
    }
    const hallwayPath = path.join(dir, `${prefix}_hallway.json`);
    if (fs.existsSync(hallwayPath)) fs.unlinkSync(hallwayPath);
  } else {
    main.exits = main.exits.filter(e => e.to === streetId || e.to === `${prefix}_hallway`);
    main.objects = main.objects.filter(o => !o.move_to_room_id || o.move_to_room_id === streetId || o.move_to_room_id === `${prefix}_hallway`);
    if (!main.objects.some(o => o.move_to_room_id === `${prefix}_hallway`)) {
      main.objects.push({ id: `${prefix}_main_exit_hallway`, name: '廊道', sockets: ['Move'], move_to_room_id: `${prefix}_hallway`, responses: { Move: '你朝內側走去，進入廊道。' } });
      main.exits.push({ direction: `${prefix}_hallway`, to: `${prefix}_hallway`, ui_hidden: true });
    }
  }
  writeJson(path.join(dir, `${prefix}_main.json`), main);

  const hasCourtyard = struct !== '5_no_courtyard';
  const hasHallway = struct !== '5_no_hallway';

  if (hasHallway) {
    const hallwayExits = [{ direction: `${prefix}_main`, to: `${prefix}_main`, ui_hidden: true }, { direction: `${prefix}_living`, to: `${prefix}_living`, ui_hidden: true }];
    const hallwayObjs = [
      { id: `${prefix}_hallway_exit_main`, name: '門廳', sockets: ['Move'], move_to_room_id: `${prefix}_main`, responses: { Move: '你循來路回到門廳。' } },
      { id: `${prefix}_hallway_exit_living`, name: '生息室', sockets: ['Move'], move_to_room_id: `${prefix}_living`, responses: { Move: '你朝通道深處走去，進入生息室。' } }
    ];
    if (hasCourtyard) {
      hallwayExits.push({ direction: `${prefix}_courtyard`, to: `${prefix}_courtyard`, ui_hidden: true });
      hallwayObjs.push({ id: `${prefix}_hallway_exit_courtyard`, name: '小院', sockets: ['Move'], move_to_room_id: `${prefix}_courtyard`, responses: { Move: '你踏入小院。' } });
    }
    writeJson(path.join(dir, `${prefix}_hallway.json`), {
      id: `${prefix}_hallway`,
      name: '廊道',
      description: (homeHallwayDesc[i] || homeHallwayDesc[0]).replace('、〔生息室〕與〔小院〕。', hasCourtyard ? '、〔生息室〕與〔小院〕。' : '、〔生息室〕。'),
      tags: [],
      zone: 'feistreet',
      exits: hallwayExits,
      objects: hallwayObjs
    });
  }

  writeJson(path.join(dir, `${prefix}_living.json`), {
    id: `${prefix}_living`,
    name: '生息室',
    description: homeLivingDesc[i] || homeLivingDesc[0],
    tags: [],
    zone: 'feistreet',
    exits: hasHallway
      ? [{ direction: `${prefix}_hallway`, to: `${prefix}_hallway`, ui_hidden: true }, { direction: `${prefix}_kitchen`, to: `${prefix}_kitchen`, ui_hidden: true }, { direction: `${prefix}_bedroom`, to: `${prefix}_bedroom`, ui_hidden: true }]
      : [{ direction: `${prefix}_main`, to: `${prefix}_main`, ui_hidden: true }, { direction: `${prefix}_kitchen`, to: `${prefix}_kitchen`, ui_hidden: true }, { direction: `${prefix}_bedroom`, to: `${prefix}_bedroom`, ui_hidden: true }],
    objects: hasHallway
      ? [
          { id: `${prefix}_living_exit_hallway`, name: '廊道', sockets: ['Move'], move_to_room_id: `${prefix}_hallway`, responses: { Move: '你回到廊道。' } },
          { id: `${prefix}_living_exit_kitchen`, name: '灶間', sockets: ['Move'], move_to_room_id: `${prefix}_kitchen`, responses: { Move: '你步入灶間。' } },
          { id: `${prefix}_living_exit_bedroom`, name: '內臥房', sockets: ['Move'], move_to_room_id: `${prefix}_bedroom`, responses: { Move: '你步入內臥房。' } }
        ]
      : [
          { id: `${prefix}_living_exit_main`, name: '門廳', sockets: ['Move'], move_to_room_id: `${prefix}_main`, responses: { Move: '你回到門廳。' } },
          { id: `${prefix}_living_exit_kitchen`, name: '灶間', sockets: ['Move'], move_to_room_id: `${prefix}_kitchen`, responses: { Move: '你步入灶間。' } },
          { id: `${prefix}_living_exit_bedroom`, name: '內臥房', sockets: ['Move'], move_to_room_id: `${prefix}_bedroom`, responses: { Move: '你步入內臥房。' } }
        ]
  });

  writeJson(path.join(dir, `${prefix}_kitchen.json`), {
    id: `${prefix}_kitchen`,
    name: '灶間',
    description: homeKitchenDesc[i] || homeKitchenDesc[0],
    tags: [],
    zone: 'feistreet',
    exits: [{ direction: `${prefix}_living`, to: `${prefix}_living`, ui_hidden: true }],
    objects: [{ id: `${prefix}_kitchen_exit_living`, name: '生息室', sockets: ['Move'], move_to_room_id: `${prefix}_living`, responses: { Move: '你回到生息室。' } }]
  });

  if (hasCourtyard) {
    writeJson(path.join(dir, `${prefix}_courtyard.json`), {
      id: `${prefix}_courtyard`,
      name: '小院',
      description: homeCourtyardDesc[i] || homeCourtyardDesc[0],
      tags: [],
      zone: 'feistreet',
      exits: [{ direction: hasHallway ? `${prefix}_hallway` : `${prefix}_main`, to: hasHallway ? `${prefix}_hallway` : `${prefix}_main`, ui_hidden: true }],
      objects: [{ id: `${prefix}_courtyard_exit`, name: hasHallway ? '廊道' : '門廳', sockets: ['Move'], move_to_room_id: hasHallway ? `${prefix}_hallway` : `${prefix}_main`, responses: { Move: hasHallway ? '你回到廊道。' : '你回到門廳。' } }]
    });
  } else {
    const courtyardPath = path.join(dir, `${prefix}_courtyard.json`);
    if (fs.existsSync(courtyardPath)) fs.unlinkSync(courtyardPath);
  }

  writeJson(path.join(dir, `${prefix}_bedroom.json`), {
    id: `${prefix}_bedroom`,
    name: '內臥房',
    description: homeBedroomDesc[i] || homeBedroomDesc[0],
    tags: [],
    zone: 'feistreet',
    exits: [{ direction: `${prefix}_living`, to: `${prefix}_living`, ui_hidden: true }],
    objects: [{ id: `${prefix}_bedroom_exit_living`, name: '生息室', sockets: ['Move'], move_to_room_id: `${prefix}_living`, responses: { Move: '你離開臥房，回到生息室。' } }]
  });
}

console.log('apply_fei_variants: 商/宅 結構與描述變體已套用。');
console.log('商: 2室(主+儲)/3室+後間/3室+工間 混用。宅: 5室無小院/5室無廊道/6室標準 混用。');
