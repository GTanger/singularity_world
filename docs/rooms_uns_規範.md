# Singularity World 房間全球命名規範 (Singularity UNS)

為了確保地圖在無限擴展、座標定位以及 NPC 自動尋路時的絕對嚴謹性，Singularity World 採用 6 層級底線分隔的命名體系。

## 格式定義 (Structure)
`[ZONE]_[TYPE]_[INSTANCE]_[FLOOR]_[COOD/AREA]_[INDEX]`

1. **ZONE (區域)**: 地理大分區。如 `zonelife` (浮生區)。
2. **TYPE (類型)**: 區域內的景觀類型。如 `city` (城市)、`wild` (荒野)。
3. **INSTANCE (實例)**: 具體的建物或聚落名稱。如 `life` (浮生城主體)。
4. **FLOOR (樓層)**: 3 位數樓層碼。如 `00f` (通用/貫通)、`01f` (一樓)、`16f` (十六樓)。
5. **COORD/AREA (座標或區代碼)**: 
   - **座標規格 (uXYZ)**: `u` + 樓層(2碼) + 格位(1碼)。
     - 範例：`u160` 指 16 樓的 0 號格。
     - **規範：** 結尾為 `0` 的格位固定作為該樓層的「大廳 / 樞紐 (Hall/Lobby)」。
   - **區域規格 (代號)**: 用於跨樓層或特殊機能區。如 `elevator` (電梯間)、`cofe` (咖啡廳區域)。
6. **INDEX (序號)**: 區域內的細分格位。從 `0` 開始編號。
   - `0`: 該區域的中心、入口或第一格。
   - `1, 2, 3...`: 該區域的延伸分區。

---

## 範例 (Examples)

### 16 樓行政區
- **16 樓大廳**: `zonelife_city_life_16f_u160_0`
- **加了福 (161)**: `zonelife_city_life_16f_u161_0`
- **沃而滿 (164) 分區**: `zonelife_city_life_16f_u164_1`

### 功能性設施
- **通用電梯間**: `zonelife_city_life_00f_u000_elevator_0`
- **1 樓蕗易沙 (入口)**: `zonelife_city_life_01f_cofe_0`
- **1 樓蕗易沙 (座位)**: `zonelife_city_life_01f_cofe_1`

---

## 遷移規則 (Migration Rules)
1. 無論是 JSON 檔案名稱還是 `move_to_room_id` 引用，皆須 100% 符合此規範。
2. 禁止手動修改 ID，必須透過 Dashboard 的 `rename_room` API 或自動腳本進行，以確保資料庫（rooms / exits / rumors）同步更新。
