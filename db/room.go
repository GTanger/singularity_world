// Package db 房間與出口查詢、實體所在房間，供傳統 MUD 節點連接節點機制。
package db

import (
	"database/sql"
	"encoding/json"
	"errors"
	"log"
	"os"
	"strings"

	"singularity_world/entity"
	"singularity_world/model"
	"singularity_world/store"
)

// 與 model 一致，供既有程式與 JSON 使用；store 只依賴 model 以打破 import cycle。
type Room = model.Room
type Exit = model.Exit

// roomsFile 房間定義檔 JSON 結構。
type roomsFile struct {
	Rooms []roomDef `json:"rooms"`
	Exits []exitDef `json:"exits"`
}
type roomDef struct {
	ID          string   `json:"id"`
	Name        string   `json:"name"`
	Tags        []string `json:"tags"`
	Zone        string   `json:"zone"`
	Description string   `json:"description"`
}
type exitDef struct {
	From      string `json:"from"`
	Direction string `json:"direction"`
	To        string `json:"to"`
}

// GetRoom 依 id 查詢房間；若 store 已啟用則從 JSON 背板讀取，否則從 DB。
func GetRoom(db *sql.DB, id string) (*model.Room, error) {
	if store.Default != nil {
		return store.Default.GetRoom(id)
	}
	var r Room
	var tagsJSON string
	err := db.QueryRow("SELECT id, name, description, tags, zone FROM rooms WHERE id = ?", id).Scan(&r.ID, &r.Name, &r.Description, &tagsJSON, &r.Zone)
	if err == sql.ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	_ = json.Unmarshal([]byte(tagsJSON), &r.Tags)
	return &r, nil
}

// TerrainFromRoom 依房間 tags／zone 回傳戰鬥地形標籤（lush／chaos／silent／grip），供 combat.CombatOpt 使用；無則空字串。
func TerrainFromRoom(db *sql.DB, roomID string) string {
	room, err := GetRoom(db, roomID)
	if err != nil || room == nil {
		return ""
	}
	for _, t := range room.Tags {
		switch strings.ToLower(strings.TrimSpace(t)) {
		case "lush", "chaos", "silent", "grip":
			return strings.ToLower(strings.TrimSpace(t))
		}
	}
	if strings.ToLower(strings.TrimSpace(room.Zone)) == "chaos" {
		return "chaos"
	}
	return ""
}

// GetRoomsByTag 回傳所有帶有指定 tag 的房間 ID；store 啟用時從 JSON 背板讀取。
func GetRoomsByTag(database *sql.DB, tag string) ([]string, error) {
	if store.Default != nil {
		return store.Default.GetRoomsByTag(tag), nil
	}
	rows, err := database.Query("SELECT id, tags FROM rooms")
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var result []string
	for rows.Next() {
		var id, tagsJSON string
		if err := rows.Scan(&id, &tagsJSON); err != nil {
			return nil, err
		}
		var tags []string
		_ = json.Unmarshal([]byte(tagsJSON), &tags)
		for _, t := range tags {
			if t == tag {
				result = append(result, id)
				break
			}
		}
	}
	return result, rows.Err()
}

// GetRoomsByZone 回傳指定 zone 中的所有房間 ID；store 啟用時從 JSON 背板讀取。
func GetRoomsByZone(database *sql.DB, zone string) ([]string, error) {
	if store.Default != nil {
		return store.Default.GetRoomsByZone(zone), nil
	}
	rows, err := database.Query("SELECT id FROM rooms WHERE zone = ?", zone)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var result []string
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return nil, err
		}
		result = append(result, id)
	}
	return result, rows.Err()
}

// GetExitsForRoom 回傳某房間的所有出口（含目標房間名稱）；store 啟用時從 JSON 背板讀取。
func GetExitsForRoom(db *sql.DB, fromRoomID string) ([]Exit, error) {
	if store.Default != nil {
		return store.Default.GetExitsForRoom(fromRoomID)
	}
	rows, err := db.Query(
		`SELECT e.direction, e.to_room_id, r.name
		 FROM exits e JOIN rooms r ON r.id = e.to_room_id
		 WHERE e.from_room_id = ? ORDER BY e.direction`,
		fromRoomID,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var list []Exit
	for rows.Next() {
		var ex Exit
		if err := rows.Scan(&ex.Direction, &ex.ToRoomID, &ex.ToRoomName); err != nil {
			return nil, err
		}
		list = append(list, ex)
	}
	return list, rows.Err()
}

// GetEntitiesInRoom 回傳指定房間內的所有實體；store 啟用時從 store 取房內 entity_id 再自 DB 查 entities。
// NPC 的 DisplayTitle 依討論 001 改為自指派推導，無指派時 fallback 為 entities.display_title。
func GetEntitiesInRoom(db *sql.DB, roomID string) ([]*entity.Character, error) {
	var rows *sql.Rows
	var err error
	if store.Default != nil {
		ids := store.Default.EntityIDsInRoom(roomID)
		if len(ids) == 0 {
			return nil, nil
		}
		var list []*entity.Character
		for _, id := range ids {
			se := store.Default.GetEntity(id)
			if se == nil || se.Vit <= 0 {
				continue
			}
			title := ""
			if se.Kind == "npc" {
				title = GetNPCTitle(db, id)
			}
			list = append(list, storeEntityToCharacter(se, title))
		}
		return list, nil
	}
	rows, err = db.Query(
		`SELECT c.id, c.kind, c.display_char, c.x, c.y, c.move_state, c.target_x, c.target_y, c.walk_or_run,
		 c.move_started_at, c.vit, c.qi, c.dex, c.magnesium, c.last_observed_at, c.created_at, c.gender, c.soul_seed,
		 c.display_title
		 FROM entity_room er JOIN entities c ON c.id = er.entity_id
		 WHERE er.room_id = ? AND c.vit > 0`,
		roomID,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var list []*entity.Character
	list, err = scanCharacterList(rows)
	if err != nil {
		return nil, err
	}
	for _, c := range list {
		if c.Kind == "npc" {
			c.DisplayTitle = GetNPCTitle(db, c.ID)
		}
	}
	return list, nil
}

// GetEntityRoom 回傳實體當前房間 id；store 啟用時從 JSON 背板讀取並可即時覆寫。
func GetEntityRoom(db *sql.DB, entityID string) (string, error) {
	if store.Default != nil {
		return store.Default.GetEntityRoom(entityID)
	}
	var roomID string
	err := db.QueryRow("SELECT room_id FROM entity_room WHERE entity_id = ?", entityID).Scan(&roomID)
	if err == sql.ErrNoRows {
		return "", nil
	}
	if err != nil {
		return "", err
	}
	return roomID, nil
}

// SetEntityRoom 將實體設為在指定房間；store 啟用時寫入記憶體並原子寫回 runtime/entity_rooms.json。
func SetEntityRoom(db *sql.DB, entityID, roomID string) error {
	if store.Default != nil {
		return store.Default.SetEntityRoom(entityID, roomID)
	}
	_, err := db.Exec("INSERT OR REPLACE INTO entity_room (entity_id, room_id) VALUES (?, ?)", entityID, roomID)
	return err
}

// GetNPCIDsWithRoom 回傳所有「有房間」且 Vit>0（存活）的 NPC 實體 ID（供主迴圈註冊腦驅動 Traveler 用）。死亡（Vit≤0）NPC 不參與。
func GetNPCIDsWithRoom(db *sql.DB) ([]string, error) {
	if store.Default != nil {
		return store.Default.GetNPCIDsWithRoom(), nil
	}
	rows, err := db.Query(
		`SELECT er.entity_id FROM entity_room er JOIN entities e ON e.id = er.entity_id WHERE e.kind = 'npc' AND e.vit > 0`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []string
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return nil, err
		}
		out = append(out, id)
	}
	return out, rows.Err()
}

// GetPlayerIDsWithRoom 回傳所有有房間的玩家實體 ID；池總量計入玩家時使用。
func GetPlayerIDsWithRoom(db *sql.DB) ([]string, error) {
	if store.Default != nil {
		return store.Default.GetPlayerIDsWithRoom(), nil
	}
	rows, err := db.Query(
		`SELECT er.entity_id FROM entity_room er JOIN entities e ON e.id = er.entity_id WHERE e.kind = 'player'`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []string
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return nil, err
		}
		out = append(out, id)
	}
	return out, rows.Err()
}

// GetRoomName 查詢房間名稱；store 啟用時從 JSON 背板讀取。
func GetRoomName(database *sql.DB, roomID string) (string, error) {
	if store.Default != nil {
		return store.Default.GetRoomName(roomID), nil
	}
	var name string
	err := database.QueryRow("SELECT name FROM rooms WHERE id = ?", roomID).Scan(&name)
	if err == sql.ErrNoRows {
		return "", nil
	}
	return name, err
}

// SpawnRoomName 創生預設格之唯一名稱；讀到這個名稱即為玩家創生預設房間。
const SpawnRoomName = "界壁"

// GetRoomIDByName 依房間名稱回傳第一個符合的房間 id；若無則空字串。
func GetRoomIDByName(database *sql.DB, name string) (string, error) {
	if store.Default != nil {
		return store.Default.GetRoomIDByName(name), nil
	}
	var id string
	err := database.QueryRow("SELECT id FROM rooms WHERE name = ? LIMIT 1", name).Scan(&id)
	if err == sql.ErrNoRows {
		return "", nil
	}
	return id, err
}

// GetSpawnRoomID 回傳創生預設房間 id（名稱為「界壁」的房間）；若無則回傳 "lobby" 以相容舊資料。
func GetSpawnRoomID(database *sql.DB) string {
	id, _ := GetRoomIDByName(database, SpawnRoomName)
	if id == "" {
		return "lobby"
	}
	return id
}

// RoomWithExits 房間與其出口列表，供管理 API 使用。
type RoomWithExits struct {
	Room  Room  `json:"room"`
	Exits []Exit `json:"exits"`
}

// ListAllRooms 回傳所有房間及各自出口；store 啟用時從 JSON 背板讀取。
func ListAllRooms(db *sql.DB) ([]RoomWithExits, error) {
	if store.Default != nil {
		var list []RoomWithExits
		for _, id := range store.Default.RoomIDs() {
			r, _ := store.Default.GetRoom(id)
			if r == nil {
				continue
			}
			exits, _ := store.Default.GetExitsForRoom(id)
			list = append(list, RoomWithExits{Room: *r, Exits: exits})
		}
		return list, nil
	}
	rows, err := db.Query("SELECT id, name, description, tags, zone FROM rooms ORDER BY id")
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var list []RoomWithExits
	for rows.Next() {
		var r Room
		var tagsJSON string
		if err := rows.Scan(&r.ID, &r.Name, &r.Description, &tagsJSON, &r.Zone); err != nil {
			return nil, err
		}
		_ = json.Unmarshal([]byte(tagsJSON), &r.Tags)
		exits, _ := GetExitsForRoom(db, r.ID)
		list = append(list, RoomWithExits{Room: r, Exits: exits})
	}
	return list, rows.Err()
}

// CreateRoom 新增房間。
func CreateRoom(db *sql.DB, id, name, description string) error {
	_, err := db.Exec("INSERT INTO rooms (id, name, description) VALUES (?, ?, ?)", id, name, description)
	return err
}

// ErrRoomNotFound 表示找不到要更新的房間（UPDATE 影響 0 列）。
var ErrRoomNotFound = errors.New("room not found")

// UpdateRoom 更新房間名稱與描述；若 id 不存在則回傳 ErrRoomNotFound。
func UpdateRoom(db *sql.DB, id, name, description string) error {
	result, err := db.Exec("UPDATE rooms SET name = ?, description = ? WHERE id = ?", name, description, id)
	if err != nil {
		return err
	}
	n, _ := result.RowsAffected()
	if n == 0 {
		return ErrRoomNotFound
	}
	return nil
}

// DeleteRoom 刪除房間：先刪出口、將房內實體移到創生格（界壁）、再刪房間。
func DeleteRoom(db *sql.DB, id string) error {
	if _, err := db.Exec("DELETE FROM exits WHERE from_room_id = ? OR to_room_id = ?", id, id); err != nil {
		return err
	}
	spawnID := GetSpawnRoomID(db)
	if _, err := db.Exec("UPDATE entity_room SET room_id = ? WHERE room_id = ?", spawnID, id); err != nil {
		return err
	}
	_, err := db.Exec("DELETE FROM rooms WHERE id = ?", id)
	return err
}

// AddExit 新增一筆出口。
func AddExit(db *sql.DB, fromRoomID, direction, toRoomID string) error {
	_, err := db.Exec("INSERT INTO exits (from_room_id, direction, to_room_id) VALUES (?, ?, ?)", fromRoomID, direction, toRoomID)
	return err
}

// RemoveExit 刪除一筆出口。
func RemoveExit(db *sql.DB, fromRoomID, direction string) error {
	_, err := db.Exec("DELETE FROM exits WHERE from_room_id = ? AND direction = ?", fromRoomID, direction)
	return err
}

// SyncRoomsFromFile 讀取 data/rooms.json，將房間與出口同步進 DB。
// 檔案裡有的房間：不存在就新增，已存在就更新名稱與描述。
// 檔案裡有的出口：不存在就新增。
// DB 裡有但檔案沒有的房間/出口不會被刪（admin.html 手動加的不受影響）。
func SyncRoomsFromFile(database *sql.DB, path string) error {
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			log.Printf("rooms: %s not found, skip sync", path)
			return nil
		}
		return err
	}
	var f roomsFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}

	for _, r := range f.Rooms {
		tagsJSON, _ := json.Marshal(r.Tags)
		if r.Tags == nil {
			tagsJSON = []byte("[]")
		}
		var exists int
		_ = database.QueryRow("SELECT COUNT(*) FROM rooms WHERE id = ?", r.ID).Scan(&exists)
		if exists > 0 {
			_, _ = database.Exec("UPDATE rooms SET name = ?, description = ?, tags = ?, zone = ? WHERE id = ?", r.Name, r.Description, string(tagsJSON), r.Zone, r.ID)
		} else {
			if _, err := database.Exec("INSERT INTO rooms (id, name, description, tags, zone) VALUES (?, ?, ?, ?, ?)", r.ID, r.Name, r.Description, string(tagsJSON), r.Zone); err != nil {
				return err
			}
			log.Printf("rooms: created %s (%s)", r.ID, r.Name)
		}
	}

	for _, e := range f.Exits {
		var exists int
		_ = database.QueryRow("SELECT COUNT(*) FROM exits WHERE from_room_id = ? AND direction = ?", e.From, e.Direction).Scan(&exists)
		if exists == 0 {
			if _, err := database.Exec("INSERT INTO exits (from_room_id, direction, to_room_id) VALUES (?, ?, ?)", e.From, e.Direction, e.To); err != nil {
				return err
			}
			log.Printf("rooms: exit %s -[%s]-> %s", e.From, e.Direction, e.To)
		}
	}

	// 沒有房間的實體放進創生格（名稱為界壁的房間）
	spawnID := GetSpawnRoomID(database)
	rows, err := database.Query(
		"SELECT id FROM entities WHERE id NOT IN (SELECT entity_id FROM entity_room)")
	if err != nil {
		return err
	}
	defer rows.Close()
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return err
		}
		_ = SetEntityRoom(database, id, spawnID)
	}
	return rows.Err()
}

// SeedRooms 向下相容：讀 data/rooms.json 同步房間。
func SeedRooms(db *sql.DB) error {
	return SyncRoomsFromFile(db, "data/rooms.json")
}

// scanCharacterList 共用：從 entities 的 SELECT 結果（含 display_title）掃成 []*entity.Character。
func scanCharacterList(rows *sql.Rows) ([]*entity.Character, error) {
	var list []*entity.Character
	for rows.Next() {
		var c entity.Character
		var targetX, targetY sql.NullInt64
		var moveStartedAt, lastObservedAt sql.NullInt64
		var walkOrRun, gender sql.NullString
		var soulSeed sql.NullInt64
		var displayTitle sql.NullString
		if err := rows.Scan(
			&c.ID, &c.Kind, &c.DisplayChar, &c.X, &c.Y, &c.MoveState,
			&targetX, &targetY, &walkOrRun, &moveStartedAt,
			&c.Vit, &c.Qi, &c.Dex, &c.Magnesium, &lastObservedAt, &c.CreatedAt, &gender, &soulSeed,
			&displayTitle,
		); err != nil {
			return nil, err
		}
		if targetX.Valid {
			x := int(targetX.Int64)
			c.TargetX = &x
		}
		if targetY.Valid {
			y := int(targetY.Int64)
			c.TargetY = &y
		}
		if walkOrRun.Valid {
			c.WalkOrRun = walkOrRun.String
		}
		if moveStartedAt.Valid {
			t := moveStartedAt.Int64
			c.MoveStartedAt = &t
		}
		if lastObservedAt.Valid {
			t := lastObservedAt.Int64
			c.LastObservedAt = &t
		}
		if gender.Valid {
			c.Gender = gender.String
		}
		if soulSeed.Valid {
			c.SoulSeed = &soulSeed.Int64
		}
		if displayTitle.Valid {
			c.DisplayTitle = displayTitle.String
		}
		list = append(list, &c)
	}
	return list, rows.Err()
}
