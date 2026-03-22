# tools/ — 008 P6 工具集

與執行期伺服器分離的一次性／維護用程式。

| 路徑 | 內容 |
|------|------|
| `go/*` | 原 `cmd/*` 與原 `scripts/` 內可執行 Go 小工具 |
| `js/*` | 原 `scripts/*.js`（房間 JSON 等） |
| `py/*` | 原 `scripts/*.py` |

**執行範例**（專案根目錄）：

```bash
go run ./tools/go/soulseed_demo/main.go
node tools/js/merge-rooms-one-per-line.js
go run ./tools/go/clean-npc-npc-pollution
```
