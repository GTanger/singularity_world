// Package db 負責 entities 表之讀寫，供觀測坍縮與狀態回推使用。本檔為單筆查詢與 last_observed_at 更新。
package db

import (
	"database/sql"
	"time"

	"singularity_world/entity"
)

// InsertEntity 新增一筆玩家實體（創角用）；id 不可重複。gender 為 "M" 或 "F"，空則預設 "M"。
func InsertEntity(db *sql.DB, id, displayChar, gender string) error {
	if displayChar == "" {
		displayChar = "我"
	}
	if gender != "M" && gender != "F" {
		gender = "M"
	}
	now := time.Now().Unix()
	_, err := db.Exec(
		`INSERT INTO entities (id, kind, display_char, x, y, move_state, vit, qi, dex, magnesium, created_at, gender)
		 VALUES (?, 'player', ?, 0, 0, 'idle', 10, 10, 10, 0, ?, ?)`,
		id, displayChar, now, gender,
	)
	return err
}

// GetEntity 依 id 查詢 entities 表，回傳一筆角色；若無則回傳 nil, nil。
// 參數：db 為資料庫連線，id 為實體 id。
// 回傳：*entity.Character 與 error；若查無則 (nil, nil)。
func GetEntity(db *sql.DB, id string) (*entity.Character, error) {
	var c entity.Character
	var targetX, targetY sql.NullInt64
	var moveStartedAt, lastObservedAt sql.NullInt64
	var walkOrRun, gender sql.NullString

	err := db.QueryRow(
		`SELECT id, kind, display_char, x, y, move_state, target_x, target_y, walk_or_run,
		 move_started_at, vit, qi, dex, magnesium, last_observed_at, created_at, gender
		 FROM entities WHERE id = ?`,
		id,
	).Scan(
		&c.ID, &c.Kind, &c.DisplayChar, &c.X, &c.Y, &c.MoveState,
		&targetX, &targetY, &walkOrRun, &moveStartedAt,
		&c.Vit, &c.Qi, &c.Dex, &c.Magnesium, &lastObservedAt, &c.CreatedAt, &gender,
	)
	if err == sql.ErrNoRows {
		return nil, nil
	}
	if err != nil {
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
	return &c, nil
}

// UpdateLastObserved 將指定實體的 last_observed_at 更新為 at；用於觀測觸發時標記。
// 參數：db 為資料庫連線，id 為實體 id，at 為時間戳。
// 回傳：error。副作用：UPDATE entities 一筆。
func UpdateLastObserved(db *sql.DB, id string, at int64) error {
	_, err := db.Exec("UPDATE entities SET last_observed_at = ? WHERE id = ?", at, id)
	return err
}

// UpdatePosition 將指定實體位置更新為 (x, y)，並設為 idle（清除移動目標）。用於 1.3.1 點擊移動與抵達時。
func UpdatePosition(db *sql.DB, id string, x, y int) error {
	_, err := db.Exec(
		"UPDATE entities SET x = ?, y = ?, move_state = 'idle', target_x = NULL, target_y = NULL, move_started_at = NULL WHERE id = ?",
		x, y, id,
	)
	return err
}

// SetMoveTarget 設定移動目標，move_state 設為 moving；供 1.3.3 單擊走雙擊跑、tick 逐步推進。
func SetMoveTarget(db *sql.DB, id string, targetX, targetY int, walkOrRun string, startedAt int64) error {
	if walkOrRun == "" {
		walkOrRun = "walk"
	}
	_, err := db.Exec(
		"UPDATE entities SET target_x = ?, target_y = ?, move_state = 'moving', walk_or_run = ?, move_started_at = ? WHERE id = ?",
		targetX, targetY, walkOrRun, startedAt, id,
	)
	return err
}

// UpdatePositionOnly 僅更新實體座標（用於移動中每 tick 步進），不清除 target。
func UpdatePositionOnly(db *sql.DB, id string, x, y int) error {
	_, err := db.Exec("UPDATE entities SET x = ?, y = ? WHERE id = ?", x, y, id)
	return err
}

// GetMovingEntities 回傳所有 move_state = 'moving' 的實體，供每 tick 推進位置用。
func GetMovingEntities(db *sql.DB) ([]*entity.Character, error) {
	rows, err := db.Query(
		`SELECT id, kind, display_char, x, y, move_state, target_x, target_y, walk_or_run,
		 move_started_at, vit, qi, dex, magnesium, last_observed_at, created_at, gender
		 FROM entities WHERE move_state = 'moving' AND target_x IS NOT NULL AND target_y IS NOT NULL`,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var list []*entity.Character
	for rows.Next() {
		var c entity.Character
		var targetX, targetY sql.NullInt64
		var moveStartedAt, lastObservedAt sql.NullInt64
		var walkOrRun, gender sql.NullString
		if err := rows.Scan(
			&c.ID, &c.Kind, &c.DisplayChar, &c.X, &c.Y, &c.MoveState,
			&targetX, &targetY, &walkOrRun, &moveStartedAt,
			&c.Vit, &c.Qi, &c.Dex, &c.Magnesium, &lastObservedAt, &c.CreatedAt, &gender,
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
		list = append(list, &c)
	}
	return list, rows.Err()
}

// GetEntitiesInBox 查詢座標落在 [xMin,xMax]×[yMin,yMax] 內的實體；kind 為 "npc" 僅 NPC，空字串為全部。
// 供視野內即時模擬只載入可能進入視野的 NPC 用。
func GetEntitiesInBox(db *sql.DB, xMin, xMax, yMin, yMax int, kind string) ([]*entity.Character, error) {
	var query string
	var args []interface{}
	if kind != "" {
		query = `SELECT id, kind, display_char, x, y, move_state, target_x, target_y, walk_or_run,
		 move_started_at, vit, qi, dex, magnesium, last_observed_at, created_at, gender
		 FROM entities WHERE kind = ? AND x >= ? AND x <= ? AND y >= ? AND y <= ?`
		args = []interface{}{kind, xMin, xMax, yMin, yMax}
	} else {
		query = `SELECT id, kind, display_char, x, y, move_state, target_x, target_y, walk_or_run,
		 move_started_at, vit, qi, dex, magnesium, last_observed_at, created_at, gender
		 FROM entities WHERE x >= ? AND x <= ? AND y >= ? AND y <= ?`
		args = []interface{}{xMin, xMax, yMin, yMax}
	}
	rows, err := db.Query(query, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var list []*entity.Character
	for rows.Next() {
		var c entity.Character
		var targetX, targetY sql.NullInt64
		var moveStartedAt, lastObservedAt sql.NullInt64
		var walkOrRun, gender sql.NullString
		err := rows.Scan(
			&c.ID, &c.Kind, &c.DisplayChar, &c.X, &c.Y, &c.MoveState,
			&targetX, &targetY, &walkOrRun, &moveStartedAt,
			&c.Vit, &c.Qi, &c.Dex, &c.Magnesium, &lastObservedAt, &c.CreatedAt, &gender,
		)
		if err != nil {
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
		list = append(list, &c)
	}
	return list, rows.Err()
}
