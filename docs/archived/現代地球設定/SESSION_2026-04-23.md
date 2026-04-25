# Session Checkpoint — 2026-04-20

> Token 公理校準 + 第十話入典 + G集團支配範圍定邊界

## 一句話總結

第十話「打怪獸了」完成並解決「為何只有 G 集團」設定邏輯；Token 物理四處校準把游離輻射、水的角色、高頻擠/低頻搶食、個體念場梯度全部釘死成硬公理；附篇綠植從「屏障」改寫為「共生清道」。

## 檔案層級產出

### 敘事（singularity_world commit `ed47e5ff`）
- `docs/stories/010_打怪獸了.md`（新，正典錨點）
  - O集團執行長遭大創世紀降臨 T 彈攻擊 + 配偶法說會接手
  - 美元紀西方寡頭 M/X/F/A + O 五家同日併入 G 集團
  - 「奧特曼去地球打怪獸了」× 3 次 + 同心圓磨損表（硬約束不得刪）

### Token 公理四處校準
- `docs/reference/世界觀：Token降維與生命演化.md`
  - §一 補：「游離輻射散逸態」+「水為 Token 能量載體 / 生命起源」
  - §二 擴：「高頻擠 / 低頻搶食」通則表 + 富態化雙機制（無念累積 vs 低頻過量）
- `docs/design/歷史模擬器—規格草案.md`
  - §4.3 擴：擠者限高頻、個體念場梯度、富態化雙路 L1 公式指示
  - §6.1 已補（前輪）：歷史合法性節點 + G集團支配範圍硬約束（美元紀西方寡頭承接體系，非全球單頭；東亞/中東/俄歐留白）
- `docs/reference/世界觀附篇—電子與精密機械.md`
  - 綠植從「吸收/緩衝輻射」→「搶食殘餘共生模型」
- `CLAUDE.md`：浮生城漏網替換收尾

### Wiki 同步（obsidian-vault commit `c6722c0`）
- `wiki/concepts/worldview/token-ontology.md` — 補游離輻射/水載體/高頻擠低頻搶食/個體念場梯度
- `wiki/concepts/worldview/electronics-and-machinery.md` — 綠植共生模型
- 連帶清理：9 份 wiki md「浮生城→城鎮」殘留一併提交
- `.gitignore`：`.llm-wiki-kit.db` 移出版控
- 根目錄野檔 `concepts/ design/ narrative/` 清除（graphthulhu 匯出跑錯路徑的副本）

## shodh 入庫決策（5 條）
- `958128dc` — 游離輻射非常規物理輻射，屏蔽/衰減詞彙禁用
- `a0bc374d` — 水為 Token 能量載體 + 生命起源推導
- `0ef7c55f` — 高頻擠 / 低頻搶食通則 + 富態化雙機制
- `03a6689c` — 個體念場梯度（一般民眾眼前一米，功法增強擴大）
- `0a374e46` — 第十話入典 + G集團支配範圍（非全球單頭）

## 本輪關鍵反轉（教訓留存）
- 第一版移動機制推導違反 Token 公理（假設水屏蔽、預設人人念場米級）→ 整份退回
- 正解三條：水是載體非屏蔽、擠者限高頻、個體念場是修煉成果非標配
- 綠植不是防護層，是低頻搶食殘餘的共生夥伴
- 游離輻射是 Token 混沌能量被動散發的微弱能量，不是常規物理輻射 → 後續勿用屏蔽/衰減/反射詞彙

## 下場繼續的線頭

本輪碰規格 §4.3 時提到「L1 富態化擴張公式需兼顧無念累積與低頻過量兩路」——公式目前是單路，實作時要擴。不急，記著。

Token 公理更新後，5 話以前的敘事是否需要一致性校對？——現狀：底層物理是敘事層、代碼層用通則，前 9 話的描述不需回改（feedback: dual_layer_standard）。新產出對齊就行。

## Git 狀態
- `singularity_world` @ `ed47e5ff` master — pushed
- `obsidian-vault` @ `c6722c0` master — no remote, local clean
- `singularity_simulator` — 本輪未動
