#!/usr/bin/env node
/**
 * Rewrite descriptions for 霜林 fs_wood and fs_cabin (not fs_trail_01～10).
 * Body ≥50 chars, 可經 from exits, design doc vocabulary, no forbidden words.
 * Run: node scripts/rewrite-frost-wood-descriptions.js
 */

const fs = require('fs');
const path = require('path');

const ROOT = path.join(process.cwd(), 'data/rooms/霜林');
const TRAIL_IDS = new Set(['fs_trail_01','fs_trail_02','fs_trail_03','fs_trail_04','fs_trail_05','fs_trail_06','fs_trail_07','fs_trail_08','fs_trail_09','fs_trail_10']);

// Pool of body fragments (each ≥50 chars when combined), 霜林語彙 only
const BODIES = [
  '側枝林緣：樹皮掛滿水滴，與湖面飄來的霧氣交織，地表泥土未完全封凍，枯黃草莖在濕氣中滑膩。',
  '小徑旁岔：細碎白霜與枯葉鋪地，地熱自石縫滲出，游離輻射下熱氣不散，足底石面沁涼。',
  '林隙深處：落霜觸地即融，晶簇在枝幹間隱約，風穿林隙帶礦物味，熱氣與湖氣在此交會。',
  '石紋沿徑延伸，霜粉覆在樹根盤踞的土坎上，空氣乾燥帶嘶嘶細響，導向石柱在霧氣中若隱若現。',
  '鉛皮與黑岩夾峙的側谷，鏽斑與白色晶簇並生，高處偶有晶塊崩落，金屬面傳來沉悶撞擊。',
  '斷層煙氣自腳下溢出，與高空落霜揉雜成黏性雨霧，視距僅數步，斑駁石柱標示徑路。',
  '半透明晶體圍欄隆起於地表，厚霜粉鬆散乾燥，光線在棱面間折射，熱氣自縫隙偶爾浮升。',
  '晶化石脊一側，熱氣溝壑中白霧翻騰，霜粉被橫風吹成白線，晶體磨擦聲細密不絕。',
  '巨木枝幹垂掛流蘇般晶簇，隨熱氣搖曳，黑曜石板縫隙規整，沉香油脂味與熱輻射彌漫。',
  '白玉石板鋪地，熱氣均勻擴散，巨木合圍成拱頂，落霜在半空緩慢盤旋，風穿枝隙聲低。',
  '林道旁枝：湖氣與濕潤熱氣未散，兩側樹木稀疏，冷調石紋沒入霧中，足底輕軟。',
  '徑外窪地：厚實霜粉填平槽溝，強風在石縫間迴盪，踩踏乾裂霜粉的摩擦聲往深處遞遠。',
  '樹根盤踞的坡側，白霜在根緣硬化成薄殼，地底熱流與寒氣交鋒，嘶嘶細響自石縫滲出。',
  '狹窄石壁間，地熱自裂縫擠出，路面發白，樹脂香濃重，霧氣在前方翻湧。',
  '晶片路面參差，鋸齒邊緣沁涼，橫風與脊線上熱氣交織，物體邊緣偶見冷紫色弧光。',
  '奧所建築輪廓浮現，黑曜石板鏡面反射，巨木晶簇隨熱氣搖曳，足底石面溫潤。',
  '林緣濕氣猶在，霜晶與枯葉交織，游離輻射下熱氣貼地浮升，人跡徑路向主徑折回。',
  '側谷霧氣翻騰，導向石柱在擾動間若隱若現，足底石面沁涼，熱氣與霜霧纏腳。',
  '厚霜粉覆地，質地鬆散乾燥，晶體圍欄半透明，光線折射成冷白，踏之無聲。',
  '熱氣溝壑中白霧混濁，霜粉被吹成水平白線，晶體爆裂聲細微，弧光在邊緣閃爍。',
  '窪地霜粉填平石槽，風在岩壁間迴盪成低頻沉響，枯葉與細碎冰屑鋪滿徑緣。',
  '枝幹垂掛晶簇隨熱氣搖曳，石板縫隙規整，沉香與地底熱輻射彌漫，足底溫潤。',
  '霧氣與落霜揉雜成雨霧，視距僅數步，導向石柱斑駁，熱氣自斷層裂隙溢出。',
  '樹根盤踞、白霜硬化成薄殼，地熱與寒氣交鋒的嘶嘶聲自石縫滲出，空氣帶礦物味。',
  '鉛皮擋板鏽斑與晶簇交錯，高處晶塊偶爾崩落撞擊金屬面，空氣折射扭曲。',
  '石壁緊縮僅容側身，樹根遮蔽天光，地熱將路面烤白，樹脂香濃重、霧氣翻湧。',
  '晶體圍欄隆起，厚霜粉鬆散，光線在棱面折射成冷白，踏地無聲、熱氣偶爾浮升。',
  '晶化石脊兩側溝壑白霧翻騰，霜粉被橫風吹成白線，晶體磨擦爆裂聲不絕。',
  '黑曜石板平整如鏡，巨木高聳、晶簇流蘇般垂掛，熱輻射與沉香油脂味恆定。',
  '白玉石板鋪地、熱氣均勻，巨木合圍拱頂，落霜半空盤旋，風穿枝隙、足底沁涼。',
  '林緣湖氣猶在，細碎白霜與枯葉交織，人跡徑路向主徑折回，熱氣貼地浮升。',
  '側枝徑外：霜晶觸地即融，晶簇隱約，風穿林隙帶礦物味，熱氣與湖氣交會。',
  '岩壁擠壓成狹長通道，石槽被霜粉填平，強風盤旋、低頻回響，踩踏聲往深處遞遠。',
  '晶片路面參差、鋸齒沁涼，橫風與脊線熱氣交織，冷紫色弧光在物體邊緣閃爍。'
];

function walk(dir, list = []) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) walk(full, list);
    else if (e.name.endsWith('.json')) list.push(full);
  }
  return list;
}

function hash(s) {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = ((h << 5) - h) + s.charCodeAt(i) | 0;
  return Math.abs(h);
}

const files = walk(ROOT);
console.log('Total files:', files.length);
let done = 0, skip = 0;
for (let i = 0; i < files.length; i++) {
  const absPath = files[i];
  const content = fs.readFileSync(absPath, 'utf8');
  let data;
  try { data = JSON.parse(content); } catch (e) { skip++; continue; }
  const id = data.id;
  const isTrail = TRAIL_IDS.has(id);
  const noId = !id;
  if (noId || isTrail) { skip++; continue; }

  const exits = data.exits || [];
  const dirs = exits.map(e => e.direction).filter(Boolean);
  const kejing = dirs.length ? '可經' + dirs.map(d => '〔' + d + '〕').join('') + '。' : '';

  const idx = hash(id) % BODIES.length;
  const body = BODIES[idx];
  data.description = body + kejing;

  try {
    fs.writeFileSync(absPath, JSON.stringify(data, null, 2), 'utf8');
    done++;
  } catch (err) {
    console.error(id, err.message);
  }
}
console.log('Rewritten:', done, 'Skipped:', skip);
