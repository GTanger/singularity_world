# E2E 驗收基線

方向四重構（Store God Object 拆解）前的安全網。

## 用法

```bash
cd tests/e2e
npm install   # 只要跑一次
npm test      # 跑全套（需 server 跑在 localhost:1721 + PG 在 docker）
```

## 涵蓋

| # | Scenario | 驗什麼 |
|---|---|---|
| 01 | ping | 最基本 WS 心跳 |
| 02 | create-character | 創角 → me + PG entities/auth 寫入 |
| 03 | login | 既有帳號登入 → view/me |
| 04 | login-wrong-password | 錯密碼被拒 |
| 05 | move | 移動一格後 player_x/y 更新 |
| 06 | stress-then-login | A 狂 move + B 同時創角 → B 不卡 |
| 07 | disconnect-stress-login | **核心 bug 回歸點**：壓測斷線後新 client login 應 < 3s |
| 08 | broadcast | 多玩家同房間 view.entities 更新 |
| 09 | concurrent-login | 10 client 並發 login 全 OK |
| 10 | reconnect | 同帳號連續 5 次登入／斷線 |
| 11 | grid-reveal | 移動後 explored 格數不減 |
| 12 | invalid-move | 四向不存在的方向（如「東北」）應被拒 |
| 13 | entity-persistence | PG entities 欄位正確（display_char/gender/hex_q/hex_r） |
| 14 | stats | me 訊息帶合理 hp/inner/spirit/stamina/vit |
| 15 | inventory | get_inventory 查詢 |

## 現狀 baseline（2026-04-21 方向四前）

```
8 passed, 7 failed
```

failing 的都是 lock 爭用症狀的直接後果（server 被前面 test 拖進卡住狀態）。方向四重構後這些應該變 PASS——若沒變、重構無效。

## 為何 test 之間有 cascade

現階段 server 是全局 singleton、test 共用 state。方向四把 Store 拆成 domain 後，理想上可以每 test 獨立 DB instance、無 cascade。但這是後續改進、不擋現在的 baseline。
