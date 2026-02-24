// Package db 房間與出口查詢、實體所在房間，供傳統 MUD 節點連接節點機制。
package db

import (
	"database/sql"

	"singularity_world/entity"
)

// Room 單一房間節點。
type Room struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Description string `json:"description"`
}

// Exit 單一出口：方向 → 目標房間。
type Exit struct {
	Direction  string `json:"direction"`
	ToRoomID   string `json:"to_room_id"`
	ToRoomName string `json:"to_room_name"`
}

// GetRoom 依 id 查詢房間；若無則回傳 nil, nil。
func GetRoom(db *sql.DB, id string) (*Room, error) {
	var r Room
	err := db.QueryRow("SELECT id, name, description FROM rooms WHERE id = ?", id).Scan(&r.ID, &r.Name, &r.Description)
	if err == sql.ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	return &r, nil
}

// GetExitsForRoom 回傳某房間的所有出口（含目標房間名稱）。
func GetExitsForRoom(db *sql.DB, fromRoomID string) ([]Exit, error) {
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

// GetEntitiesInRoom 回傳指定房間內的所有實體（依 entity_room 與 entities  join）。
func GetEntitiesInRoom(db *sql.DB, roomID string) ([]*entity.Character, error) {
	rows, err := db.Query(
		`SELECT c.id, c.kind, c.display_char, c.x, c.y, c.move_state, c.target_x, c.target_y, c.walk_or_run,
		 c.move_started_at, c.vit, c.qi, c.dex, c.magnesium, c.last_observed_at, c.created_at
		 FROM entity_room er JOIN entities c ON c.id = er.entity_id
		 WHERE er.room_id = ?`,
		roomID,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanCharacterList(rows)
}

// GetEntityRoom 回傳實體當前房間 id；若無則回傳空字串, nil。
func GetEntityRoom(db *sql.DB, entityID string) (string, error) {
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

// SetEntityRoom 將實體設為在指定房間（INSERT OR REPLACE）。
func SetEntityRoom(db *sql.DB, entityID, roomID string) error {
	_, err := db.Exec("INSERT OR REPLACE INTO entity_room (entity_id, room_id) VALUES (?, ?)", entityID, roomID)
	return err
}

// RoomWithExits 房間與其出口列表，供管理 API 使用。
type RoomWithExits struct {
	Room  Room  `json:"room"`
	Exits []Exit `json:"exits"`
}

// ListAllRooms 回傳所有房間及各自出口。
func ListAllRooms(db *sql.DB) ([]RoomWithExits, error) {
	rows, err := db.Query("SELECT id, name, description FROM rooms ORDER BY id")
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var list []RoomWithExits
	for rows.Next() {
		var r Room
		if err := rows.Scan(&r.ID, &r.Name, &r.Description); err != nil {
			return nil, err
		}
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

// UpdateRoom 更新房間名稱與描述。
func UpdateRoom(db *sql.DB, id, name, description string) error {
	_, err := db.Exec("UPDATE rooms SET name = ?, description = ? WHERE id = ?", name, description, id)
	return err
}

// DeleteRoom 刪除房間：先刪出口、將房內實體移到大廳、再刪房間。
func DeleteRoom(db *sql.DB, id string) error {
	if _, err := db.Exec("DELETE FROM exits WHERE from_room_id = ? OR to_room_id = ?", id, id); err != nil {
		return err
	}
	if _, err := db.Exec("UPDATE entity_room SET room_id = 'lobby' WHERE room_id = ?", id); err != nil {
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

// SeedRooms 若尚無房間則建立預設房間與出口，並將所有尚無 entity_room 的實體放入預設房間。
func SeedRooms(db *sql.DB) error {
	var n int
	if err := db.QueryRow("SELECT COUNT(*) FROM rooms").Scan(&n); err != nil || n > 0 {
		return err
	}
	// 預設房間（描述不寫出口，由移動欄顯示）
	rooms := []struct {
		id, name, desc string
	}{
		{"lobby", "大廳", "寬敞的大廳，幾盞燈映著牆上的地圖。"},
		{"east_street", "東街", "一條東西向的街道，人來人往。"},
		{"west_alley", "西巷", "窄巷幽靜，盡頭有扇小門。"},
		{"south_plaza", "南廣場", "開闊的廣場，中央有噴泉。"},
	}
	for _, r := range rooms {
		if _, err := db.Exec("INSERT INTO rooms (id, name, description) VALUES (?, ?, ?)", r.id, r.name, r.desc); err != nil {
			return err
		}
	}
	// 出口：節點連接節點
	exits := []struct{ from, dir, to string }{
		{"lobby", "東", "east_street"},
		{"lobby", "西", "west_alley"},
		{"east_street", "西", "lobby"},
		{"west_alley", "東", "lobby"},
		{"west_alley", "南", "south_plaza"},
		{"south_plaza", "北", "west_alley"},
	}
	for _, e := range exits {
		if _, err := db.Exec("INSERT INTO exits (from_room_id, direction, to_room_id) VALUES (?, ?, ?)", e.from, e.dir, e.to); err != nil {
			return err
		}
	}
	// 所有實體放入大廳
	rows, err := db.Query("SELECT id FROM entities")
	if err != nil {
		return err
	}
	defer rows.Close()
	defaultRoom := "lobby"
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return err
		}
		if err := SetEntityRoom(db, id, defaultRoom); err != nil {
			return err
		}
	}
	return rows.Err()
}

// scanCharacterList 共用：從 entities 的 SELECT 結果掃成 []*entity.Character。
func scanCharacterList(rows *sql.Rows) ([]*entity.Character, error) {
	var list []*entity.Character
	for rows.Next() {
		var c entity.Character
		var targetX, targetY sql.NullInt64
		var moveStartedAt, lastObservedAt sql.NullInt64
		var walkOrRun sql.NullString
		if err := rows.Scan(
			&c.ID, &c.Kind, &c.DisplayChar, &c.X, &c.Y, &c.MoveState,
			&targetX, &targetY, &walkOrRun, &moveStartedAt,
			&c.Vit, &c.Qi, &c.Dex, &c.Magnesium, &lastObservedAt, &c.CreatedAt,
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
		list = append(list, &c)
	}
	return list, rows.Err()
}
