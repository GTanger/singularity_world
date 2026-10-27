# Singularity World — 當前交班狀態 (SESSION.md)

- **更新日期**：2026-10-27
- **當前狀態**：`completed`（台灣山海經活世界四重奏 ＋ 500 詞 × 361 星盤 × 一字雙境 ＋ 五源五層管線 ＋ 萬人萬相 `soul_seed` 星盤推演 × 器物無分工具武器與外盤銘文 `[字]` × 碎招→套路→技能與一力十會博弈 × 真人 NPC 自主刻字穿戴已全數實裝、測試通過並部署至 `singularity.service`）

## 一、 本輪完成里程碑（2026-10-27）

1. **「台灣山海經」十九獵場百獸生態與活世界四重奏實裝**：
   - **十九獵場 103 種台灣山海生靈與四階戰力梯度**：實裝於 `src/game/room.rs:442-780`（`ensure_ambient_npc_at_grid`、`ambient_creature_loot`），擊敗異獸掉落對應《台灣山海經》部位（`src/server/action/combat.rs:115-145`）。
   - **①【食之／佩之／炙之】部位奇效與戰鬥護佑**：`src/game/room.rs:815-990`（`shanhai_item_use_effect`、`player_shanhai_talisman_bonus`），串接 `src/server/handler/inventory.rs:631-662`、`src/server/action/object.rs:1114-1127` 與 `src/server/action/combat.rs:125-145`。隨身靈佩（帝雉長尾翎、藍腹鷴彩羽、雲豹軟毛皮、石虎斑皮、穿山甲鱗片、巨鱷厚麟皮、山豬王獠牙、白鼻心香皮、熊鷹翎羽）使用不耗損且交鋒自動觸發【山海佩飾護佑】；本草珍饈（百步蛇乾蛻、月熊掌、月熊腹脂膏、深山牛樟芝）服食回補體魄與元氣。
   - **②【晝伏夜出、潮退現身】出沒時辰與潮汐律動**：`src/game/room.rs:625-668`，結合 `current_game_hour()` 呈現晨霧瑞獸（05:00–09:00）、夜行山靈（18:00–06:00）與大潮海怪（潮汐退湧時段）動態。
   - **③【未見其獸，先見其跡】十九獵場霸主環境異兆**：`src/game/room.rs:782-812`（`hunting_zone_apex_omen`）與 `src/server/action/object.rs:995-1007`，查看野外地景（獸徑足印、避風岩龕、山澗清泉眼、巡山木樁、野地營火堆）自動浮現該獵場鎮場霸主之【山海異兆】。
   - **④【山海合味】營火與食堂來料代客料理**：`src/game/object_ecology.rs:2505-2615`（`try_cook_shanhai_delicacy`）與 `src/server/action/object.rs:2200-2210`，攜帶山海獵獲搭配台灣道地調味物產（馬告粒、陳年老菜脯、黑豆壺底蔭油、甘醇蔭油膏、老薑黑糖磚）可現煮八道山海名膳。
2. **「500 詞 × 361 星盤 × 一字雙境（在戰陣為武相、在日常為藝相）」收斂實裝**：
   - **500 詞不擴編 × 一字雙境**：`data/config/word_elements.json` 洗淨全部第四面牆系統黑話；`data/config/word_elements_gamedims.json` 校準 500 詞語義對比（修正強/弱、速/緩、堅/脆數值）並新增 4 維百工行旅向量 `craft_dims`（`forge` 鍛鑄百工、`culinary` 庖廚丹膳、`harvest` 採獵剝名、`wayfare` 山海行旅），喚醒現有 500 詞中之 `鑄、織、剝、種、磨、捏、味、色、音、爐、鼎、鮮、陳、巧、純` 等百工字。
   - **星盤起承轉合語序權重（破除加法交換律）**：`src/combat/wordplate.rs:585-690`（`StarSentence::synthesize`）依句首【起】、句中【承轉】、句尾【合】賦予位置權重（`[A, B, C] != [C, B, A]`），並同步結算 `CraftDims4D` 與 `CraftAspect`（`dual_context_summary`）。
   - **361 星盤與五層管線正典對齊**：完整保留 361 四區分層（1 生之奇點 + 20 主樞 + 100 五常邏輯閘 + 240 十二微型狀態）、760 邊總阻力 10,000 守恆、星盤迷霧「名存實未知」與 `[內視]`、微循環 vs 大循環、G07/G08 斷點橫向繞路；於五層管線（採集→剝名→結晶→提出→裸詞元）增設個人不可交易之【未定字譜（識海試排槽）】。

3. **「五源採獵 → 剝名凝晶 → 碎晶提出 → 500 詞入未定字譜（含同名疊煉、鎔鑄化鎂與持久化）」實機接通**：
   - **五大天地來源（獸 193／植 153／礦 205／水 161／氣 212）實機抽樣**：`src/combat/wordplate.rs` 新增 `WordSource`、`WordQuality`（丙品／乙品／甲品）、`WordRegistry::words_by_source` 與 `roll_word_from_source`、`roll_strip_quality`（結合百工 `harvest` 採獵剝名造詣與材料品階）、`compute_extraction_ratio`（結合百工 `forge` 鍛鑄造詣與官署工坊保底 7 成，產出 3～9 成提取成數）。
   - **15 種可交易「詞元結晶（舍利子態）」與萬物【剝名凝晶】**：`src/game/object_ecology.rs`（`lookup_ecology_item_def`、`classify_material_word_source`）註冊 `crystal_{beast|plant|mineral|water|qi}_{bing|yi|jia}` 共 15 種結晶並支援市集動態收購；對山海獸部位、草木、礦石、水產執行 `StripName` 即可剝名凝晶，於星軌銅儀、山澗清泉、避風岩龕、河岸竹筏與汲水埠階互動亦可【觀境凝晶】取得氣／水源結晶（`src/server/action/object.rs`）。
   - **【碎晶提出】、【同名疊煉（含玄類入竅改骰）】、【鎔鑄化鎂】與磁碟持久化**：`src/server/handler/inventory.rs` 與 `web/sw-panes.js` 完整串接 `ExtractWord`、`StackWord`（消耗同名詞元充能 0%→100% 衝擊丙昇乙、乙昇甲，並計入 `TacticalLoadout::xuan_luck_bonus` 玄類詞元改骰加成）、`MintWord`（化為 12/28/60 鎂錠），並將玩家未定字譜與四序位戰術落盤持久化於 `data/runtime/player_wordplates.json`。
4. **「萬人萬相（`soul_seed` × 361 星盤）× 器物無分工具與武器（外盤銘文 `[字]`）× 碎招→套路→技能 × 真人 NPC」實機落地**：
   - **萬人萬相星盤推演**：`src/combat/wordplate.rs`（`build_loadout_from_soul_seed`）與 `src/server/action/combat.rs` 廢除 3 套罐頭 NPC 模板，依 64-bit `soul_seed` 與 760 條星軌阻力決定性衍生每位角色/NPC 專屬四序位套路。
   - **碎招 $\to$ 套路 $\to$ 技能 ＆ 一力與十會博弈**：`resolve_bout` 落實「成功叫技能，失敗、被格擋就只是碎招」，並實裝「同量級專克拆招（玉女克全真）」、「跨量級正面壓制（一力降十會）」與「入微避實擊虛（十會解一力／庖丁解牛）」三態動態裁決。
   - **器物無分工具與武器 ＆ 外盤銘文 `[字]` ＆ 真人 NPC**：`WordCasket::inscribe_item` 支援將詞元烙印於持物／護具（如 `圓弧雙柄削皮彎刀 [薄][巧][解]`），同時增幅日常【剝名凝晶（庖丁解牛共鳴）】與戰陣攻防；`classify_catalog_wearable_slot` 開放千物總典個人刀具、鐵鎬、竹傘、衣帽靴腕穿戴，NPC 於背景自主提字、刻字並穿戴上身（`npc_autonomous_word_and_equip_step`），`Look` 即時顯化其手持器物與 `[字]` 銘文。
   - **人物背板「技能」欄位自然生成與自動施展**：`src/server/protocol.rs`（`NaturalSkillView`）與 `src/server/handler/inventory.rs`（`synthesize_entity_natural_skills`）依人物生命種子經脈、三維屬性、所學字印與熟習器物自然生成 4 項【臨陣武相】與 4 項【百工藝相】技能，移除 `web/sw-panes.js` 自訂招式名稱輸入框，於背板【技能】欄完整呈現並於戰鬥、剝名、百工煉製與器物異變中由後台自動施展。
   - **後台結構化數據 $\to$ 本機 LLM 數據轉描述（Data-to-Text）與嚴格事實保真度核對（Fact-Fidelity Guardrail）**：`src/combat/wordplate.rs`（`derive_elementary_motions_from_chars`、`a_completed_motions` 碎招序列與交鋒斷點）與 `src/ai/narrative.rs`（`synthesize_skill_from_data`、`compose_combat_bout_from_data`、`verify_skill_output`、`verify_combat_bout_output`）串接本機 `Gemma-4-26B`（`127.0.0.1:11434/v1`）生成技能名稱、身法演繹（快取於 `data/runtime/llm_skill_cache.json`）與現場交鋒敘事 Log。嚴格隔離未成招之敗方技能名不入 Prompt，並由驗證器強制核對雙方稱呼、成招／碎招狀態、器物白名單、真實受創部位與零數字／零系統詞，確保 LLM 100% 依提供的資料編撰。
5. **過往多模型（Claude Opus / Gemini / Gemma3）歷史討論稽核與四大進階系統完整實裝**：
   - **① 部位傷勢、架勢失衡（`PostureState`）、瘴毒（`Miasma`）與診所／藥圃／旅社療傷閉環**：`src/pg/living_world.rs`（`entity_body_injuries`、`ground_dropped_items`）、`src/server/action/combat.rs`、`src/server/handler/movement.rs` 與 `src/server/action/object.rs`。實裝四部位傷勢（頭面震盪、胸脅內傷、腕肘挫傷、下盤扭傷）與黑沼林「瘴毒入體」（僅藥圃本草可拔除，一般病榻僅治外傷）；常規交鋒落敗或架勢潰散觸發脈絡診所 `+` 救援並將遺落物資寫入地面暫存池（重啟不遺失，且保護手札、殘軀、結晶、屋契、鑰匙絕不掉落）。
   - **② 著譜立說《功法手札》與研讀傳承「同法不同相」**：`src/server/handler/inventory.rs`（`TranscribeManual`、`StudyManual`）與 `web/sw-panes.js`。消耗「空白手札」將三／五／七言套路寫入 PostgreSQL `transcribed_manuals` 生成實體《手札》；研讀時嚴格檢驗詞匣是否持有對應字印（絕不憑空印字，缺字列為參悟殘譜），字印齊全則依研讀者自身 `soul_seed` 361 星盤通暢度推衍「同法不同相」碎招動作（`derive_elementary_motions_with_fluency`）。
   - **③ 狩獵現場「山海獸殘軀：整軀剝名 vs 肢體分解」雙插座**：擊敗山海異獸掉落帶新鮮度時效之 `corpse_*`（`beast_corpse_instances`），提供 `StripNameWhole`（含失手靈韻散逸與成功保底乙／甲品結晶）與 `ButcherCorpse`（拆為部位與獸骨，且部位不可再分解）雙動作；NPC 獵戶以純公式自主處置。
   - **④ 戰場語境殘留（`BattlefieldResidue`）、三節並行 LLM 轉譯與第五戰術「搏命硬撼」**：實裝二階以上招式留場語境（熾烈燥氣、凝寒霜氣、沉雄暗震、疾風銳勢、牽引旋流）之順應共鳴與煞氣逆沖、`compose_combat_bouts_concurrent` 並行 LLM 轉譯，以及 `ACTIVE_ENGAGEMENTS` 綁定之五項戰術抉擇（含唯一主動承擔反噬並重創對手之「搏命硬撼」）。
6. **脫離舊「數理組做題家／配電盤／自動販賣機」之五大有機活世界系統實裝**：
   - **① 氣機遇阻自動繞路與「動中體察」**：`src/combat/wordplate.rs`（`compute_auto_detour_meridian_note`、`somatic_meridian_awareness`）依 `soul_seed` 760 邊阻力自動避開險阻與縱向斷點完成橫向支絡繞行，寫入技能由來與手札研讀；於靜室、茶席、蒲團、病榻安歇調息時自然回饋周天暢流體察（廢除點 360 次開盲盒）。
   - **② 自然養成具慣性之【內息底色】**：`src/pg/living_world.rs`（`entity_breath_habits`：`鍛爐沉息`、`山林長息`、`海潮綿息`、`靜室清息`、`百戰烈息`）隨人物常做活計、交鋒與靜修自然累積並具轉移慣性，置頂呈現於背板技能欄（`web/sw-panes.js` 隱藏著譜按鈕並展示行氣節奏，廢除手動插拔心法槽）。
   - **③ 器物「人器磨合 × 鋒刃器況」雙軸與磨刀／回火閉環**：`src/pg/living_world.rs`（`entity_vessel_states`：`初握生澀 → 漸趨順手 → 沉穩趁手 → 如臂使指` × `鋒芒畢露 / 刃口微鈍 / 捲刃待修`）。拔除 `object.rs` 街邊磨刀石加人物體魄元氣之舊碼，改為街邊磨刀石／活水磨刀槽修復「刃口微鈍」、鍛爐鐵砧回火修復「捲刃待修」。
   - **④ 日常百工高光分層 LLM 數據轉描述**：`src/ai/narrative.rs`（`CraftActionFactPacket`、`compose_craft_highlight_from_data`、`verify_craft_output`），尋常工序俐落秒回，整軀剝名高階晶、趁手刀具庖丁解牛與鍛造器物異變交由本機 `Gemma-4-26B` 編撰並經零數字／零禁詞護欄核對。
   - **⑤ 公議堂具名急徵與【師承烙印】傳承**：`src/server/action/object.rs` 與 `src/game/object_ecology.rs` 移除 1.15 倍萬物自動販賣機與 `庫存 x/y` 試算表，改為分家巷溫掌事具名急徵；百工協力與請益累積 `entity_lineage_imprints`（岑鐵匠、溫大娘、柯木匠、溫掌事），達標時親贈可立即研讀之真字《傳世手札》。
7. **「`NPC = 玩家` 自主磨刀修繕 × `Look` 內息器況外顯 × `talk.rs` 三棒師承交涉接力」完整閉環**：
   - **NPC 自主磨刀與回火修繕**：`src/server/handler/inventory.rs`（`npc_autonomous_word_and_equip_step`）使全島 NPC 隨日常鍛造、庖廚、巡防、採獵自然累積【內息底色】與所持器物之【人器磨合 × 鋒刃器況】，並在刀刃微鈍或捲刃時自主於磨刀槽推磨或回火修整。
   - **`Look` 觀察見底細**：`src/server/action/look.rs`（`build_look_narrative`）打量人物或 NPC 時，直觀顯化其所持器物之磨合度與器況（如 `（漸趨順手・鋒芒畢露）`）、呼吸吐納神態（`內息底色：【鍛爐沉息】`）與肉身部位傷勢（如 `似帶有腕肘挫傷之態`）。
   - **`talk.rs` 三棒師承請益與 PG 關係交涉接力**：`src/server/action/talk.rs`（`effective_npc_disposition_and_trust`、`handle_three_baton_mentorship_and_obligation`）全面對接 PostgreSQL `resident_relationships`，向名匠／獵戶／武師／掌事請益手藝時即時推進【師承烙印】並於熟稔達標時親贈可研讀之真字《傳世手札》。81 項單元與端到端閉環測試全數通過。
8. **「星盤【百工藝相】四維（`forge / culinary / harvest / wayfare`）× 借物器況檢驗 × 師承熟客折讓 × 山海營火 LLM 高光」全線貫通**：
   - **百工藝相四維實質驅動日常活計（`src/game/object_ecology.rs` ＆ `src/server/handler/inventory.rs`）**：公開 `entity_total_craft_dims(pid)`（匯總星盤套路與手持銘文器物之四維藝境加成），實質驅動工坊煉製器物異變率加成（`forge / culinary`）、地景物產採集額外完好收穫與工具磨合／捲刃提醒（`harvest` + `harvest_from_world_object`），以及營火現煮山海名饌滋養增幅與本機 LLM 高光描寫（`try_camp_shanhai_cooking` $\to$ `compose_craft_highlight_from_data`）。
   - **借物歸還器況檢驗與師承同道折讓（`src/server/action/borrow.rs` ＆ `src/server/action/trade.rs`）**：歸還借用器物時自動檢驗 `VesselCondition`——若捲刃待修則掌事皺眉斥責並開啟 `repair_tool` 義務；若经推磨雪亮歸還則大加讚賞並累積《百工惜器訣》師承烙印。身具【師承烙印】或高熟識度者於市井交易享有同道熟客折讓。
   - **行旅涉險與部位傷勢連動（`src/server/handler/movement.rs`）**：野外跋涉自然演進《山林長息》／《海潮綿息》；身負「下盤扭傷」於崎嶇地勢會觸發足踝隱痛提醒，而高 `wayfare` 藝相或長息底色則觸發「長息輕身」回氣減耗。82 項單元與整合測試全數通過。
9. **「設計師上帝視角全地圖（`/world_atlas`）」全面對齊台灣山海經活世界並完成 `ITERATE -> PASS` 視覺驗收**：
   - **後端三端點擴充（`src/server/http_api/eco.rs`、`src/server/run.rs`）**：`/api/eco/map` 完整供應 `14,424` 格地塊與手寫描寫 `d` 及地面遺落物；`/api/eco/entities` 整合全島在場人物與 19 獵場 57 隻《台灣山海經》生靈（含 `breath_habit` 內息底色、`held_vessel` 器物人器磨合・鋒刃器況／可剝取山海靈材、`afflictions` 部位傷勢、`lineage` 師承烙印）並消弭鎖重入死結；新增 `/api/eco/pulse` 供應 15 聚落供需急徵行情與 19 獵場霸主【山海異兆】及天地氣源。
   - **前端上帝全地圖與 `ITERATE -> PASS` 視覺驗收（`web/world_atlas.html`、`src/game/room.rs`）**：修正 `room.rs` 殘留舊奇幻獸名與古河巨木林異兆錯配，全面正名為《台灣山海經》物種（長風四目水鹿王、古河樟林雲豹、赤石鯪鯉穿山甲、淺山錢紋石虎、大武百步蛇王、寒潭千年花斑鱸鰻、霧林黑長尾帝雉、雪巔霜冠大水鹿等）；將 `#inspector-panel` 改為右下角停靠（`right: 68px; bottom: 18px`）並於全島總覽預設收合，確保 `800×1,000` 全島南北視野 100% 無遮擋；新增同格無人時自動調閱鄰近 12 格內在場人物與山海生靈，通過桌面版（`1520×920`）與行動版（`430×932`）實機截圖驗收。
10. **「全島 15 座聚落 595 棟建築零空屋家戶化（`925` 位居民 ＋ `57` 隻山海生靈＝`982` 在場實體）」全數補齊**：
   - **外圍 14 聚落家戶與百工作息全數掛載（`src/store/outer_households.rs`、`src/store/entity.rs`）**：補齊風舶角（42 宅／13 機能，91 人）、四大鎮（風舶嶼 61 人、大圳頭 71 人、半嶺鎮 73 人、南哨 67 人）與九大村（老堰頭、北嶺腳、霜林口、風櫃尾、烏石鼻、牛埔、紅土崁、三號坑口、水尾寮，各 21～27 人）共 `542` 個家戶槽位（全島合計 `866` 家戶槽位＋`59` 獨立當家＝`925` 位活躍居民）。
   - **家庭／師徒一屋（80%）＋單身職人一屋（20%）與在地作息閉環**：全島 `406` 棟民居住宅達成 `100%` 有人居住（平均 $2.28$ 人／宅，零幽靈空屋），`189` 棟機能建築白天 `100%` 有人當值；修正外圍聚落作息與義務路由，確保各城鎮村落居民於所屬聚落之居所、工坊、市集、診所與食堂間自然往返。
## 二、 關鍵檔案與正典錨點

- **Rust 實機代碼**：
  - `src/game/room.rs:442-1090`（十九獵場百獸、霸主異兆、部位奇效与佩飾護佑）
  - `src/game/object_ecology.rs:2505-2630`（營火與食堂「山海合味」現煮料理）
  - `src/combat/wordplate.rs:1-1370`（500 詞一字雙境 `CraftDims4D`、起承轉合語序權重與合成引擎）
  - `data/config/word_elements.json` ＆ `data/config/word_elements_gamedims.json`
- **Wiki 與 Repo 正典文件**：
  - `/home/tanger/Projects/obsidian-vault/wiki/05-專案/Singularity-World/20-世界觀/台灣山海經-十九獵場百獸與台灣物產圖譜.md`
  - `/home/tanger/Projects/obsidian-vault/wiki/05-專案/Singularity-World/30-角色與養成/wordplate-system/`（5 篇正典）
  - `/home/tanger/Projects/obsidian-vault/wiki/05-專案/Singularity-World/20-世界觀/背景設定目前確認內容.md`
  - `/home/tanger/Projects/singularity_world/docs/design/world/current-background-setting.md`

## 三、 下一步接續提示 (Next Cue)
- 實機服務 `singularity.service` 已編譯重啟為最新 `--release` 版本；九大有機活世界系統、百工藝相四維連動與設計師上帝視角全地圖（`http://127.0.0.1:1721/world_atlas`）皆已完成端到端實裝與實機視覺驗收。
