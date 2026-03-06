// Package db 負責 entities 表之讀寫，供觀測坍縮與狀態回推使用。本檔為單筆查詢與 last_observed_at 更新。
package db

import (
	"database/sql"
	"encoding/binary"
	"math"
	"math/rand"
	"time"

	"singularity_world/entity"

	cryptorand "crypto/rand"
)

// 三軸區間與映射常數（與人物屬性彙整 §二、cmd/soulseed_demo 一致）
const (
	ampMin, ampMax     = 0.1, 3.0
	freqMin, freqMax   = 0.5, 2.0
	phaseMin, phaseMax = -1.0, 1.0
	baseStat            = 10.0
	kAmp, kFreq, kPhase = 0.2, 0.2, 0.2
	minStat, maxStat    = 1, 30 // 基礎體敏氣 clamp 範圍
)

// GenerateSoulSeed 創角時產生僅屬於該角色的 int64 種子（加密亂數）。規格：人物屬性彙整 §2.0。
func GenerateSoulSeed() (int64, error) {
	var b [8]byte
	if _, err := cryptorand.Read(b[:]); err != nil {
		return 0, err
	}
	return int64(binary.BigEndian.Uint64(b[:])), nil
}

// ResourceMaxes 四項資源的當前值與最大值（氣血、內力、精神、體力）。第一版當前＝最大值。
type ResourceMaxes struct {
	HpCur, HpMax           float64 // 氣血
	InnerCur, InnerMax     float64 // 內力
	SpiritCur, SpiritMax   float64 // 精神
	StaminaCur, StaminaMax float64 // 體力
}

// ComputeResourceMaxes 由體質、氣脈、靈敏計算四項資源最大值（無條件進位至整數）；第一版當前值＝最大值（無耗損）。
func ComputeResourceMaxes(vit, qi, dex int) ResourceMaxes {
	v, q, d := float64(vit), float64(qi), float64(dex)
	hpMax := math.Ceil(((0.7*v)+(0.3*q))*((0.05*d)+1)*v)
	innerMax := math.Ceil(((0.7*q)+(0.3*v))*((0.05*d)+1)*q)
	spiritMax := math.Ceil(((0.6*d)+(0.4*q))*((0.05*v)+1)*d)
	staminaMax := math.Ceil(((0.5*v)+(0.4*q)+(0.3*d)) * ((v + q + d) / 3))
	return ResourceMaxes{
		HpCur: hpMax, HpMax: hpMax,
		InnerCur: innerMax, InnerMax: innerMax,
		SpiritCur: spiritMax, SpiritMax: spiritMax,
		StaminaCur: staminaMax, StaminaMax: staminaMax,
	}
}

// GenerateOriginSentence 由三軸（能階、時脈、相位）產出一句話語感，供狀態分頁【本源】顯示；三軸不傳前端。規格：狀態與星盤分頁規格 §五.1。
func GenerateOriginSentence(amp, freq, phase float64) string {
	ampWord := "綿長"
	if amp < 1.0 {
		ampWord = "幽微"
	} else if amp > 2.0 {
		ampWord = "霸道"
	}
	freqWord := "洞察"
	if freq < 1.0 {
		freqWord = "渾厚"
	} else if freq > 1.6 {
		freqWord = "敏銳"
	}
	phaseWord := "順流"
	if phase < -0.3 {
		phaseWord = "混沌"
	} else if phase > 0.3 {
		phaseWord = "秩序"
	}
	return "你的神識" + ampWord + "且" + freqWord + "，隱隱透著一股" + phaseWord + "的逆流。"
}

// ExpandSoulSeedToOriginSentence 由 soul_seed 前 3 次 RNG 展開三軸，再呼叫 GenerateOriginSentence 產出一句話；僅後端使用，不傳三軸數字。規格：狀態與星盤分頁規格 §五.1。
func ExpandSoulSeedToOriginSentence(seed int64) string {
	rng := rand.New(rand.NewSource(seed))
	u1, u2, u3 := rng.Float64(), rng.Float64(), rng.Float64()
	amp := ampMin + u1*(ampMax-ampMin)
	freq := freqMin + u2*(freqMax-freqMin)
	phase := phaseMin + u3*(phaseMax-phaseMin)
	return GenerateOriginSentence(amp, freq, phase)
}

// ExpandSoulSeedToBaseStats 由 soul_seed 前 3 次 RNG 展開三軸，再映射為基礎體質／氣脈／靈敏（取整並 clamp）。規格：人物屬性彙整 §二。
func ExpandSoulSeedToBaseStats(seed int64) (vit, qi, dex int) {
	rng := rand.New(rand.NewSource(seed))
	u1, u2, u3 := rng.Float64(), rng.Float64(), rng.Float64()
	amp := ampMin + u1*(ampMax-ampMin)
	freq := freqMin + u2*(freqMax-freqMin)
	phase := phaseMin + u3*(phaseMax-phaseMin)
	v := int(math.Round(baseStat * (1 + kAmp*(amp-1))))
	q := int(math.Round(baseStat * (1 + kFreq*(freq-1))))
	d := int(math.Round(baseStat * (1 + kPhase*phase)))
	clamp := func(x int) int {
		if x < minStat {
			return minStat
		}
		if x > maxStat {
			return maxStat
		}
		return x
	}
	return clamp(v), clamp(q), clamp(d)
}

// InsertEntity 新增一筆玩家實體（創角用）；id 不可重複。gender 為 "M" 或 "F"，空則預設 "M"。會寫入 soul_seed，並由 seed 展開三軸映射為 vit/qi/dex 寫入。
func InsertEntity(db *sql.DB, id, displayChar, gender string) error {
	if displayChar == "" {
		displayChar = "我"
	}
	if gender != "M" && gender != "F" {
		gender = "M"
	}
	seed, err := GenerateSoulSeed()
	if err != nil {
		return err
	}
	vit, qi, dex := ExpandSoulSeedToBaseStats(seed)
	now := time.Now().Unix()
	equip := StarterEquipment(gender)
	_, err = db.Exec(
		`INSERT INTO entities (id, kind, display_char, x, y, move_state, vit, qi, dex, magnesium, created_at, gender, soul_seed, equipment_slots)
		 VALUES (?, 'player', ?, 0, 0, 'idle', ?, ?, ?, 0, ?, ?, ?, ?)`,
		id, displayChar, vit, qi, dex, now, gender, seed, equip,
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
	var walkOrRun, gender, displayTitle, activatedNodes, equipSlots sql.NullString

	var soulSeed sql.NullInt64
	var inventory sql.NullString
	err := db.QueryRow(
		`SELECT id, kind, display_char, x, y, move_state, target_x, target_y, walk_or_run,
		 move_started_at, vit, qi, dex, magnesium, last_observed_at, created_at, gender, soul_seed,
		 display_title, activated_nodes, equipment_slots, inventory
		 FROM entities WHERE id = ?`,
		id,
	).Scan(
		&c.ID, &c.Kind, &c.DisplayChar, &c.X, &c.Y, &c.MoveState,
		&targetX, &targetY, &walkOrRun, &moveStartedAt,
		&c.Vit, &c.Qi, &c.Dex, &c.Magnesium, &lastObservedAt, &c.CreatedAt, &gender, &soulSeed,
		&displayTitle, &activatedNodes, &equipSlots, &inventory,
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
	if soulSeed.Valid {
		c.SoulSeed = &soulSeed.Int64
	}
	if displayTitle.Valid && displayTitle.String != "" {
		c.DisplayTitle = displayTitle.String
	}
	if activatedNodes.Valid && activatedNodes.String != "" {
		c.ActivatedNodes = activatedNodes.String
	} else {
		c.ActivatedNodes = `["N000"]`
	}
	if equipSlots.Valid && equipSlots.String != "" {
		c.EquipmentSlots = equipSlots.String
	}
	if inventory.Valid && inventory.String != "" {
		c.Inventory = inventory.String
	} else {
		c.Inventory = "[]"
	}
	if c.Kind == "npc" {
		c.DisplayTitle = GetNPCTitle(db, id)
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
		 move_started_at, vit, qi, dex, magnesium, last_observed_at, created_at, gender, soul_seed, equipment_slots
		 FROM entities WHERE move_state = 'moving' AND target_x IS NOT NULL AND target_y IS NOT NULL`,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanCharacters(rows)
}

// scanCharacters 共用：從 entities 的 SELECT（含 soul_seed）掃成 []*entity.Character。
func scanCharacters(rows *sql.Rows) ([]*entity.Character, error) {
	var list []*entity.Character
	for rows.Next() {
		var c entity.Character
		var targetX, targetY sql.NullInt64
		var moveStartedAt, lastObservedAt sql.NullInt64
		var walkOrRun, gender, equipSlots sql.NullString
		var soulSeed sql.NullInt64
		if err := rows.Scan(
			&c.ID, &c.Kind, &c.DisplayChar, &c.X, &c.Y, &c.MoveState,
			&targetX, &targetY, &walkOrRun, &moveStartedAt,
			&c.Vit, &c.Qi, &c.Dex, &c.Magnesium, &lastObservedAt, &c.CreatedAt, &gender, &soulSeed,
			&equipSlots,
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
		if equipSlots.Valid {
			c.EquipmentSlots = equipSlots.String
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
		 move_started_at, vit, qi, dex, magnesium, last_observed_at, created_at, gender, soul_seed, equipment_slots
		 FROM entities WHERE kind = ? AND x >= ? AND x <= ? AND y >= ? AND y <= ?`
		args = []interface{}{kind, xMin, xMax, yMin, yMax}
	} else {
		query = `SELECT id, kind, display_char, x, y, move_state, target_x, target_y, walk_or_run,
		 move_started_at, vit, qi, dex, magnesium, last_observed_at, created_at, gender, soul_seed, equipment_slots
		 FROM entities WHERE x >= ? AND x <= ? AND y >= ? AND y <= ?`
		args = []interface{}{xMin, xMax, yMin, yMax}
	}
	rows, err := db.Query(query, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanCharacters(rows)
}

// DeleteAllEntities 刪除所有實體（entity_auth、entity_room、entities、event_log 內與實體相關可選）。用於清空角色重測創角等。
func DeleteAllEntities(db *sql.DB) error {
	if _, err := db.Exec("DELETE FROM entity_auth"); err != nil {
		return err
	}
	if _, err := db.Exec("DELETE FROM entity_room"); err != nil {
		return err
	}
	if _, err := db.Exec("DELETE FROM event_log"); err != nil {
		return err
	}
	if _, err := db.Exec("DELETE FROM entities"); err != nil {
		return err
	}
	return nil
}
