#!/usr/bin/env node
/**
 * Copy 霜林 from rooms_archive to data/rooms, fix trail_01→fei_lake_north,
 * add ui_hidden to exits and objects (Move) for each exit.
 * Run from project root: node scripts/copy-frost-wood-connect.js
 */

const fs = require('fs');
const path = require('path');

const ARCHIVE = 'data/rooms_archive/霜林';
const TARGET = 'data/rooms/霜林';

function walkDir(dir, fileList = []) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) walkDir(full, fileList);
    else if (e.isFile() && e.name.endsWith('.json')) fileList.push(full);
  }
  return fileList;
}

function sanitizeId(s) {
  return s.replace(/[^a-zA-Z0-9_]/g, '_');
}

const files = walkDir(ARCHIVE);
console.log(`Found ${files.length} room files.`);

let trail01Fixed = 0;
for (const absPath of files) {
  const rel = path.relative(ARCHIVE, absPath);
  const targetPath = path.join(TARGET, rel);
  const dir = path.dirname(targetPath);
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

  let data;
  try {
    data = JSON.parse(fs.readFileSync(absPath, 'utf8'));
  } catch (e) {
    console.error('Read error', absPath, e.message);
    continue;
  }

  const id = data.id || path.basename(absPath, '.json');
  const exits = data.exits || [];
  const newExits = [];
  const objects = [];

  for (const ex of exits) {
    let to = ex.to;
    let direction = ex.direction;
    if (id === 'fs_trail_01' && to === 'feishuang_lake_road') {
      to = 'fei_lake_north';
      direction = '湖背';
      trail01Fixed++;
    }
    newExits.push({
      direction: direction,
      to: to,
      ui_hidden: true
    });
    const objId = id + '_to_' + sanitizeId(to);
    objects.push({
      id: objId,
      name: direction,
      sockets: ['Move'],
      move_to_room_id: to,
      responses: { Move: '你往' + direction + '走去。' }
    });
  }

  data.exits = newExits;
  data.objects = objects;
  data.description = data.description || ''; // keep for later rewrite

  fs.writeFileSync(targetPath, JSON.stringify(data, null, 2), 'utf8');
}

console.log('Trail_01 exit fixed to fei_lake_north:', trail01Fixed);
console.log('Written to', TARGET);
