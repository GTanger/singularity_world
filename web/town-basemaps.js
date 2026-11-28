/* 奇點世界｜銀鷗島五大核心城鎮手繪水彩羊皮紙輿圖對位表（各鎮獨立地形與聚落輪廓，嚴禁複刻白鷗港八角城） */
(function () {
  'use strict';

  var VER = '20261128r6';

  window.TOWN_ILLUSTRATED_MAPS = {
    // 1. 白鷗港：五區滿版 Qwen-Image-2512 20B 簡約暖陽水彩繪本街區圖（跨區換圖、零拼接，對齊七步四迴路正典動線）
    white_gull_harbor: {
      slug: 'white_gull_harbor',
      title: '白鷗港手繪水彩海港輿圖',
      imgUrl: '/maps/white_gull_harbor.webp?v=' + VER,
      cropMode: 'wg_legacy',
      bgDay: '#ede2ca',
      bgNight: '#14191e',
      districts: [
        {
          id: 'wgh_d1_harbor',
          title: '白鷗港・深水港池',
          imgUrl: '/maps/wgh_d1_harbor.webp?v=' + VER,
          width: 1024,
          height: 768,
          bgDay: '#cfe3e6',
          bgNight: '#101b22',
          glows: [[878, 128, 76], [356, 224, 82], [156, 612, 68], [458, 616, 75]],
          members: [
            'oi:white_gull_harbor:dock',
            'oi:white_gull_harbor:north_lighthouse',
            'oi:white_gull_harbor:linjin_bridge',
            'oi:white_gull_harbor:south_dock',
            'oi:white_gull_harbor:tally_house'
          ],
          pills: [
            { nid: 'oi:white_gull_harbor:dock', label: '港務北棧', x: 356, y: 224, kind: 'main' },
            { nid: 'oi:white_gull_harbor:north_lighthouse', label: '北防波堤燈塔', x: 878, y: 178, kind: 'main' },
            { nid: 'oi:white_gull_harbor:linjin_bridge', label: '臨津石拱橋', x: 132, y: 345, kind: 'street' },
            { nid: 'oi:white_gull_harbor:south_dock', label: '港務南棧', x: 458, y: 616, kind: 'main' },
            { nid: 'oi:white_gull_harbor:tally_house', label: '港務記籌寮屋', x: 156, y: 612, kind: 'shop' },
            { nid: 'oi:white_gull_harbor:market_avenue', label: '白鷗市集館', x: 168, y: 128, kind: 'street' },
            { nid: 'oi:white_gull_harbor:white_gull_bridge', label: '白鷗大橋', x: 64, y: 376, kind: 'street' },
            { nid: 'oi:white_gull_harbor:tavern_st', label: '車馬驛站街', x: 198, y: 704, kind: 'street' }
          ]
        },
        {
          id: 'wgh_d2_plaza',
          title: '白鷗港・市集廣場',
          imgUrl: '/maps/wgh_d2_plaza.webp?v=' + VER,
          width: 1024,
          height: 768,
          bgDay: '#ede2ca',
          bgNight: '#161b1f',
          glows: [[472, 412, 110], [248, 224, 85], [762, 390, 95]],
          members: [
            'oi:white_gull_harbor:plaza',
            'oi:white_gull_harbor:market_avenue',
            'oi:white_gull_harbor:citadel_gate',
            'oi:white_gull_harbor:civic_hall',
            'oi:white_gull_harbor:cooper_ln',
            'oi:white_gull_harbor:rope_ln'
          ],
          pills: [
            { nid: 'oi:white_gull_harbor:plaza', label: '市集廣場', x: 472, y: 412, kind: 'main' },
            { nid: 'oi:white_gull_harbor:citadel_gate', label: '內城要塞軍堡', x: 248, y: 224, kind: 'main' },
            { nid: 'oi:white_gull_harbor:civic_hall', label: '市政廳銀號', x: 198, y: 108, kind: 'shop' },
            { nid: 'oi:white_gull_harbor:market_avenue', label: '白鷗市集館', x: 762, y: 390, kind: 'main' },
            { nid: 'oi:white_gull_harbor:cooper_ln', label: '桶匠巷', x: 846, y: 176, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:rope_ln', label: '繩索巷', x: 136, y: 548, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:craft_south_st', label: '百工南街', x: 466, y: 70, kind: 'street' },
            { nid: 'oi:white_gull_harbor:dock', label: '港務北棧', x: 916, y: 636, kind: 'street' },
            { nid: 'oi:white_gull_harbor:white_gull_bridge', label: '白鷗大橋', x: 474, y: 704, kind: 'street' },
            { nid: 'oi:white_gull_harbor:west_watergate_bridge', label: '箍桶便橋', x: 92, y: 698, kind: 'street' }
          ]
        },
        {
          id: 'wgh_d3_north',
          title: '白鷗港・北城百工',
          imgUrl: '/maps/wgh_d3_north.webp?v=' + VER,
          width: 1024,
          height: 768,
          bgDay: '#ebe0c6',
          bgNight: '#161a19',
          glows: [[284, 326, 95], [710, 562, 85], [480, 184, 65]],
          members: [
            'oi:white_gull_harbor:north_gate',
            'oi:white_gull_harbor:smithy_st',
            'oi:white_gull_harbor:bal_forge',
            'oi:white_gull_harbor:craft_south_st',
            'oi:white_gull_harbor:herb_hall',
            'oi:white_gull_harbor:iron_anvil_ln',
            'oi:white_gull_harbor:sail_ln',
            'oi:white_gull_harbor:copper_ln',
            'oi:white_gull_harbor:salted_fish_ln'
          ],
          pills: [
            { nid: 'oi:white_gull_harbor:north_gate', label: '北城門', x: 480, y: 184, kind: 'main' },
            { nid: 'oi:white_gull_harbor:smithy_st', label: '鐵匠街', x: 474, y: 346, kind: 'street' },
            { nid: 'oi:white_gull_harbor:bal_forge', label: '巴爾鍛鐵鋪', x: 284, y: 326, kind: 'shop' },
            { nid: 'oi:white_gull_harbor:sail_ln', label: '船帆巷', x: 118, y: 328, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:iron_anvil_ln', label: '鐵砧巷', x: 812, y: 304, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:craft_south_st', label: '百工南街', x: 466, y: 566, kind: 'street' },
            { nid: 'oi:white_gull_harbor:herb_hall', label: '苦艾草藥堂', x: 710, y: 562, kind: 'shop' },
            { nid: 'oi:white_gull_harbor:salted_fish_ln', label: '鹹魚巷', x: 138, y: 608, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:copper_ln', label: '銅幣巷', x: 912, y: 578, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:citadel_gate', label: '內城要塞軍堡', x: 282, y: 704, kind: 'street' },
            { nid: 'oi:white_gull_harbor:plaza', label: '市集廣場', x: 466, y: 712, kind: 'main' }
          ]
        },
        {
          id: 'wgh_d4_river',
          title: '白鷗港・澤昂河畔',
          imgUrl: '/maps/wgh_d4_river.webp?v=' + VER,
          width: 1024,
          height: 768,
          bgDay: '#d7e6e1',
          bgNight: '#121b20',
          glows: [[302, 578, 95], [532, 348, 80], [758, 574, 65]],
          members: [
            'oi:white_gull_harbor:west_watergate_bridge',
            'oi:white_gull_harbor:white_gull_bridge',
            'oi:white_gull_harbor:river_tavern_st',
            'oi:white_gull_harbor:rusty_anchor_tavern',
            'oi:white_gull_harbor:inn_upstairs',
            'oi:white_gull_harbor:lantern_ln',
            'oi:white_gull_harbor:seabreeze_ln'
          ],
          pills: [
            { nid: 'oi:white_gull_harbor:west_watergate_bridge', label: '箍桶便橋', x: 228, y: 414, kind: 'main' },
            { nid: 'oi:white_gull_harbor:white_gull_bridge', label: '白鷗大橋', x: 532, y: 348, kind: 'main' },
            { nid: 'oi:white_gull_harbor:river_tavern_st', label: '臨河酒館街', x: 522, y: 574, kind: 'street' },
            { nid: 'oi:white_gull_harbor:rusty_anchor_tavern', label: '鏽錨酒館', x: 302, y: 578, kind: 'shop' },
            { nid: 'oi:white_gull_harbor:inn_upstairs', label: '酒館二樓宿房', x: 168, y: 636, kind: 'shop' },
            { nid: 'oi:white_gull_harbor:seabreeze_ln', label: '海風巷', x: 138, y: 704, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:lantern_ln', label: '石燈巷', x: 758, y: 574, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:plaza', label: '市集廣場', x: 554, y: 78, kind: 'main' },
            { nid: 'oi:white_gull_harbor:rope_ln', label: '繩索巷', x: 256, y: 94, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:linjin_bridge', label: '臨津石拱橋', x: 902, y: 232, kind: 'street' },
            { nid: 'oi:white_gull_harbor:tavern_st', label: '車馬驛站街', x: 516, y: 712, kind: 'street' }
          ]
        },
        {
          id: 'wgh_d5_south',
          title: '白鷗港・南城驛站',
          imgUrl: '/maps/wgh_d5_south.webp?v=' + VER,
          width: 1024,
          height: 768,
          bgDay: '#ede2ca',
          bgNight: '#161a1c',
          glows: [[648, 242, 95], [392, 466, 75], [786, 536, 70]],
          members: [
            'oi:white_gull_harbor:tavern_st',
            'oi:white_gull_harbor:gale_carriage',
            'oi:white_gull_harbor:old_well_ln',
            'oi:white_gull_harbor:south_wall_ln',
            'oi:white_gull_harbor:south_gate'
          ],
          pills: [
            { nid: 'oi:white_gull_harbor:tavern_st', label: '車馬驛站街', x: 408, y: 302, kind: 'street' },
            { nid: 'oi:white_gull_harbor:gale_carriage', label: '疾風驛馬行', x: 648, y: 242, kind: 'shop' },
            { nid: 'oi:white_gull_harbor:old_well_ln', label: '老井巷', x: 786, y: 536, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:south_wall_ln', label: '南牆根巷', x: 194, y: 346, kind: 'alley' },
            { nid: 'oi:white_gull_harbor:south_gate', label: '南城門', x: 392, y: 466, kind: 'main' },
            { nid: 'oi:white_gull_harbor:river_tavern_st', label: '臨河酒館街', x: 406, y: 74, kind: 'street' },
            { nid: 'oi:white_gull_harbor:south_dock', label: '港務南棧', x: 846, y: 266, kind: 'street' }
          ]
        }
      ],
      glows: [
        [246, 306, 85], [340, 598, 105], [252, 885, 95],
        [645, 598, 80], [816, 320, 68], [342, 202, 58], [340, 1068, 58]
      ],
      pills: [
        { nid: 'oi:white_gull_harbor:north_gate', label: '北城門', x: 342, y: 162, gx: 342, gy: 162, kind: 'main', mxy: [342, 202] },
        { nid: 'oi:white_gull_harbor:smithy_st', label: '鐵匠街', x: 342, y: 348, gx: 342, gy: 348, kind: 'street', mxy: [342, 345] },
        { nid: 'oi:white_gull_harbor:plaza', label: '市集廣場', x: 340, y: 560, gx: 340, gy: 560, kind: 'main', mxy: [340, 598] },
        { nid: 'oi:white_gull_harbor:tavern_st', label: '驛站街', x: 340, y: 828, gx: 340, gy: 828, kind: 'street', mxy: [340, 825] },
        { nid: 'oi:white_gull_harbor:south_gate', label: '南城門', x: 340, y: 1114, gx: 340, gy: 1114, kind: 'main', mxy: [340, 1068] },
        { nid: 'oi:white_gull_harbor:dock', label: '港務碼頭', x: 725, y: 605, gx: 736, gy: 605, kind: 'main', mxy: [645, 598] },
        { nid: 'oi:white_gull_harbor:bal_forge', label: '巴爾鍛鐵鋪', x: 242, y: 382, gx: 248, gy: 376, kind: 'shop', mxy: [248, 306] },
        { nid: 'oi:white_gull_harbor:herb_hall', label: '苦艾草藥堂', x: 456, y: 382, gx: 448, gy: 376, kind: 'shop', mxy: [445, 306] },
        { nid: 'oi:white_gull_harbor:rusty_anchor_tavern', label: '鏽錨酒館', atlasLabel: '鏽錨酒館(一樓)', x: 244, y: 974, gx: 254, gy: 976, kind: 'shop', mxy: [252, 885] },
        { nid: 'oi:white_gull_harbor:inn_upstairs', label: '酒館二樓宿房', x: 244, y: 1012, kind: 'alley', atlasOnly: true, mxy: [252, 885] },
        { nid: 'oi:white_gull_harbor:gale_carriage', label: '疾風驛馬行', x: 468, y: 845, gx: 462, gy: 840, kind: 'shop', mxy: [462, 898] }
      ],
      alleys: [
        { nid: 'oi:white_gull_harbor:iron_anvil_ln', name: '鐵砧巷', lx: 500, ly: 424, startIdx: 0, maxIdx: 12, p0: [390, 455], p1: [555, 396] },
        { nid: 'oi:white_gull_harbor:sail_ln', name: '船帆巷', lx: 146, ly: 368, startIdx: 0, maxIdx: 12, p0: [185, 396], p1: [118, 326] },
        { nid: 'oi:white_gull_harbor:copper_ln', name: '銅幣巷', lx: 526, ly: 506, startIdx: 0, maxIdx: 12, p0: [455, 482], p1: [575, 522] },
        { nid: 'oi:white_gull_harbor:salted_fish_ln', name: '鹹魚巷', lx: 206, ly: 446, startIdx: 0, maxIdx: 12, p0: [305, 465], p1: [114, 425] },
        { nid: 'oi:white_gull_harbor:cooper_ln', name: '桶匠巷', lx: 502, ly: 668, startIdx: 0, maxIdx: 12, p0: [405, 628], p1: [568, 695] },
        { nid: 'oi:white_gull_harbor:rope_ln', name: '繩索巷', lx: 176, ly: 600, startIdx: 0, maxIdx: 12, p0: [278, 598], p1: [98, 598] },
        { nid: 'oi:white_gull_harbor:lantern_ln', name: '石燈巷', lx: 496, ly: 746, startIdx: 0, maxIdx: 12, p0: [398, 725], p1: [568, 772] },
        { nid: 'oi:white_gull_harbor:seabreeze_ln', name: '海風巷', lx: 198, ly: 758, startIdx: 0, maxIdx: 12, p0: [308, 728], p1: [106, 782] },
        { nid: 'oi:white_gull_harbor:old_well_ln', name: '老井巷', lx: 564, ly: 840, startIdx: 0, maxIdx: 12, p0: [545, 795], p1: [582, 885] },
        { nid: 'oi:white_gull_harbor:south_wall_ln', name: '南牆根巷', lx: 162, ly: 944, startIdx: 0, maxIdx: 12, p0: [108, 855], p1: [212, 1022] }
      ]
    },

    // 2. 鷗尾漁港：南灣半月抱海避風石澳、北坡乾砌矮石牆、鯨骨肋拱與成排曬網架（60 戶漁鎮）
    gull_tail_port: {
      slug: 'gull_tail_port',
      title: '鷗尾漁港・南灣月牙石澳輿圖',
      imgUrl: '/maps/gull_tail_port.webp?v=' + VER,
      cropMode: 'full_bleed',
      bgDay: '#e5dac3',
      bgNight: '#131b21',
      glows: [
        [362, 260, 78], [558, 420, 92], [462, 655, 98], [458, 825, 82], [448, 138, 56]
      ],
      pills: [
        { nid: 'oi:gull_tail_port:north_gate', label: '北坡石拱門', x: 448, y: 136, kind: 'main' },
        { nid: 'oi:gull_tail_port:smithy_st', label: '北坡石街', x: 472, y: 288, kind: 'street' },
        { nid: 'oi:gull_tail_port:bal_forge', label: '修錨鍛鐵鋪', x: 356, y: 248, kind: 'shop' },
        { nid: 'oi:gull_tail_port:gale_carriage', label: '海產篷車場', x: 572, y: 214, kind: 'shop' },
        { nid: 'oi:gull_tail_port:herb_hall', label: '海風苦艾堂', x: 682, y: 292, kind: 'shop' },
        { nid: 'oi:gull_tail_port:tavern_st', label: '澳口坡街', x: 430, y: 472, kind: 'street' },
        { nid: 'oi:gull_tail_port:rusty_anchor_tavern', label: '木桶漁人酒館', atlasLabel: '木桶酒館(一樓)', x: 576, y: 420, kind: 'shop' },
        { nid: 'oi:gull_tail_port:inn_upstairs', label: '酒館二樓宿房', x: 576, y: 458, kind: 'alley', atlasOnly: true },
        { nid: 'oi:gull_tail_port:plaza', label: '南澳魚市廣場', x: 462, y: 656, kind: 'main' },
        { nid: 'oi:gull_tail_port:dock', label: '南灣雙棧漁埠', x: 458, y: 832, kind: 'main' },
        { nid: 'oi:gull_tail_port:south_gate', label: '西崖海道口', x: 138, y: 868, kind: 'main' }
      ],
      alleys: [
        { nid: 'oi:gull_tail_port:net_drying_ln', name: '曬網巷', lx: 752, ly: 458, startIdx: 1, maxIdx: 12, p0: [634, 488], p1: [740, 690] },
        { nid: 'oi:gull_tail_port:whalebone_ln', name: '鯨骨巷', lx: 228, ly: 332, startIdx: 1, maxIdx: 12, p0: [312, 286], p1: [126, 514] },
        { nid: 'oi:gull_tail_port:oyster_ln', name: '牡蠣殼巷', lx: 642, ly: 626, startIdx: 1, maxIdx: 12, p0: [524, 542], p1: [676, 696] },
        { nid: 'oi:gull_tail_port:cliff_steps_ln', name: '石階巷', lx: 118, ly: 692, startIdx: 1, maxIdx: 12, p0: [390, 508], p1: [216, 696] },
        { nid: 'oi:gull_tail_port:kelp_ln', name: '海藻巷', lx: 804, ly: 778, startIdx: 1, maxIdx: 12, p0: [748, 536], p1: [816, 732] }
      ]
    },

    // 3. 西峽渡口：西臨怒濤海峽、零城牆開放斜坡渡津、跨海雙桅渡輪長棧橋與木樁長屋（30 戶渡鎮）
    west_strait_ferry: {
      slug: 'west_strait_ferry',
      title: '西峽渡口・海峽長棧橋渡津輿圖',
      imgUrl: '/maps/west_strait_ferry.webp?v=' + VER,
      cropMode: 'full_bleed',
      bgDay: '#e4d8be',
      bgNight: '#121b22',
      glows: [
        [628, 368, 78], [492, 565, 96], [238, 540, 86], [662, 650, 88], [728, 502, 74]
      ],
      pills: [
        { nid: 'oi:west_strait_ferry:north_gate', label: '北岸哨柵口', x: 594, y: 84, kind: 'main' },
        { nid: 'oi:west_strait_ferry:smithy_st', label: '渡口北坡街', x: 518, y: 334, kind: 'street' },
        { nid: 'oi:west_strait_ferry:bal_forge', label: '船釘鍛鐵鋪', x: 640, y: 390, kind: 'shop' },
        { nid: 'oi:west_strait_ferry:herb_hall', label: '海峽苦艾寮', x: 778, y: 386, kind: 'shop' },
        { nid: 'oi:west_strait_ferry:gale_carriage', label: '海峽車馬場', x: 734, y: 504, kind: 'shop' },
        { nid: 'oi:west_strait_ferry:plaza', label: '渡口石板廣場', x: 492, y: 564, kind: 'main' },
        { nid: 'oi:west_strait_ferry:dock', label: '跨海渡輪長棧橋', x: 254, y: 568, kind: 'main' },
        { nid: 'oi:west_strait_ferry:tavern_st', label: '渡口南街', x: 542, y: 722, kind: 'street' },
        { nid: 'oi:west_strait_ferry:rusty_anchor_tavern', label: '避風木桶酒館', atlasLabel: '避風酒館(一樓)', x: 666, y: 664, kind: 'shop' },
        { nid: 'oi:west_strait_ferry:inn_upstairs', label: '酒館二樓宿房', x: 666, y: 702, kind: 'alley', atlasOnly: true },
        { nid: 'oi:west_strait_ferry:south_gate', label: '南岸界碑口', x: 586, y: 1118, kind: 'main' }
      ],
      alleys: [
        { nid: 'oi:west_strait_ferry:ferry_row', name: '渡頭木屋街', lx: 424, ly: 262, startIdx: 1, maxIdx: 10, p0: [486, 128], p1: [446, 444] },
        { nid: 'oi:west_strait_ferry:rope_shed_ln', name: '纜繩寮巷', lx: 468, ly: 868, startIdx: 1, maxIdx: 10, p0: [522, 666], p1: [478, 986] },
        { nid: 'oi:west_strait_ferry:star_watch_ln', name: '望海坡巷', lx: 774, ly: 836, startIdx: 1, maxIdx: 10, p0: [828, 760], p1: [698, 1068] }
      ]
    },

    // 4. 灰松林驛：純內陸千年灰松巨木林、十字古道篝火塘、山澗水車鋸木場與換馬大廄（27 戶林驛，零海零牆）
    greypine_post: {
      slug: 'greypine_post',
      title: '灰松林驛・島心巨木十字林驛輿圖',
      imgUrl: '/maps/greypine_post.webp?v=' + VER,
      cropMode: 'full_bleed',
      bgDay: '#dfd6be',
      bgNight: '#151a14',
      glows: [
        [450, 548, 118], [318, 398, 80], [318, 710, 88], [658, 695, 82], [180, 565, 62]
      ],
      pills: [
        { nid: 'oi:greypine_post:north_gate', label: '北林原木哨門', x: 450, y: 96, kind: 'main' },
        { nid: 'oi:greypine_post:smithy_st', label: '北林驛道', x: 448, y: 312, kind: 'street' },
        { nid: 'oi:greypine_post:bal_forge', label: '林間炭火鐵鋪', x: 332, y: 414, kind: 'shop' },
        { nid: 'oi:greypine_post:herb_hall', label: '精靈林藥堂', x: 614, y: 442, kind: 'shop' },
        { nid: 'oi:greypine_post:plaza', label: '十字火塘廣場', x: 450, y: 568, kind: 'main' },
        { nid: 'oi:greypine_post:gale_carriage', label: '林驛換馬大廄', x: 658, y: 696, kind: 'shop' },
        { nid: 'oi:greypine_post:tavern_st', label: '南林驛道', x: 452, y: 796, kind: 'street' },
        { nid: 'oi:greypine_post:rusty_anchor_tavern', label: '獵人巨木酒館', atlasLabel: '獵人酒館(一樓)', x: 318, y: 726, kind: 'shop' },
        { nid: 'oi:greypine_post:inn_upstairs', label: '酒館二樓宿房', x: 318, y: 764, kind: 'alley', atlasOnly: true },
        { nid: 'oi:greypine_post:south_gate', label: '南林原木哨門', x: 450, y: 1118, kind: 'main' }
      ],
      alleys: [
        { nid: 'oi:greypine_post:pine_resin_ln', name: '松脂巷', lx: 716, ly: 296, startIdx: 1, maxIdx: 9, p0: [596, 248], p1: [814, 454] },
        { nid: 'oi:greypine_post:sawmill_ln', name: '鋸木場巷', lx: 194, ly: 896, startIdx: 1, maxIdx: 9, p0: [162, 836], p1: [312, 1024] },
        { nid: 'oi:greypine_post:moss_well_ln', name: '苔井巷', lx: 706, ly: 984, startIdx: 1, maxIdx: 9, p0: [596, 916], p1: [794, 1084] }
      ]
    },

    // 5. 北岬燈塔：三面暗礁怒海黑岩狹長斷崖、崖頂巨型獸脂長明火石塔、東崖石階避風小塢（22 戶海岬哨站）
    north_cape_beacon: {
      slug: 'north_cape_beacon',
      title: '北岬燈塔・極北斷崖長明火塔輿圖',
      imgUrl: '/maps/north_cape_beacon.webp?v=' + VER,
      cropMode: 'full_bleed',
      bgDay: '#ddd4be',
      bgNight: '#10181f',
      glows: [
        [462, 232, 140], [352, 418, 76], [368, 580, 84], [468, 502, 82], [756, 690, 68]
      ],
      pills: [
        { nid: 'oi:north_cape_beacon:north_gate', label: '崖頂長明燈塔', x: 456, y: 274, kind: 'main' },
        { nid: 'oi:north_cape_beacon:smithy_st', label: '燈塔鞍部石坪', x: 462, y: 364, kind: 'street' },
        { nid: 'oi:north_cape_beacon:bal_forge', label: '崖頂防風鐵爐', x: 352, y: 432, kind: 'shop' },
        { nid: 'oi:north_cape_beacon:herb_hall', label: '熬脂苦艾石屋', x: 576, y: 416, kind: 'shop' },
        { nid: 'oi:north_cape_beacon:plaza', label: '塔下避風廣場', x: 448, y: 492, kind: 'main' },
        { nid: 'oi:north_cape_beacon:gale_carriage', label: '山脊騾車棚', x: 576, y: 536, kind: 'shop' },
        { nid: 'oi:north_cape_beacon:rusty_anchor_tavern', label: '抗風紅岩酒館', atlasLabel: '紅岩酒館(一樓)', x: 366, y: 594, kind: 'shop' },
        { nid: 'oi:north_cape_beacon:inn_upstairs', label: '酒館二樓宿房', x: 366, y: 632, kind: 'alley', atlasOnly: true },
        { nid: 'oi:north_cape_beacon:tavern_st', label: '海岬山脊石道', x: 466, y: 738, kind: 'street' },
        { nid: 'oi:north_cape_beacon:dock', label: '東澳避風石埠', x: 758, y: 682, kind: 'main' },
        { nid: 'oi:north_cape_beacon:south_gate', label: '南嶺雙柱哨口', x: 454, y: 1098, kind: 'main' }
      ],
      alleys: [
        { nid: 'oi:north_cape_beacon:beacon_foot_ln', name: '塔下石屋巷', lx: 586, ly: 662, startIdx: 1, maxIdx: 11, p0: [528, 578], p1: [572, 856] },
        { nid: 'oi:north_cape_beacon:driftwood_ln', name: '拾木灘巷', lx: 262, ly: 836, startIdx: 1, maxIdx: 11, p0: [258, 682], p1: [266, 1022] }
      ]
    }
  };
})();
