# 決策 006：登錄與玩家模板實作順序

> 討論：先實做登錄，再實做玩家模板（創造第一位玩家角色）；玩家＝NPC，模板做完所有 NPC 可沿用。

---

## 一、順序與範圍

| 階段 | 內容 | 說明 |
|------|------|------|
| 1. 登錄 | 識別「誰在連線」、建立／取得可持久的身分 | 目前：前端送 `player_id`，後端查 entities 有無此人；無則 "player not found"。尚無註冊／建立身分流程。 |
| 2. 玩家模板 | 用「同一套實體結構」創造遊戲第一位玩家角色 | 即「創造角色」流程：寫入 entities 一筆 `kind=player`，並寫入 entity_room；之後登錄用此 id。 |

先做登錄，再做玩家模板，邏輯上合理：先決定身分（或先有「可登入的入口」），再在該身分下建立或選角。

---

## 二、玩家＝NPC，NPC 可否沿用？

**可以，且現況已是共用。**

- **資料**：`entities` 表與 `entity.Character` 結構已為玩家／NPC 共用（id, kind, display_char, x, y, move_state, vit, qi, dex, magnesium, last_observed_at, …）。
- **語義**：`kind` 區分 `player` | `npc`，其餘欄位一致；對齊 [人物角色模板](../reference/人物角色模板.md) §一、§八。
- **結論**：玩家模板＝「用這套結構建立一個 `kind=player` 的實體」；之後所有 NPC 同樣用這套結構寫入 `kind=npc` 即可，無需另一套模板。第一版最小集已足夠（必要時再擴欄位）。

因此：**玩家模板做完，所有 NPC 都沿用同一套 entities／Character 結構與寫入方式。**

---

## 三、登錄可選方向（已選：A ＋ id＋密碼）

| 方案 | 說明 | 優點 | 缺點 |
|------|------|------|------|
| A. 維持現狀＋「建立角色」 | 仍用 player_id 登入；若 DB 尚無該 id，先走「創造角色」流程（取名、display_char 等），寫入 entities＋entity_room 後再當登入成功 | 改動小、與現有 WS 相容 | 身分僅「一個 id」，無帳密、多裝置同角色需自己保管 id |
| B. 簡易帳號＋角色 | 先「註冊／登入帳號」（例如帳密或 token），再選角／建角；角色仍寫 entities，帳號與角色可一對多 | 可支援多角色、多裝置 | 需帳號表、登入態、選角 UI 與協議 |
| C. 純訪客＋建角 | 無帳號；連線後若無角色則引導「建立第一位角色」，id 由系統產生（如 UUID），存 localStorage 或 cookie | 免註冊即可玩 | 換裝置／清快取即「新角色」 |

第一版採用：**方案 A，並加上玩家自訂登入密碼**。創角時設定 id＋密碼；之後開啟網頁即為「輸入 id＋密碼」登入。

---

## 四、登錄與密碼（方案 A ＋ id＋密碼）

### 4.1 行為

| 情境 | 行為 |
|------|------|
| **首次／創角** | 前端顯示創角表單：輸入 id、密碼、display_char（可選或預設）。送出後後端建立 entities＋entity_room＋密碼檔，並視為登入成功，送 view／me。 |
| **之後每次開網頁** | 前端顯示登入表單：輸入 id＋密碼。送出後後端驗證密碼，通過則與現有流程相同（EnsureEntityInRoom、view、me）。 |

### 4.2 密碼儲存

- **不存明碼**：只存雜湊（例如 bcrypt 或 Go 內建 `golang.org/x/crypto/bcrypt`）。
- **儲存位置二選一**：  
  - **選項甲**：新表 `entity_auth (entity_id PRIMARY KEY, password_hash TEXT NOT NULL)`，僅玩家有列；NPC 不碰。  
  - **選項乙**：`entities` 加欄 `password_hash TEXT`（nullable）；僅 `kind=player` 填值，NPC 留 NULL。  

建議 **選項甲**（獨立 entity_auth），與 entities 模板分離、遷移單純。

### 4.3 協議要點

- **創角**：例如 WS 訊息 `create_character` 或 HTTP POST，欄位：`id`, `password`, `display_char`（選填）。後端檢查 id 未被使用 → 寫入 entities、entity_room、entity_auth → 回傳成功並可同一連線直接送 view／me 或要求客戶端再送 login。
- **登入**：現有 `login` 擴充為必帶 `player_id`＋`password`；後端查 entity 存在且 kind=player，再查 entity_auth 驗證密碼，通過後同現有流程。

---

## 五、玩家模板實作要點（創造第一位玩家）

- **輸入**：創角時由前端提供 id、密碼、display_char（或預設）。
- **寫入**：  
  - 一筆 `entities`（id 由使用者輸入；kind=`player`；display_char, vit, qi, dex, magnesium 等依第一版最小集或固定預設）。  
  - 一筆 `entity_room`（該 id 放進預設出生房，與現有 `EnsureEntityInRoom` 對齊）。  
  - 一筆 `entity_auth`（entity_id＝同上 id，password_hash＝雜湊後密碼）。
- **後續**：創角成功後即視為登入（或回傳成功後前端再送 id＋密碼當 login）；之後每次開網頁為「輸入 id＋密碼」登入。

NPC 仍沿用同一套 `entities`＋`entity_room`，不寫入 `entity_auth`。

---

## 六、與清單對應

- **1.5.1**（玩家／NPC 同一套實體結構）：已由 schema ＋ `entity.Character` 滿足；玩家模板實作＝「建角流程」接上這套結構。
- **登錄**：對應擴充現有 WS login 流程（可選 A/B/C 之一）；若採「無則建角」，即與玩家模板一併完成「創造第一位玩家角色」。

---

*奇點世界 — 決策 006：登錄與玩家模板實作順序與 NPC 沿用*
