# 奇點世界 v3 下游三包外部校準研究

狀態：研究校準筆記；2026-06-21 已標記部分錯誤校準：供能普及/可靠性資料把後續設計帶向電力化供應模型，只能保留生活密度、物件動詞、命名與材料功能方面的參考價值
日期：2026-06-21
第一權威：`../../DESIGN_KEYWORDS.md`
承接：`../world/world-energy-evolution-root-v0.md`、`../world/world-bible-v3.md`

## 研究邊界

本研究只回答一件事：如何把「平行宇宙的能量充沛地球」落到材料、勢力、地圖三個下游設計包，而不是照抄地球史、現代電力系統或其他奇幻作品。

研究資料只作校準，不作奇點正典。奇點正典仍由 `DESIGN_KEYWORDS.md`、`world-energy-evolution-root-v0.md`、`world-bible-v3.md` 決定。

2026-06-21 修正：能量不是電力，不能把現實電力普及、電網可靠性、供應接入和機構控制搬成奇點模型。本研究中關於供能普及、可靠性與制度的材料，只可作為「成熟文明會提高生活密度、產生維護與責任問題」的弱參照；不得再推導 `普供/主供/低供/供能/接入/誰控制供能誰掌控`。

## 外部資料

| 面向 | 資料 | 校準結論 | 奇點轉譯 |
|---|---|---|---|
| 遊戲世界建構 | GameDesignSkills: Worldbuilding in Game Development；The Level Design Book: Worldbuilding | 遊戲世界建構要把設定接到環境敘事、互動設計、程序生成與關卡/地圖設計；大型長期專案可以有 map、timeline、notes，但不能脫離可玩需求。 | v2 三包不能只補百科，要能生成玩家看到的物件、NPC 行動、地圖路線、價格與衝突。 |
| 電力史參照 | Richmond Fed rural electrification；Southern Oral History Program；NRECA history；World Bank productive uses of electricity | 現實電力普及可提醒「生活密度、工具、夜間活動、教育、維護與責任」會改變，但不能搬成奇點的能源模型。 | 只保留生活密度與制度問題的校準；不得推導供能、接入、斷供或配給。 |
| 可靠性與制度參照 | FERC reliability explainer；NERC reliability standards；NIST materials science | 可靠運行、標準、維護、量測、證據與故障隔離可作成熟系統參照；材料也應從加工、結構、性質、性能看用途。 | 勢力只能控制介面、工具、訓練、風險資料、禁區、保固、責任與社會承認；不能控制能量本身。 |
| 材料功能 | NIST materials science；DOE insulation | 材料價值來自性質與性能，不只名稱；隔熱/隔離、導流、反射、散熱、濕度老化等會決定用途與失效。 | 材料 v2 先從導能、蓄能、轉能、隔能、散能、感能推導，再落具名材質。 |
| 物件與動詞 | Machinations affordances in game systems design；Game Mechanics sample on sources, drains, converters, traders | 好物件要讓玩家看出可做動詞；經濟要有來源、移除、轉換、交易，不能只做固定收購。 | 每個材料/工具都要有動詞、限制、正負結果、轉換鏈與回訪痕跡。 |
| 地圖與命名 | The Level Design Book: Wayfinding；International Health Facility Guidelines wayfinding；OpenStreetMap Names；Kevin Lynch The Image of the City | 好地圖讓玩家能定位和找路；地點要有身份、區域、路徑、節點、邊界、地標；名字應是玩家/本地人會用的名稱，不是描述句或作者備註。 | 地圖 v2 的命名要通俗可讀，能反推尺度、功能、風險與成因；M1/M2/M3/M4 要讓玩家一眼知道能去哪裡和碰什麼。 |

## 來源

- https://gamedesignskills.com/game-design/worldbuilding/
- https://book.leveldesignbook.com/process/preproduction/worldbuilding
- https://book.leveldesignbook.com/process/blockout/wayfinding
- https://www.richmondfed.org/publications/research/econ_focus/2020/q1/economic_history
- https://sohp.org/exhibit/digital-exhibit-example-copy-2/
- https://www.electric.coop/our-organization/history
- https://openknowledge.worldbank.org/bitstreams/e7158c70-ca3d-5e7d-a1a3-9d4fcec63938/download
- https://www.ferc.gov/reliability-explainer
- https://www.nerc.com/standards/reliability-standards
- https://www.nist.gov/mml/materials-science-and-engineering-division
- https://www.energy.gov/energysaver/insulation
- https://machinations.io/articles/affordances-in-game-systems-design
- https://ptgmedia.pearsoncmg.com/images/9780321820273/samplepages/0321820274.pdf
- https://healthfacilityguidelines.com/Guidelines/ViewPDF/iHFG/iHFG_part_w_wayfinding_design_principles
- https://wiki.openstreetmap.org/wiki/Names
- https://cus.ubt-uni.net/wp-content/uploads/2024/11/Kevin-Lynch-The-Image-of-the-City.pdf

## 對三包的硬校準

### 材料包

材料不是「自然物 + 用途」。材料必須回答：

- 它是哪一種能量功能族。
- 它如何被取得、加工、定標、保存、失效。
- 它提供哪個玩家動詞。
- NPC 會如何同規則使用、拒收、排隊、偷換或追責。
- 它如何進入來源、轉換、交易、損耗或事故。

### 勢力包

勢力不是王國、行會或門派表。勢力必須回答：

- 它控制哪種介面、工具、技藝、風險資料、禁區、標準、教育、認證、保固或責任。
- 誰因它受益，誰被它排除。
- 玩家如何在畫面上碰到它的規矩。
- NPC 未被玩家接觸時如何被它約束或利用它。

### 地圖包

地圖不是地名數量。地圖必須回答：

- 這個地帶的能量地質/公共節點/生活密度/事故歷史/生態能流/社會排除是什麼。
- 名字是否是玩家和居民會用的通俗名稱。
- 玩家站入時能看到什麼路、物、人、標記和風險。
- 玩家可做的第一個動詞如何改變能量狀態、路況、物件、NPC 或信用。
- 回訪時如何顯示世界沒有因玩家離開而停止。
