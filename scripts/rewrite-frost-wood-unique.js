#!/usr/bin/env node
/**
 * Rewrite 霜林 fs_wood + fs_cabin descriptions: UNIQUE per room, body ≥50 chars,
 * 可經 from exits. Uses OPEN + MID + END combo so no two rooms share the same body.
 * Run: node scripts/rewrite-frost-wood-unique.js
 */

const fs = require('fs');
const path = require('path');

const ROOT = path.join(process.cwd(), 'data/rooms/霜林');
const TRAIL_IDS = new Set(['fs_trail_01','fs_trail_02','fs_trail_03','fs_trail_04','fs_trail_05','fs_trail_06','fs_trail_07','fs_trail_08','fs_trail_09','fs_trail_10']);

// 開頭（一句入畫）
const OPEN = [
  '側枝林緣，', '小徑旁岔：', '林隙深處，', '徑外窪地，', '樹根盤踞的坡側，', '狹窄石壁間，',
  '鉛皮與黑岩夾峙的側谷，', '斷層煙氣自腳下溢出，', '半透明晶體圍欄隆起，', '晶化石脊一側，',
  '巨木枝幹垂掛流蘇般晶簇，', '林道旁枝：', '厚霜粉覆地，', '熱氣溝壑中白霧翻騰，',
  '窪地霜粉填平石槽，', '霧氣與落霜揉雜成雨霧，', '岩壁擠壓成狹長通道，', '石壁緊縮僅容側身，',
  '晶體圍欄隆起於地表，', '黑曜石板平整如鏡，', '白玉石板鋪地、熱氣均勻，', '林緣湖氣猶在，',
  '側谷霧氣翻騰，', '石紋沿徑延伸，', '枝幹垂掛晶簇隨熱氣搖曳，', '奧所建築輪廓浮現，',
  '導向石柱在霧氣擾動間，', '地熱自裂縫擠出，', '強風在石縫間往復盤旋，', '霜粉被橫風吹成白線，'
];

// 中段（感官、材質、聲觸）
const MID = [
  '樹皮掛滿水滴、與湖面霧氣交織，地表泥土未完全封凍，枯黃草莖滑膩。',
  '細碎白霜與枯葉鋪地，地熱自石縫滲出，游離輻射下熱氣不散，足底石面沁涼。',
  '落霜觸地即融，晶簇在枝幹間隱約，風穿林隙帶礦物味，熱氣與湖氣在此交會。',
  '霜粉覆在樹根盤踞的土坎上，空氣乾燥帶嘶嘶細響，導向石柱若隱若現。',
  '鏽斑與白色晶簇並生，高處偶有晶塊崩落，金屬面傳來沉悶撞擊。',
  '與高空落霜揉雜成黏性雨霧，視距僅數步，斑駁石柱標示徑路。',
  '厚霜粉鬆散乾燥，光線在棱面間折射，熱氣自縫隙偶爾浮升。',
  '熱氣溝壑中白霧翻騰，晶體磨擦聲細密不絕。',
  '黑曜石板縫隙規整，沉香油脂味與熱輻射彌漫，足底溫潤。',
  '巨木合圍成拱頂，落霜在半空緩慢盤旋，風穿枝隙聲低。',
  '湖氣與濕潤熱氣未散，兩側樹木稀疏，冷調石紋沒入霧中，足底輕軟。',
  '強風在石縫間迴盪，踩踏乾裂霜粉的摩擦聲往深處遞遠。',
  '白霜在根緣硬化成薄殼，地底熱流與寒氣交鋒，嘶嘶細響自石縫滲出。',
  '路面發白，樹脂香濃重，霧氣在前方翻湧。',
  '鋸齒邊緣沁涼，橫風與脊線上熱氣交織，物體邊緣偶見冷紫色弧光。',
  '巨木晶簇隨熱氣搖曳，足底石面溫潤。',
  '霜晶與枯葉交織，游離輻射下熱氣貼地浮升，人跡徑路向主徑折回。',
  '足底石面沁涼，熱氣與霜霧纏腳。',
  '質地鬆散乾燥，光線折射成冷白，踏之無聲。',
  '晶體爆裂聲細微，弧光在邊緣閃爍。',
  '風在岩壁間迴盪成低頻沉響，枯葉與細碎冰屑鋪滿徑緣。',
  '石板縫隙規整，沉香與地底熱輻射彌漫，足底溫潤。',
  '視距僅數步，導向石柱斑駁，熱氣自斷層裂隙溢出。',
  '地熱與寒氣交鋒的嘶嘶聲自石縫滲出，空氣帶礦物味。',
  '高處晶塊偶爾崩落撞擊金屬面，空氣折射扭曲。',
  '樹根遮蔽天光，地熱將路面烤白，樹脂香濃重、霧氣翻湧。',
  '光線在棱面折射成冷白，踏地無聲、熱氣偶爾浮升。',
  '溝壑白霧翻騰，晶體磨擦爆裂聲不絕。',
  '巨木高聳、晶簇流蘇般垂掛，熱輻射與沉香油脂味恆定。',
  '巨木合圍拱頂，落霜半空盤旋，風穿枝隙、足底沁涼。',
];

// 結尾（收束一句；避免與 MID 句尾重複，不重複「足底石面沁涼」「踏之無聲」等）
const END = [
  '熱氣自腳下縫隙浮升。', '人跡徑路在此折回主徑。', '此段為小徑周邊側景。',
  '導向石柱在霧氣中若隱若現。', '霜晶與湖氣交會。', '空氣乾燥帶礦物味。',
  '風穿林隙、聲往深處遞遠。', '厚牆隔開湖側濕冷。', '徑路向主徑折回。',
  '熱氣與霜霧纏腳。',
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

const PAD = ['游離輻射下熱氣不散，霜晶與地熱並存。', '此段人跡徑路旁側，林相與主徑有別。', '厚霜粉與枯葉鋪地，足底輕軟。'];

// 產生 126+ 組不重複的 (o, m, e) 索引，確保每間房 body 唯一
function* uniqueTriples(limit) {
  let n = 0;
  for (let e = 0; e < END.length && n < limit; e++)
    for (let m = 0; m < MID.length && n < limit; m++)
      for (let o = 0; o < OPEN.length && n < limit; o++) {
        yield [o, m, e];
        n++;
      }
}

const allPaths = walk(ROOT);
const withId = [];
for (const absPath of allPaths) {
  try {
    const data = JSON.parse(fs.readFileSync(absPath, 'utf8'));
    if (data.id && !TRAIL_IDS.has(data.id)) withId.push({ absPath, id: data.id });
  } catch (e) { /* skip */ }
}
withId.sort((a, b) => a.id.localeCompare(b.id));
const triples = [...uniqueTriples(Math.max(withId.length, 150))];
let done = 0, skip = 0;

for (let i = 0; i < withId.length; i++) {
  const { absPath, id } = withId[i];
  const content = fs.readFileSync(absPath, 'utf8');
  let data;
  try { data = JSON.parse(content); } catch (e) { skip++; continue; }
  if (data.id !== id || TRAIL_IDS.has(data.id)) { skip++; continue; }

  const [o, m, e] = triples[i];
  let body = OPEN[o] + MID[m] + END[e];
  if (body.length < 50) body += PAD[o % PAD.length];
  if (body.length < 50) body += PAD[m % PAD.length];

  const exits = data.exits || [];
  const dirs = exits.map((x) => x.direction).filter(Boolean);
  const kejing = dirs.length ? '可經' + dirs.map((d) => '〔' + d + '〕').join('') + '。' : '';
  data.description = body + kejing;

  try {
    fs.writeFileSync(absPath, JSON.stringify(data, null, 2), 'utf8');
    done++;
  } catch (err) {
    console.error(id, err.message);
  }
}
console.log('Rewritten:', done, 'Skipped:', skip);
