#!/usr/bin/env node
/**
 * 依一巷模板生成打鐵三巷 4 商 (shop09~12) + 2 宅 (home08~09)，並更新 ironland_lane3.json。
 * 執行：node tools/js/gen_ironland_lane3_rooms.js
 */

const fs = require('fs');
const path = require('path');

const LANE1 = path.join(__dirname, '..', 'data', 'rooms', '打鐵巷', '打鐵一巷');
const LANE3 = path.join(__dirname, '..', 'data', 'rooms', '打鐵巷', '打鐵三巷');

const SHOP_NAMES = { 9: '鍛錘鋪', 10: '爐膛鋪', 11: '鐵屑鋪', 12: '風箱鋪' };
const SHOP_ROOMS = ['main', 'workshop', 'workshop2', 'storage', 'living', 'bedroom', 'kiln', 'toolroom', 'kitchen', 'courtyard'];
const HOME_NAMES = { 8: '簷下宅', 9: '巷底宅' };
const HOME_ROOMS = ['main', 'hallway', 'living', 'kitchen', 'courtyard', 'bedroom'];

function shopPrefix(n) {
  return n < 10 ? `ironland_shop0${n}` : `ironland_shop${n}`;
}
function homePrefix(n) {
  return n < 10 ? `ironland_home0${n}` : `ironland_home${n}`;
}

function repl(raw, fromId, toId, fromName, toName) {
  let s = raw.replace(new RegExp(fromId.replace(/\./g, '\\.'), 'g'), toId);
  if (fromName && toName) s = s.replace(new RegExp(fromName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), toName);
  return s;
}

// ---------- 商鋪 (09~12) ----------
for (let n = 9; n <= 12; n++) {
  const prefix = shopPrefix(n);
  const name = SHOP_NAMES[n];
  const dir = path.join(LANE3, name);
  for (const room of SHOP_ROOMS) {
    const id = `${prefix}_${room}`;
    const src = path.join(LANE1, '鍛造鋪', `ironland_shop01_${room}.json`);
    const dest = path.join(dir, `${id}.json`);
    let raw = fs.readFileSync(src, 'utf8');
    raw = repl(raw, 'ironland_shop01', prefix, '鍛造鋪', name);
    raw = repl(raw, 'ironland_lane1', 'ironland_lane3', '打鐵一巷', '打鐵三巷');
    fs.writeFileSync(dest, raw, 'utf8');
  }
}

// ---------- 住宅 (08~09) ----------
for (let n = 8; n <= 9; n++) {
  const prefix = homePrefix(n);
  const name = HOME_NAMES[n];
  const dir = path.join(LANE3, name);
  for (const room of HOME_ROOMS) {
    const id = `${prefix}_${room}`;
    const src = path.join(LANE1, '巷內宅', `ironland_home01_${room}.json`);
    const dest = path.join(dir, `${id}.json`);
    let raw = fs.readFileSync(src, 'utf8');
    raw = repl(raw, 'ironland_home01', prefix, null, null);
    raw = repl(raw, 'ironland_lane1', 'ironland_lane3', '打鐵一巷', '打鐵三巷');
    raw = raw.replace(/巷內宅門廳/g, `${name}門廳`);
    fs.writeFileSync(dest, raw, 'utf8');
  }
}

// ---------- 更新 lane3 ----------
const lane3Path = path.join(LANE3, 'ironland_lane3.json');
const lane3 = JSON.parse(fs.readFileSync(lane3Path, 'utf8'));

const shopMains = [9, 10, 11, 12].map(n => ({ id: `${shopPrefix(n)}_main`, name: SHOP_NAMES[n] }));
const homeMains = [8, 9].map(n => ({ id: `${homePrefix(n)}_main`, name: HOME_NAMES[n] }));

lane3.description = '巷尾即風道出口，自一巷沿風道流瀉而來的熱氣在此釋出、往飛霜大街四段而去。兩側牆面暗紅脈絡隱約。可經〔打鐵二巷〕折返，或自〔巷尾〕步出至飛霜大街四段；亦可入三巷四商二宅：〔鍛錘鋪〕〔爐膛鋪〕〔鐵屑鋪〕〔風箱鋪〕、〔簷下宅〕〔巷底宅〕。';

lane3.exits = [
  { direction: 'ironland_lane2', to: 'ironland_lane2', ui_hidden: true },
  { direction: 'feistreet_st4', to: 'feistreet_st4', ui_hidden: true },
  ...shopMains.map(s => ({ direction: s.id, to: s.id, ui_hidden: true })),
  ...homeMains.map(h => ({ direction: h.id, to: h.id, ui_hidden: true }))
];

lane3.objects = [
  { id: 'ironland_lane3_exit_lane2', name: '打鐵二巷', sockets: ['Move'], move_to_room_id: 'ironland_lane2', responses: { Move: '你逆著熱風往浮生方向走去，進入打鐵二巷。' } },
  { id: 'ironland_lane3_exit_fei', name: '巷尾', sockets: ['Move'], move_to_room_id: 'feistreet_st4', responses: { Move: '你步出巷尾，熱氣在身後湧向飛霜大街四段。' } },
  ...shopMains.map(s => ({ id: `ironland_lane3_exit_${s.id}`, name: s.name, sockets: ['Move'], move_to_room_id: s.id, responses: { Move: `你步入${s.name}。` } })),
  ...homeMains.map(h => ({ id: `ironland_lane3_exit_${h.id}`, name: h.name, sockets: ['Move'], move_to_room_id: h.id, responses: { Move: `你走向${h.name}。` } }))
];

fs.writeFileSync(lane3Path, JSON.stringify(lane3, null, 2) + '\n', 'utf8');

console.log('Lane3: 4 shops (shop09~12) + 2 homes (home08~09) generated; ironland_lane3.json updated.');