#!/usr/bin/env node
/**
 * 與 format-rooms-pretty.js 同為 2 空格根層縮排，但將 exits[] 與 objects[] 內
 * 每個物件的屬性各自換行，與向陽八段等檔案風格一致。
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

function indentBlock(str, spaces) {
  const pad = ' '.repeat(spaces);
  return str.split('\n').map(line => pad + line).join('\n');
}

function formatRoom(room) {
  const lines = ['{'];
  const keys = Object.keys(room);
  for (let i = 0; i < keys.length; i++) {
    const k = keys[i];
    const v = room[k];
    const comma = i < keys.length - 1 ? ',' : '';
    if (k === 'exits' && Array.isArray(v)) {
      lines.push('  "exits": [');
      v.forEach((e, j) => {
        const block = JSON.stringify(e, null, 2);
        lines.push(indentBlock(block, 4));
        if (j < v.length - 1) lines[lines.length - 1] += ',';
      });
      lines.push('  ]' + comma);
    } else if (k === 'objects' && Array.isArray(v)) {
      lines.push('  "objects": [');
      v.forEach((o, j) => {
        const block = JSON.stringify(o, null, 2);
        lines.push(indentBlock(block, 4));
        if (j < v.length - 1) lines[lines.length - 1] += ',';
      });
      lines.push('  ]' + comma);
    } else {
      lines.push('  ' + JSON.stringify(k) + ': ' + JSON.stringify(v) + comma);
    }
  }
  lines.push('}');
  return lines.join('\n') + '\n';
}

const files = listJsonFiles(ROOTS_DIR);
let ok = 0;
let err = 0;

for (const f of files) {
  try {
    const raw = fs.readFileSync(f, 'utf8');
    const data = JSON.parse(raw);
    const pretty = formatRoom(data);
    fs.writeFileSync(f, pretty, 'utf8');
    ok++;
  } catch (e) {
    console.error('Skip:', f, e.message);
    err++;
  }
}

console.log('Formatted', ok, 'room files (exits/objects expanded). Errors:', err);
