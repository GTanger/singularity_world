#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");

const ROOT = path.resolve(__dirname, "..", "..");
const ROOMS_DIR = path.join(ROOT, "data", "rooms", "editor");

const FULLWIDTH_DIGITS = {
  "0": "０",
  "1": "１",
  "2": "２",
  "3": "３",
  "4": "４",
  "5": "５",
  "6": "６",
  "7": "７",
  "8": "８",
  "9": "９",
};

function toFullwidthNumber(value) {
  return String(value)
    .split("")
    .map((ch) => FULLWIDTH_DIGITS[ch] || ch)
    .join("");
}

function replaceDeep(value, floor) {
  if (Array.isArray(value)) return value.map((item) => replaceDeep(item, floor));
  if (value && typeof value === "object") {
    const out = {};
    for (const [k, v] of Object.entries(value)) out[k] = replaceDeep(v, floor);
    return out;
  }
  if (typeof value !== "string") return value;

  const floorCode = String(floor).padStart(2, "0");
  const fullFloorCode = toFullwidthNumber(floorCode);
  const fullFloorSingle = toFullwidthNumber(String(floor));

  return value
    .replace(/city_life_4f/g, `city_life_${floor}f`)
    .replace(/_u04([1-9])/g, `_u${floorCode}$1`)
    .replace(/０４([１-９])/g, `${fullFloorCode}$1`)
    .replace(/浮生城4F/g, `浮生城${floor}F`)
    .replace(/浮生城4樓/g, `浮生城${floor}樓`)
    .replace(/浮生城４樓/g, `浮生城${fullFloorSingle}樓`);
}

function pruneStairForFloor(room, floor) {
  // 10F is top floor in current layout, remove "up stairs".
  if (floor === 10) {
    room.exits = (room.exits || []).filter((e) => e.direction !== "上樓梯");
    room.objects = (room.objects || []).filter((o) => o.name !== "上樓梯");
  }
}

function fixStairTargets(room, floor) {
  const upTarget = floor < 10 ? `city_life_${floor + 1}f` : null;
  const downTarget = `city_life_${floor - 1}f`;

  for (const exit of room.exits || []) {
    if (exit.direction === "上樓梯" && upTarget) exit.to = upTarget;
    if (exit.direction === "下樓梯") exit.to = downTarget;
  }

  for (const obj of room.objects || []) {
    if (obj.name === "上樓梯" && upTarget) {
      obj.id = upTarget;
      obj.move_to_room_id = upTarget;
      if (obj.responses && obj.responses.Move) {
        obj.responses.Move = `你前往「浮生城${floor + 1}F」。`;
      }
    }
    if (obj.name === "下樓梯") {
      obj.id = downTarget;
      obj.move_to_room_id = downTarget;
      if (obj.responses && obj.responses.Move) {
        obj.responses.Move = `你前往「浮生城${floor - 1}F」。`;
      }
    }
  }
}

function main() {
  const baseFiles = fs
    .readdirSync(ROOMS_DIR)
    .filter((name) => /^city_life_4f.*\.json$/.test(name))
    .sort();

  if (baseFiles.length === 0) {
    throw new Error("No 4F template files found.");
  }

  let written = 0;

  for (let floor = 5; floor <= 10; floor += 1) {
    const floorCode = String(floor).padStart(2, "0");

    for (const sourceName of baseFiles) {
      const sourcePath = path.join(ROOMS_DIR, sourceName);
      const sourceJson = JSON.parse(fs.readFileSync(sourcePath, "utf8"));

      let transformed = replaceDeep(sourceJson, floor);
      const targetName = sourceName
        .replace("city_life_4f", `city_life_${floor}f`)
        .replace(/_u04([1-9])/, `_u${floorCode}$1`);

      if (targetName === `city_life_${floor}f.json`) {
        fixStairTargets(transformed, floor);
        pruneStairForFloor(transformed, floor);
      }

      const targetPath = path.join(ROOMS_DIR, targetName);
      fs.writeFileSync(targetPath, `${JSON.stringify(transformed, null, 2)}\n`, "utf8");
      written += 1;
    }
  }

  console.log(`OK: cloned 4F template to 5F-10F (${written} files written)`);
}

main();
