# 規格：disposition 改 Thought Log（NPC 帶來源、有衰減的心境）

> 狀態：待派碼農
> 拍板日：2026-04-30
> 來源：對話坍縮，借鑑 RimWorld Thought 系統 + Raph Koster 「正確時機反應」原則
> Source of truth：本檔；wiki 拍板記錄在 `concepts/npc-system/thought-log.md`（待寫）

---

## 四查證據包

### 查 1：現行 disposition 結構

- `src/entity/mod.rs:107` — `Character.disposition: i32`（clamp -100~+100，0=中性）
- `src/db/disposition.rs` — 模組整體 42 行，提供 `adjust_disposition(entity_id, delta)` 與 `get_disposition(entity_id)`
- 8 個常數：`DISP_BROKE -20` / `DISP_DAILY -2` / `DISP_BEG_SUCCESS +3` / `DISP_GATHER +5` / `DISP_HIRED +20` / `DISP_TRADE +6` / `DISP_TALKED +5` / `DISP_SUBDUED -15`

### 查 2：寫入點（共 9 處）

| 位置 | 觸發 | 目前 delta |
|---|---|---|
| `src/server/action/combat.rs:75` | 被人留人制伏 | `DISP_SUBDUED` |
| `src/server/simulation_loop/work.rs:144` | 排班工作結算 | 變數 `disp_delta` |
| `src/npc/brain_arrival.rs:46` | 乞討成功 | `DISP_BEG_SUCCESS` |
| `src/npc/brain_arrival.rs:52` | 採集成功 | `DISP_GATHER` |
| `src/npc/brain_arrival.rs:73` | 獲指派 | `DISP_HIRED` |
| `src/npc/brain_arrival.rs:80` | 與人交談 | `DISP_TALKED` |
| `src/db/npc_trade.rs:36` | 街頭兜售成功 | `DISP_TRADE` |
| `src/db/npc_expense.rs:35` | 鎂歸零 | `DISP_BROKE` |
| `src/db/npc_expense.rs:38` | 每日支出 | `DISP_DAILY` |

### 查 3：讀取點（共 2 處）

| 位置 | 用途 |
|---|---|
| `src/npc/decision.rs:190-200` (`urgency_social`) | 社交需求權重 |
| `src/db/identity.rs:37-42` (`build_identity`) | LLM 提示組裝（disp>20 → 「你最近心情不錯」；disp<-20 → 「你最近過得不太好」） |

### 查 4：對齊判斷

- 與既有 SoulSeed 三軸不衝突——三軸是性格傾向（Facets），thought log 是當前情緒積累（Mood）
- 與既有 archival/event log 不重複——event 是世界發生的事，thought 是 NPC 對該事的內部反應與權重
- 與 NPC↔NPC 對話四層記憶不衝突——L0/L1/L2/L3 是社交知識，thought log 是個人情緒
- 拍板原則「土壤而非劇本」、「正確時機給對的反應」直接對應這份規格

---

## 設計核心

把 `disposition: i32` 換成**有來源、有強度、有衰減**的 thought 列表。當前情緒值 = log 中尚未衰減完的 thought 加總。

**Why：**

- 現在 disposition 是個沒記憶的數字——`+3` 之後就是 `+3`，沒人記得當初為何 `+3`、何時 `+3`
- 結果 `build_identity` 對 LLM 只能說「你最近心情不錯」這種無內容的話，所有 NPC 的 mood 線索都是同一句
- 改成 thought log 後，identity 能說出「上午乞討被拒，現在還悶著」「中午採到野莓，雀躍」「鎂袋癟了，發愁」——具體、有畫面、有時間感，LLM 拿到才寫得出有溫度的對話
- 同樣的工程量（一個欄位變一個結構），對話的「人味」上限躍升

**How to apply：** NPC 任何「現在心情如何」的查詢都走 thought log；舊的 i32 disposition 是它的純函數投影。

---

## 改動規格

### 1. 新增資料結構（`src/entity/mod.rs`）

```rust
/// NPC 內心活動，帶來源、強度、衰減期。
/// 加總尚未衰減完的 thoughts.delta = 當前 disposition 投影值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    /// 來源 key（對應 ThoughtSource 列舉的字串值，便於 i18n／散文化）
    pub source: String,
    /// 強度（正負，未限幅；最終 disposition 投影時才 clamp）
    pub delta: i32,
    /// 創建時的世界遊戲分鐘（從 gametime::now_minutes() 取）
    pub created_at: i64,
    /// 衰減期（遊戲分鐘，超過即不計入）
    pub decay_minutes: i64,
    /// 給 LLM 用的散文，由 source + 詞盤 / 簽名後續變化（可空，預設由 source 對應表抽）
    pub flavor: Option<String>,
}
```

`Character` 加一個欄位（**保留** `disposition: i32` 作為 cache，每次 thought 變動時重算一次寫入；保證讀取端零改動）：

```rust
/// NPC 內心活動列表（最近 100 條，超過砍最舊）
#[serde(default)]
pub thoughts: Vec<Thought>,
```

### 2. 新增 ThoughtSource 列舉（`src/db/disposition.rs` 或 `src/npc/thoughts.rs` 新檔）

替換現有 8 個常數，每條 source 對應 (delta, decay_minutes, default_flavor)：

| source | delta | decay (min) | default_flavor |
|---|---|---|---|
| `BegSuccess` | +3 | 240 (4h) | "乞了一頓飯，肚裡踏實了些" |
| `Gather` | +5 | 360 (6h) | "採到一些野貨，心裡踏實" |
| `Hired` | +20 | 1440 (24h) | "得了差事，心裡寬慰" |
| `Trade` | +6 | 240 (4h) | "今日生意做成幾筆" |
| `Talked` | +5 | 180 (3h) | "跟人說了會兒話" |
| `Broke` | -20 | 1440 (24h) | "鎂袋空了，心裡發慌" |
| `Daily` | -2 | 720 (12h) | "今日花用又見底，發愁" |
| `Subdued` | -15 | 2880 (48h) | "被人留下制伏，心裡有氣" |
| `WorkDone` | (動態) | 360 | "今日活幹完了" |

衰減策略：**線性衰減到 0**，不是斷崖。當 thought 已存在時間 t，有效 delta = `delta * max(0, (decay - t) / decay)`。

### 3. 改 disposition.rs API

```rust
pub fn add_thought(entity_id: &str, source: ThoughtSource) -> anyhow::Result<()>;
pub fn add_thought_with_flavor(entity_id: &str, source: &str, delta: i32, decay: i64, flavor: String) -> anyhow::Result<()>;

/// 即時計算當前心情（thought log 加總後 clamp）。
pub fn get_disposition(entity_id: &str) -> i32;  // 簽名不變

/// 取最近 N 條未衰減完的 thoughts，給 LLM prompt 用
pub fn get_active_thoughts(entity_id: &str, limit: usize) -> Vec<Thought>;
```

`adjust_disposition(entity_id, delta)` **保留** 作為 deprecated 兜底，內部呼叫 `add_thought_with_flavor` 用 `"Legacy"` source、衰減 6h。逐步淘汰。

### 4. 改寫入點（9 處）

每個現有 `adjust_disposition(eid, DISP_X)` 呼叫，改為 `add_thought(eid, ThoughtSource::X)`。delta、decay、flavor 由 ThoughtSource 表查。

工作結算 (`work.rs:144`) 的動態 `disp_delta` 走 `add_thought_with_flavor` 帶具體 flavor（例如「在 X 場所做了 Y 工，賺了 N 鎂」）。

### 5. 改讀取點（2 處）

**`src/db/identity.rs:37-42` 大改**——

```rust
// 原本
let disp = get_disposition(entity_id);
if disp > 20 { result.push_str("你最近心情不錯。"); }
else if disp < -20 { result.push_str("你最近過得不太好。"); }

// 改為
let thoughts = get_active_thoughts(entity_id, 5);
if !thoughts.is_empty() {
    result.push_str("你最近的心境：");
    for t in &thoughts {
        if let Some(f) = &t.flavor {
            result.push_str(f);
            result.push('；');
        }
    }
}
```

`urgency_social` (`decision.rs:190`) 不動——它讀的是 `get_disposition(entity_id)` 的 i32 投影，簽名沒變。

### 6. cache 維護

`Character.disposition: i32` 不刪，作為 thought log 加總的 cache。每次 `add_thought` 後重算寫回。`Character.thoughts` 是 source of truth。

### 7. log 大小控制

每次 `add_thought` 後，掃 `thoughts`，砍掉 `created_at + decay_minutes < now` 的條目（已完全衰減）。再加一道：若條目仍 > 100，砍最舊的。

### 8. 序列化

`thoughts` 走 serde 預設，跟現有 entity JSON 同層級。新欄位 `#[serde(default)]` 確保舊資料相容（無此欄則為空 Vec）。

---

## 驗收標準

- [ ] **無 build_identity 改動的 NPC 拿到的 prompt 包含具體 thought**，不再是「你最近心情不錯」一句
- [ ] **同一個事件不同詞盤的 NPC 有同樣 delta，但 flavor 文案可不同**（先期 flavor 都用 default，留 hook）
- [ ] **舊存檔載入正常**——舊 entity JSON 無 thoughts 欄位，反序列化後 thoughts = []，disposition 仍從欄位讀（cache）
- [ ] **disposition i32 投影值 = thoughts 加總 clamp [-100, 100]**——任何時刻 `get_disposition` 跟 `thoughts.iter().map(active_delta).sum().clamp` 一致
- [ ] **thoughts 不無限長**——衰減完即砍、上限 100 砍最舊
- [ ] **9 個寫入點全部走新 API**，舊 `adjust_disposition` 只剩 deprecated 兜底
- [ ] **`cargo build --release` 過、`cargo clippy -- -D warnings` 過、`cargo test` 過**

---

## 不做（明確排除）

- **詞盤訊息影響 thought flavor**——本規格只接 source → default flavor。詞盤色染留下一階段
- **不同 NPC 對同事件不同 delta**（例如膽大者被 subdue 衝擊較小）——本規格 delta 仍是常數，個性差由現有 SoulSeed 三軸獨立疊加
- **thought 進 LLM 對話池影響選詞**——本規格只進 identity prompt。對話池接入是下一步
- **歷史 thought 永久保存進 archival**——本規格 thought 是衰減態，衰減完即丟。永久記憶走既有 archival/summary 路徑
- **DB schema 遷移**（thoughts 欄位寫進 PostgreSQL）——本規格走 store 層 in-memory + JSON 序列化即可，DB 落庫留下一階段

---

## 對既有設計的承諾

- v5 描述欄四層 ✓ 不衝突，反而強化（NPC 的 thought 可以滲進 L4 物件卡片的 hover 文案）
- NPC 決策引擎 V2 ✓ `urgency_social` 仍走 `get_disposition` i32 投影，零改動
- NPC↔NPC 四層對話記憶 ✓ 獨立系統，不交叉
- archival/summary ✓ 兩者並存——archival 是事件歷史、thoughts 是內心反應
- 馬斯洛需求引擎 ✓ thoughts 不替代需求，是需求滿足/挫敗的「後座感受」

---

## 預期效果

`build_identity` 給 LLM 的 prompt 從：

> 你是老李，職稱是麵攤老闆，在客棧街工作。性格大膽、敢衝。你最近心情不錯。

變成：

> 你是老李，職稱是麵攤老闆，在客棧街工作。性格大膽、敢衝。你最近的心境：今日採到一些野貨，心裡踏實；跟人說了會兒話；今日花用又見底，發愁。

LLM 拿到後寫出來的對話完全不同——後者有具體著力點，前者只能擠空話。

這就是 Raph Koster 講的「正確時機給對的反應」——引擎不需要更聰明，只需要把已經發生的事**有結構地累積**起來，讓對話層拿得到具體素材。
