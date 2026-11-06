static RESIDENT_SCHEDULES_PART2: [ResidentSchedule; 29] = [
        // 31. 方赤土（紅土崁選礦與耐火磚窯方家分支）
        ResidentSchedule {
            id: "方赤土",
            gender: "M",
            display_title: "紅土崁選礦與耐火磚匠師|方赤土",
            resolve: |hour| match hour {
                7..=17 => (
                    "紅土崁選礦場",
                    "督導洗選三號坑口運來的赤紋鐵礦，並用篩下的高耐火紅黏土燒製能承受千度爐溫的紅土耐火磚",
                ),
                18..=20 => (
                    "紅土百工鐵器坊",
                    "檢視三號坑口礦道鋪設用的輕便鐵槽軌與坑口滑輪台車輪軸",
                ),
                _ => (
                    "紅土崁・方氏分支選礦與耐火磚老宅",
                    "與姪兒方石核對明日要發往文曜分家巷的重礦與耐火磚車次",
                ),
            },
        },
        // 32. 雷千鈞（南哨重型獵具工坊雷家）
        ResidentSchedule {
            id: "雷千鈞",
            gender: "M",
            display_title: "南哨重型獵具大匠|雷千鈞",
            resolve: |hour| match hour {
                7..=18 => (
                    "重型獵具工坊",
                    "用風舶角送來的巨藻韌索混編三號坑口紅鐵絲，打製困捕泥甲巨鱷的鐵絲巨藻獵網、飛爪索與紅磷引路信號筒",
                ),
                19..=21 => (
                    "高棧坪溫水浴場",
                    "泡在溫水池裡拔除南境濕氣，與羅烈討論新一批紅磷信號筒在黑沼濃霧中的照遠效果",
                ),
                _ => (
                    "南哨・雷家重型獵具大匠宅",
                    "在乾燥密封的木箱裡封存新配好的紅磷信號筒",
                ),
            },
        },
        // 33. 葉青囊（南哨脈絡解毒診所葉家）
        ResidentSchedule {
            id: "葉青囊",
            gender: "M",
            display_title: "南哨解毒診所主治醫師|葉青囊",
            resolve: |hour| match hour {
                7..=19 => (
                    "南哨脈絡解毒診所",
                    "用白茫洲採回的霧晶與浮洲奇花煉製避瘴清心丸，替自水尾寮外澤林歸來的採集隊員拔除濕氣與蟲毒",
                ),
                20..=21 => (
                    "南哨原野食堂",
                    "喝著驅濕的辛香熱肉湯，叮囑年輕獵人進林前務必帶齊避瘴丸與止血水苔敷藥",
                ),
                _ => (
                    "南哨・葉家白茫避瘴清心齋",
                    "在燈下給文曜城的師妹溫晴寫信，交換最新一批黑沼草藥的炮製心得",
                ),
            },
        },
        // 34. 阮水生（水尾寮高腳屋與平底竹筏阮家）
        ResidentSchedule {
            id: "阮水生",
            gender: "M",
            display_title: "水尾寮竹筏與高腳屋匠師|阮水生",
            resolve: |hour| match hour {
                6..=17 => (
                    "蘆葦與竹編工棚",
                    "用韌竹與浮木紮製在爛泥淺灘上也能貼水滑行的平底竹筏，並檢修水尾寮高腳木屋底下的沉水烏木基樁",
                ),
                18..=20 => (
                    "水尾寮平底竹筏埠",
                    "幫歸來的採集竹筏卸下新打撈出的沉水烏木、止血水苔與鮮活沼蝦，準備送往高棧坪收購行",
                ),
                _ => (
                    "水尾寮・阮家高腳木屋與平底竹筏匠老宅",
                    "坐在通風防潮的高腳木屋廊下修補捕魚竹簍，聽著屋下淺澤的蛙鳴入眠",
                ),
            },
        },
        // 35. wt_npc_lu_chengli (公議堂地政登記員陸承禮)
        ResidentSchedule {
            id: "wt_npc_lu_chengli",
            gender: "M",
            display_title: "公議堂地政登記員|陸承禮",
            resolve: |hour| match hour {
                6..=19 => (
                    "公議堂房地契書局",
                    "正在桌前核對《半島全戶總譜》與紅印屋契，加蓋鋼印發放黃銅門牌",
                ),
                20..=21 => (
                    "三水大食堂",
                    "在食堂角落與調解員陸長貴清點今日備案的屋契檔案，吃著熱湯麵",
                ),
                _ => (
                    "三水口・票號總管居",
                    "在檯燈下核對房地產權清冊並與家人閒聊",
                ),
            },
        },
        // 36. wt_npc_lu_changgui (公議堂民務通事陸長貴)
        ResidentSchedule {
            id: "wt_npc_lu_changgui",
            gender: "M",
            display_title: "公議堂民務通事|陸長貴",
            resolve: |hour| match hour {
                8..=11 => (
                    "三水匯流大石橋",
                    "站在三水匯流大石橋旁指引外地商旅認路，解答各項定居入籍與過戶手續",
                ),
                12..=17 => (
                    "公議堂",
                    "在公議堂前廳調解各街區鄰里糾紛，協調分家工坊排煙與給水維護",
                ),
                18..=20 => (
                    "東橋旅社",
                    "在茶屋大廳喝著鍋煮鮮奶茶，向外地旅客了解各村鎮近況",
                ),
                _ => (
                    "三水口・石板弄堂天井",
                    "在天井石桌旁核對明日待調解的民事紀錄、與鄰居打招呼",
                ),
            },
        },
        // 37. wt_npc_lu_jingxuan (蒙學先生陸敬軒)
        ResidentSchedule {
            id: "wt_npc_lu_jingxuan",
            gender: "M",
            display_title: "蒙學先生|陸敬軒",
            resolve: |hour| match hour {
                8..=16 => (
                    "學院主校區",
                    "帶領文曜蒙學堂的學生朗讀《白塔基礎讀本》，在練習板上指導書寫",
                ),
                17..=19 => (
                    "藏書閣",
                    "在藏書閣中挑選適合學童閱讀的四季自然插畫圖鑑",
                ),
                _ => (
                    "講堂街・蒙學陸先生舍",
                    "在貝母檯燈下批改學生作業並指導助教整理教材",
                ),
            },
        },
        // 38. wt_npc_qin_mingcheng (百工核算師秦明誠)
        ResidentSchedule {
            id: "wt_npc_qin_mingcheng",
            gender: "M",
            display_title: "百工核算師|秦明誠",
            resolve: |hour| match hour {
                8..=17 => (
                    "百工商算",
                    "左手壓著出貨帳冊、右手操作隨身計數板，核算各工坊出貨成本與損耗",
                ),
                18..=20 => (
                    "三水大食堂",
                    "邊吃著蔥香米烙餅邊在記事本上核算下一季的採購鎂幣預算",
                ),
                _ => (
                    "講堂街・秦氏算學居",
                    "在書桌前檢視實習核算員提交的帳務試算表",
                ),
            },
        },
        // 39. wt_npc_lin_shizhong (傳訊鐘樓司鐘長林時中)
        ResidentSchedule {
            id: "wt_npc_lin_shizhong",
            gender: "M",
            display_title: "傳訊鐘樓司鐘長|林時中",
            resolve: |hour| match hour {
                6..=19 => (
                    "傳訊鐘樓與信使總站",
                    "在鐘樓瞭望台校對旋轉日規與報時青銅大鐘，並為整裝待發的快馬信使核驗防雨郵袋與各鎮收發印戳",
                ),
                20..=21 => (
                    "三水大食堂",
                    "在食堂喝著熱騰騰的原牛骨湯，與收工的快馬信使核對明日發往風舶角與老堰頭的早班信件",
                ),
                _ => (
                    "三水口・信使快騎宿所",
                    "在透鏡聚焦檯燈下保養隨身黃銅懷錶的發條齒輪",
                ),
            },
        },
        // 40. wt_npc_zhao_guanxing (學院觀星台司天掌度趙衡宇)
        ResidentSchedule {
            id: "wt_npc_zhao_guanxing",
            gender: "M",
            display_title: "學院觀星台司天掌度|趙衡宇",
            resolve: |hour| match hour {
                8..=21 => (
                    "學院觀星台與評議廳",
                    "站在青石度盤旁調整三百六十度星軌銅儀，記錄節氣日影與北山氣流變化，供學院評議廳編修年度曆譜",
                ),
                _ => (
                    "講堂街・石硯墨香居",
                    "在書桌前整理白天測得的星度交角與潮汐推算表",
                ),
            },
        },
        // 41. wt_npc_zhou_yidao (講堂街理髮老師傅周敬修)
        ResidentSchedule {
            id: "wt_npc_zhou_yidao",
            gender: "M",
            display_title: "講堂街理髮老師傅|周敬修",
            resolve: |hour| match hour {
                7..=19 => (
                    "講堂街老字號剪剃鋪",
                    "在磨刀布上俐落推磨精鋼摺疊剃刀，擰乾紫銅溫水管裡冒著熱氣的蜂格棉毛巾，笑著替街坊學生修剪頭髮與修面",
                ),
                20..=21 => (
                    "東橋旅社與山風茶屋",
                    "在茶屋喝著烘青茶，與街坊鄰居聊著三水口與講堂街的日常趣聞",
                ),
                _ => (
                    "講堂街・抄經校勘齋",
                    "用山茶防鏽油細心保養使用多年的精鋼剃刀與理髮剪刀",
                ),
            },
        },
        // 42. wt_npc_cen_yunye (分家巷武術館教練岑雲野)
        ResidentSchedule {
            id: "wt_npc_cen_yunye",
            gender: "M",
            display_title: "分家巷武術館教練|岑雲野",
            resolve: |hour| match hour {
                6..=17 => (
                    "習武堂",
                    "指導年輕學員站穩重心，手持包革練習木刀拆解防身卸力技巧",
                ),
                18..=20 => (
                    "泉香溫水浴場",
                    "泡在溫水池中放鬆筋骨，與韓鐵山聊著近來的訓練心得",
                ),
                _ => (
                    "分家巷・岑氏武勇舍",
                    "在武術館庭院與住宿學員進行對練與呼吸調整",
                ),
            },
        },
        // 43. wt_npc_xu_agan (分家巷柑仔店阿婆許阿甘)
        ResidentSchedule {
            id: "wt_npc_xu_agan",
            gender: "F",
            display_title: "分家巷柑仔店阿婆|許阿甘",
            resolve: |hour| match hour {
                6..=20 => (
                    "分家巷口雜貨柑仔店",
                    "坐在竹編藤椅上擦拭裝滿黑糖麥芽糖與鹽津梅餅的廣口玻璃罐，招呼下工的鐵匠學徒添購紅磷火籤與艾草蚊香盤",
                ),
                _ => (
                    "分家巷・精工火磚匠廬",
                    "在暖屋裡清點今日收進的零碎鎂幣，把新進的樟木防蟲丸分裝入袋",
                ),
            },
        },
        // 44. wt_npc_ma_buxue (西門修傘補靴老匠馬守成)
        ResidentSchedule {
            id: "wt_npc_ma_buxue",
            gender: "M",
            display_title: "西門修傘補靴老匠|馬守成",
            resolve: |hour| match hour {
                6..=18 => (
                    "西門車馬前街",
                    "在街邊支起老匠晴雨傘與皮靴修補攤，將磨損皮靴套上硬木楦頭釘補厚牛皮掌，並替韌竹骨晴雨傘刷上防水熟桐油",
                ),
                19..=20 => (
                    "三水大食堂",
                    "已收攤將修鞋工具箱挑到桌邊，端著一碗熱切仔湯麵配滷蛋，與車夫閒聊路況",
                ),
                _ => (
                    "西門・皮靴油布匠居",
                    "在煤油燈下揉製塗抹皮靴接縫用的蜂蠟松脂防水膏",
                ),
            },
        },
        // 45. wt_vendor_roast_beef (炭烤牛肉番薯攤東家鄭火旺)
        ResidentSchedule {
            id: "wt_vendor_roast_beef",
            gender: "M",
            display_title: "炭烤牛肉番薯攤東家|鄭火旺",
            resolve: |hour| match hour {
                15..=23 | 0..=4 => (
                    "三水口南街",
                    "正在炭烤牛肉番薯攤前翻動滋滋冒油的原牛肉串與流蜜紅心番薯，向往來行客高聲叫賣",
                ),
                14 => (
                    "三水口・石板弄堂天井",
                    "正在院裡將穿好的原牛肉串與洗淨紅心番薯裝上木輪攤車，準備推往三水口南街擺攤",
                ),
                _ => (
                    "三水口・石板弄堂天井",
                    "已收攤推車返家，擦淨鐵網與炭爐後在屋內安歇",
                ),
            },
        },
        // 46. wt_vendor_noodle_soup (現煮切仔麵攤女東家林罔市)
        ResidentSchedule {
            id: "wt_vendor_noodle_soup",
            gender: "F",
            display_title: "現煮切仔麵攤女東家|林罔市",
            resolve: |hour| match hour {
                15..=23 | 0..=4 => (
                    "三水口南街",
                    "正在現煮大骨切仔麵攤前笊籬起落、淋上濃白大骨湯與油蔥酥，熱情招呼夜歸人吃麵",
                ),
                14 => (
                    "三水口・陳氏庖廚居",
                    "正在灶前將熬足三個鐘頭的大骨高湯裝入保溫銅桶，準備推車前往三水口南街擺攤",
                ),
                _ => (
                    "三水口・陳氏庖廚居",
                    "已收攤推車返家，刷洗大湯鍋與碗筷後在屋內安歇",
                ),
            },
        },
        // 47. wt_vendor_pepper_bun (貼爐胡椒餅攤師傅張阿發)
        ResidentSchedule {
            id: "wt_vendor_pepper_bun",
            gender: "M",
            display_title: "貼爐胡椒餅攤師傅|張阿發",
            resolve: |hour| match hour {
                15..=23 | 0..=4 => (
                    "三水口南街",
                    "正在貼爐蔥香胡椒餅攤前用鐵夾從炭火鐵桶壁鏟下金黃酥脆的胡椒餅，大聲吆喝現出爐的熱餅",
                ),
                14 => (
                    "西門・草料客商居",
                    "正在揉製老麵、切妥滿滿的三星青蔥與黑胡椒豬肉餡，準備推車出門擺攤",
                ),
                _ => (
                    "西門・草料客商居",
                    "已收攤推車返家，封好烤桶餘燼後在屋內安歇",
                ),
            },
        },
        // 48. wt_vendor_herbal_tea (寒水玉冰鎮仙草凍攤主蘇秋月)
        ResidentSchedule {
            id: "wt_vendor_herbal_tea",
            gender: "F",
            display_title: "寒水玉冰鎮仙草凍攤主|蘇秋月",
            resolve: |hour| match hour {
                15..=23 | 0..=4 => (
                    "三水口南街",
                    "正在寒水玉冰鎮仙草凍攤前手搖銅盞叮噹作響，盛裝慢熬仙草凍與清涼百草飲招呼客人解膩",
                ),
                14 => (
                    "分家巷・蘇氏繡坊居",
                    "正在將文火慢熬的仙草乾汁凝凍裝入冰鎮陶甕，準備推車出門擺攤",
                ),
                _ => (
                    "分家巷・蘇氏繡坊居",
                    "已收攤推車返家，晾乾銅盞與陶甕後在屋內安歇",
                ),
            },
        },
        // 49. wt_vendor_tofu_pudding (黑糖粉圓豆花攤伯陳順興)
        ResidentSchedule {
            id: "wt_vendor_tofu_pudding",
            gender: "M",
            display_title: "黑糖粉圓豆花攤伯|陳順興",
            resolve: |hour| match hour {
                15..=23 | 0..=4 => (
                    "三水口南街",
                    "正在古早味黑糖粉圓豆花攤前用扁平銅杓片出綿密熱豆花、舀入Q彈黑糖粉圓與薑汁叫賣",
                ),
                14 => (
                    "三水口・晨汲青石宅",
                    "正在用古井泉水沖點鹽滷熱豆花、熬煮手炒黑糖粉圓，準備推車出門擺攤",
                ),
                _ => (
                    "三水口・晨汲青石宅",
                    "已收攤推車返家，浸泡明日用的飽滿黃豆後在屋內安歇",
                ),
            },
        },
        // 50. wt_vendor_rice_roll (紫米飯糰早點攤姨吳秀琴)
        ResidentSchedule {
            id: "wt_vendor_rice_roll",
            gender: "F",
            display_title: "紫米飯糰早點攤姨|吳秀琴",
            resolve: |hour| match hour {
                5..=14 => (
                    "三水口北街",
                    "正在現包紫米飯糰早點攤前掀開大木桶蒸氣，俐落盛裝荷葉糯米炊飯與老油條飯糰，招呼居民買早點",
                ),
                15..=17 => (
                    "三水口乾貨海味行",
                    "已收攤歇息，正在挑選明日炊飯用的乾荷葉、香脆菜脯與肉鬆",
                ),
                _ => (
                    "三水口・河畔迎客小院",
                    "已收攤返家，淘洗浸泡糯米與紫米後在屋內安歇",
                ),
            },
        },
        // 51. wt_vendor_soy_milk (現磨米漿杏仁茶攤叔周文貴)
        ResidentSchedule {
            id: "wt_vendor_soy_milk",
            gender: "M",
            display_title: "現磨米漿杏仁茶攤叔|周文貴",
            resolve: |hour| match hour {
                5..=14 => (
                    "三水口北街",
                    "正在現磨米漿杏仁茶攤前攪拌濃香滾燙的現磨溫米漿與杏仁茶，配著酥脆現炸油條高聲叫賣",
                ),
                15..=17 => (
                    "分家巷口雜貨柑仔店",
                    "已收攤歇息，正在添購烘烤過的新鮮北港花生與南杏仁",
                ),
                _ => (
                    "三水口・河畔迎客小院",
                    "已收攤返家，擦拭石磨與銅壺後在屋內安歇",
                ),
            },
        },
        // 52. wt_vendor_morning_produce (時令脆甜蔬果攤農李春生)
        ResidentSchedule {
            id: "wt_vendor_morning_produce",
            gender: "M",
            display_title: "時令脆甜蔬果攤農|李春生",
            resolve: |hour| match hour {
                5..=14 => (
                    "三水口北街",
                    "正在清晨帶露時令蔬果攤前噴水整理新摘下的老堰頭水芹菜脆嫩莖與脆甜高麗菜，熱情招呼買菜居民",
                ),
                15..=17 => (
                    "三水大食堂",
                    "已收攤賣完今日鮮菜，正坐在食堂喝碗熱湯歇腳",
                ),
                _ => (
                    "分家巷・柯氏木作居",
                    "已收攤返家，捆紮明日要用的竹編菜簍後在屋內安歇",
                ),
            },
        },
        // 53. wt_vendor_morning_fish (現撈河鮮蛤蜊攤販許海生)
        ResidentSchedule {
            id: "wt_vendor_morning_fish",
            gender: "M",
            display_title: "現撈河鮮蛤蜊攤販|許海生",
            resolve: |hour| match hour {
                5..=14 => (
                    "三水口北街",
                    "正在現撈三水河鮮蛤蜊攤前給淺木盆換上清水，讓水尾寮淺澤青殼河蝦與大文蛤保鮮，大聲吆喝早市客人挑選",
                ),
                15..=17 => (
                    "三水口乾貨海味行",
                    "已收攤歇息，將剩餘鮮魚河蝦交給海味行鹽漬烘乾",
                ),
                _ => (
                    "三水口・老水夫舍",
                    "已收攤返家，修補魚網與木盆後在屋內安歇",
                ),
            },
        },
        // 54. 馬行頭 (wt_mahanger / 車馬行大行頭馬振遠)
        ResidentSchedule {
            id: "馬行頭",
            gender: "M",
            display_title: "西麓車馬大行頭|馬振遠",
            resolve: |hour| match hour {
                6..=16 => (
                    "西門車馬聯運總站",
                    "在總站前調配出城前往西麓林道的大車，檢查韁繩、蹄鐵與路面車轍",
                ),
                17..=20 => (
                    "東橋旅社",
                    "在旅社大堂與各路車伕核對西麓林道與沿線物料運送回執",
                ),
                _ => (
                    "西門・韓氏總站宅院",
                    "在後院馬廄照料健壯草獸，整理明晨西出林道車隊裝備",
                ),
            },
        },
        // 55. 林山 (wt_linshan / 巡林獵戶)
        ResidentSchedule {
            id: "林山",
            gender: "M",
            display_title: "翠冠巨木巡林獵戶|林山",
            resolve: |hour| match hour {
                6..=16 => (
                    "水輪石磨工坊",
                    "在老堰頭石磨工坊周邊巡查巨木林獵徑與林木生長，排查野獸足跡與落葉堆",
                ),
                17..=20 => (
                    "堰渠水畔食堂",
                    "在水畔食堂大堂喝著熱茶，與過路獵人交流巨木林深處獸蹤情報",
                ),
                _ => (
                    "北嶺腳・馬家鐵齒登山釘鞋工坊宅",
                    "在屋內修整獵弓、皮靴與引路標記，記錄巡林心得",
                ),
            },
        },
        // 56. 雷橫山 (wt_leihengshan / 水澤機關匠人)
        ResidentSchedule {
            id: "雷橫山",
            gender: "M",
            display_title: "南哨水澤機關匠人|雷橫山",
            resolve: |hour| match hour {
                6..=17 => (
                    "重型獵具工坊",
                    "在工坊調校防腐捕獸夾與耐潮鐵樁，準備運往淺澤泥口與水澤邊緣防務",
                ),
                18..=20 => (
                    "南哨原野食堂",
                    "與巡邏守衛同桌用膳，盤算各處泥口獵具機關磨損情況",
                ),
                _ => (
                    "南哨・雷家重型獵具大匠宅",
                    "在油燈下保養防腐工具與滑輪索具，備足替換部件",
                ),
            },
        },
        // 57. 葉青 (wt_yeqing / 白茫白蘚採藥師)
        ResidentSchedule {
            id: "葉青",
            gender: "F",
            display_title: "百茫白蘚採藥師|葉青",
            resolve: |hour| match hour {
                6..=17 => (
                    "南哨脈絡解毒診所",
                    "在診所整理烘乾的白茫洲白蘚與藥草，調配清熱避瘴草藥包",
                ),
                18..=20 => (
                    "南哨原野食堂",
                    "在大堂分發避瘴藥茶，叮囑夜巡守衛防範水澤濕氣",
                ),
                _ => (
                    "南哨・葉家白茫避瘴清心齋",
                    "在藥室翻檢陰乾的鮮葉與藥膏，記錄草藥配伍比例",
                ),
            },
        },
        // 58. 田大犁 (wt_tiandali / 草甸改良農具匠師田長耕)
        ResidentSchedule {
            id: "田大犁",
            gender: "M",
            display_title: "草甸改良農具匠師|田長耕",
            resolve: |hour| match hour {
                6..=17 => (
                    "大圳頭・大圳農具與百工坊",
                    "在工坊鍛造改良型草甸翻土犁刀，測試鋼口韌度與犁架配重",
                ),
                18..=20 => (
                    "田園風味大食堂",
                    "與莊戶農夫切磋開墾草甸硬土的農具心得，記錄改進建議",
                ),
                _ => (
                    "大圳頭・田氏農具匠人老宅",
                    "在油燈下打磨農具樣模與齒輪配件，準備明日測試",
                ),
            },
        },
        // 59. 牧野 (wt_muye / 長風草甸巡牧官)
        ResidentSchedule {
            id: "牧野",
            gender: "M",
            display_title: "長風草甸巡牧官|牧野",
            resolve: |hour| match hour {
                6..=17 => (
                    "羊毛與乳酪精製坊",
                    "在精製坊協調長風草甸牛群放牧路線與草料調配，清點毛氈原料",
                ),
                18..=20 => (
                    "牧野歇腳旅社",
                    "在旅社大堂歇息喝熱奶茶，向往來商隊打聽草甸邊界水源狀況",
                ),
                _ => (
                    "牛埔・牧家牛埔硬乳酪發酵老宅",
                    "檢查草場圍欄與飲水坑備忘錄，確認草甸畜群安然入圈",
                ),
            },
        },
];

fn iter_resident_schedules() -> impl Iterator<Item = &'static ResidentSchedule> {
    RESIDENT_SCHEDULES_PART1.iter().chain(RESIDENT_SCHEDULES_PART2.iter())
}

pub fn resident_schedule_display_title(npc_id: &str) -> Option<&'static str> {
    iter_resident_schedules()
        .find(|s| s.id == npc_id)
        .map(|s| s.display_title)
}

pub fn resident_schedule_home_and_work(npc_id: &str) -> Option<(&'static str, &'static str)> {
    let trimmed = npc_id.trim();
    if trimmed.is_empty() {
        return None;
    }
    iter_resident_schedules()
        .find(|s| {
            s.id == trimmed
                || s.display_title
                    .split(['|', '｜'])
                    .nth(1)
                    .map(|n| n.trim() == trimmed)
                    .unwrap_or(false)
        })
        .map(|s| {
            let (home_room, _) = (s.resolve)(23);
            let (work_room, _) = (s.resolve)(10);
            (home_room, work_room)
        })
}
