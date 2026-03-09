#!/usr/bin/env node
/**
 * 依一巷模板生成打鐵二巷 5 商 (shop04~08) + 5 宅 (home03~07)，並更新 ironland_lane2.json。
 * 執行：node scripts/gen_ironland_lane2_rooms.js
 */

const fs = require('fs');
const path = require('path');

const LANE1 = path.join(__dirname, '..', 'data', 'rooms', '打鐵巷', '打鐵一巷');
const LANE2 = path.join(__dirname, '..', 'data', 'rooms', '打鐵巷', '打鐵二巷');

const SHOP_NAMES = { 4: '鐵脊鋪', 5: '焰心鋪', 6: '砧聲鋪', 7: '熔流鋪', 8: '鍛骨鋪' };
const SHOP_ROOMS = ['main', 'workshop', 'workshop2', 'storage', 'living', 'bedroom', 'kiln', 'toolroom', 'kitchen', 'courtyard'];
const HOME_NAMES = { 3: '灰磚宅', 4: '窄廊宅', 5: '簷角宅', 6: '石階宅', 7: '巷尾宅' };
const HOME_ROOMS = ['main', 'hallway', 'living', 'kitchen', 'courtyard', 'bedroom'];

function repl(raw, fromId, toId, fromName, toName) {
  let s = raw.replace(new RegExp(fromId.replace(/\./g, '\\.'), 'g'), toId);
  if (fromName && toName) s = s.replace(new RegExp(fromName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), toName);
  return s;
}

// ---------- 商鋪 ----------
for (let n = 4; n <= 8; n++) {
  const prefix = `ironland_shop0${n}`;
  const name = SHOP_NAMES[n];
  const dir = path.join(LANE2, name);
  for (const room of SHOP_ROOMS) {
    const id = `${prefix}_${room}`;
    const src = path.join(LANE1, '鍛造鋪', `ironland_shop01_${room}.json`);
    const dest = path.join(dir, `${id}.json`);
    let raw = fs.readFileSync(src, 'utf8');
    raw = repl(raw, 'ironland_shop01', prefix, '鍛造鋪', name);
    raw = repl(raw, 'ironland_lane1', 'ironland_lane2', '打鐵一巷', '打鐵二巷');
    fs.writeFileSync(dest, raw, 'utf8');
  }
}

// ---------- 住宅 ----------
for (let n = 3; n <= 7; n++) {
  const prefix = `ironland_home0${n}`;
  const name = HOME_NAMES[n];
  const dir = path.join(LANE2, name);
  for (const room of HOME_ROOMS) {
    const id = `${prefix}_${room}`;
    const src = path.join(LANE1, '巷內宅', `ironland_home01_${room}.json`);
    const dest = path.join(dir, `${id}.json`);
    let raw = fs.readFileSync(src, 'utf8');
    raw = repl(raw, 'ironland_home01', prefix, null, null);
    raw = repl(raw, 'ironland_lane1', 'ironland_lane2', '打鐵一巷', '打鐵二巷');
    raw = raw.replace(/巷內宅門廳/g, `${name}門廳`);
    fs.writeFileSync(dest, raw, 'utf8');
  }
}

// ---------- 更新 lane2 ----------
const lane2Path = path.join(LANE2, 'ironland_lane2.json');
const lane2 = JSON.parse(fs.readFileSync(lane2Path, 'utf8'));

const shopMains = [4, 5, 6, 7, 8].map(n => ({ id: `ironland_shop0${n}_main`, name: SHOP_NAMES[n] }));
const homeMains = [3, 4, 5, 6, 7].map(n => ({ id: `ironland_home0${n}_main`, name: HOME_NAMES[n] }));

lane2.description = '巷身收束為風道，兩側牆面暗紅脈絡爬滿。自一巷被風帶來的熱氣與鐵腥貫穿此段、往飛霜端流瀉。此處可經〔打鐵一巷〕折返浮生方向，或朝〔打鐵三巷〕往飛霜大街；亦可入二巷五商五宅：〔鐵脊鋪〕〔焰心鋪〕〔砧聲鋪〕〔熔流鋪〕〔鍛骨鋪〕、〔灰磚宅〕〔窄廊宅〕〔簷角宅〕〔石階宅〕〔巷尾宅〕。';

lane2.exits = [
  { direction: 'ironland_lane1', to: 'ironland_lane1', ui_hidden: true },
  { direction: 'ironland_lane3', to: 'ironland_lane3', ui_hidden: true },
  ...shopMains.map(s => ({ direction: s.id, to: s.id, ui_hidden: true })),
  ...homeMains.map(h => ({ direction: h.id, to: h.id, ui_hidden: true }))
];

lane2.objects = [
  { id: 'ironland_lane2_exit_lane1', name: '打鐵一巷', sockets: ['Move'], move_to_room_id: 'ironland_lane1', responses: { Move: '你逆著熱風往浮生方向走去，進入打鐵一巷。' } },
  { id: 'ironland_lane2_exit_lane3', name: '打鐵三巷', sockets: ['Move'], move_to_room_id: 'ironland_lane3', responses: { Move: '你順風往飛霜方向走去，進入打鐵三巷。' } },
  ...shopMains.map(s => ({ id: `ironland_lane2_exit_${s.id}`, name: s.name, sockets: ['Move'], move_to_room_id: s.id, responses: { Move: `你步入${s.name}。` } })),
  ...homeMains.map(h => ({ id: `ironland_lane2_exit_${h.id}`, name: h.name, sockets: ['Move'], move_to_room_id: h.id, responses: { Move: `你走向${h.name}。` } }))
];

fs.writeFileSync(lane2Path, JSON.stringify(lane2, null, 2) + '\n', 'utf8');

console.log('Lane2: 5 shops (shop04~08) + 5 homes (home03~07) generated; ironland_lane2.json updated.');