# 2026-06-19 元素系統外部研究

狀態：外部研究筆記，不是奇點權威
第一權威：`docs/DESIGN_KEYWORDS.md`
對接文件：`docs/design/world/world-foundation-v0.md`

## 研究對象

本輪查的是「奇幻/RPG/遊戲裡元素系統到底怎麼組成」，用來修正四元素只列名的問題。

要回答：

- 元素是物質、屬性、相位、傷害類型，還是互動規則。
- 元素系統如何落到世界、物件、材料、人、戰鬥。
- 奇點應該採用什麼最小結構，避免一開始就膨脹成百科表。

## 查到的重點

| 來源 | 查到的重點 | 對奇點的轉譯 |
| --- | --- | --- |
| Britannica, `chemical element` | 古典四元素不是現代化學元素，而是用火、水、氣、土承載乾/熱/濕/冷等特性，解釋萬物性質。 | 奇點的元素應先是「可讀性質」，不是化學週期表，也不是玩家面板數值。 |
| Britannica, `wuxing` | 五行是 earth/wood/metal/fire/water 五個「phase」，重點是宇宙變化與循環，不只是物質名單。 | 木、金更適合當材料族群或變化相位，不必和火水風土混成同一層。 |
| Britannica, `qi` | 氣被描述為瀰漫宇宙的心理-物理能量，並透過陰陽與五行表現為世界基本過程。 | 若奇點日後需要「能量」，也應是底層活性/流動的解釋，不應回到舊特殊名詞或玩家負擔。 |
| Breath of the Wild chemistry engine | 元素可改變材料狀態；元素可改變元素狀態；材料不能直接改變材料。 | 奇點元素系統應以「元素改狀態」為核心：火燒木、水滅火、潮氣壞貨、風散霧。 |
| Zelda simple-system analysis | 小型元素表更易讀；物件有少數屬性如 material、elemental nature、buoyancy 等，就能支撐大量玩家實驗。 | 手機文字 MUD 不應一開始做十幾種元素；先用少數基礎元素 + 少數物件屬性。 |
| Wired, elemental reactions | 元素反應讓玩家清楚知道自己的行動如何改變世界；環境能變成玩家策略的一部分。 | 奇點元素不是文案氣氛，而要讓玩家能判斷「我這樣做會讓場景/物件/戰鬥發生什麼」。 |
| Roll20 / D&D 5e damage types | 傷害類型本身沒有獨立規則，但抗性等其他規則會依賴它們；類型用來標記有害效果。 | 奇點戰鬥可把元素當傷害/狀態標籤，但真正有趣的是抗性、環境、工具、後果如何引用標籤。 |
| Genshin elemental reactions | 元素先附著為狀態，再由第二元素觸發反應；反應可作用於敵人、玩家或物件。 | 奇點可借「先附著、後觸發」的簡化想法，但不能變成純戰鬥配隊系統。 |
| NOAA / NWS / UCAR lightning science | 雷電來自雷暴中電荷分離與電場累積；冰晶、雹粒、過冷水滴、上升/下降氣流與地面/雲內電荷差是關鍵。 | 雷不應寫成「風/火高能」。雷是雲氣、冰水、風暴流動與導體/地勢共同形成的放電現象；火只是雷可能造成的結果。 |

## 修正結論

奇點元素系統不應只有「火水風土」四個名字。

更重要的是，元素系統不應停在自然物理現象。自然來源只解釋自洽性；遊戲設計要先回答玩家動詞、世界狀態、NPC 同規則與回訪痕跡。這部分已拆到 `docs/design/gdd/15-elements-fantasy-gameview.md`。

應分三層：

### 1. 基礎元素

世界最底層可讀性質，第一版保留四個：

- 火：熱、燃燒、乾燥、爆裂、灼傷。
- 水：流動、濕、霧、潮、腐壞/保存。
- 風：流向、聲音、速度、散播、視線干擾。
- 土：重量、承載、阻擋、生長、礦石。

### 2. 派生狀態 / 傷害標籤

不是新元素，而是元素作用後留下的狀態：

- 冰：水 + 寒冷狀態。
- 雷：雲氣/冰水 + 風暴流動造成電荷蓄積，再經由地勢、金屬、水體或生物導出；可先不進 P0。
- 毒：水/土/生物腐敗派生狀態。
- 酸：水 + 腐蝕性材料狀態。
- 煙：火 + 風或火 + 濕物。
- 泥：水 + 土。
- 蒸氣：火 + 水。

### 3. 材料族群

不是元素，而是承載元素性質的物質家族：

- 木：偏生長、可燃、可腐、可加工。
- 金/金屬：偏導熱、導電、硬、可鍛、重。
- 石/礦：偏土、硬、重、可裂、可熔煉。
- 草/纖維：偏木/風/水，易燃、易濕、可編織。
- 皮/骨：偏生物材料，可能承載毒、腐、韌性。

## 奇點應採用的最小規則

1. 每個物件有 `material_family`，例如木、金屬、石、草纖維、皮骨、液體。
2. 每個物件可有 `element_state`，例如濕、熱、燃燒、腐、硬化、煙、泥。
3. 元素行動先改狀態，不直接抽象扣血。
4. 戰鬥中的元素性來自狀態、地形、武器與對手，而不是每個人天生有元素職業。
5. 玩家文字要先看到狀態，再做判斷：濕木、熱鐵、煙霧、泥地、腐貨、乾草。
6. 元素系統初期只需要讓世界更可讀、更可操作，不需要做完整相剋表。

## 雷的修正

雷不是基礎元素，也不是火 + 風。

雷在奇點裡應定為「放電現象」：

| 層 | 雷的規則 |
| --- | --- |
| 來源 | 雷暴雲、雲氣、冰水顆粒、雹、強上升/下降氣流造成電荷分離 |
| 蓄積 | 雲內、雲與地面、地貌高點、金屬、濕物、水面之間形成電位差 |
| 導出 | 金屬、濕地、水面、高樹、高坡、礦脈、生物身體可能成為路徑 |
| 表現 | 閃光、巨響、焦痕、麻痺、擊穿、點燃、短時失明或驚嚇 |
| 遊戲用途 | 天候危險、地形判斷、金屬/水體風險、工具導引、戰鬥場地變化 |

四元素內部對應：

- 水：雲氣、雨、濕度、冰水顆粒。
- 風：雷暴流動、上升氣流、雲層推移、聲音傳遞。
- 土：高地、礦脈、樹、地面承接。
- 火：不是來源；火是雷擊後可能出現的熱、灼傷、點燃與焦黑。

因此文件中凡寫「雷 = 風/火高能」都要改掉。雷的核心不是燃燒，而是蓄積與導出。

## 對 `world-foundation-v0.md` 的修正要求

- 把四元素列表升級成三層：基礎元素、派生狀態/傷害標籤、材料族群。
- 保留來源/承載/狀態/作用/後果五欄。
- 加一條硬規則：元素不先是傷害數字，而先是世界狀態變化。
- 加一條禁區：不要把所有常見 fantasy 詞都升格成元素。

## 來源

- Britannica, `Chemical element`: https://www.britannica.com/science/chemical-element
- Britannica, `Wuxing`: https://www.britannica.com/topic/wuxing
- Britannica, `Qi`: https://www.britannica.com/topic/qi-Chinese-philosophy
- GamesBeat, `Breath of the Wild makes chemistry just as important as physics`: https://gamesbeat.com/the-legend-of-zelda-breath-of-the-wild-makes-chemistry-just-as-important-as-physics/
- Zelda Universe, `Simple complexity`: https://zeldauniverse.net/features/simple-complexity-how-breath-of-the-wilds-simple-systems-generate-infinite-fun/
- Wired, `Why Do Exploding Barrels Make Video Games So Much More Fun?`: https://www.wired.com/story/exploding-barrels-video-games-elemental-reactions/
- Roll20, `D&D 5e Combat - Damage Types`: https://roll20.net/compendium/dnd5e/Combat
- HoYoLAB, `Elemental effects and reactions guide`: https://www.hoyolab.com/article/93204
- NOAA JetStream, `How Lightning is Created`: https://www.noaa.gov/jetstream/lightning/how-lightning-is-created
- NWS, `Understanding Lightning Science`: https://www.weather.gov/safety/lightning-science-overview
- UCAR Center for Science Education, `Thunder and Lightning`: https://scied.ucar.edu/learning-zone/storms/thunder-and-lightning
- Britannica, `Lightning`: https://www.britannica.com/science/lightning-meteorology
