package db

import (
	"database/sql"
	"fmt"
	"math/rand"
	"strconv"
	"time"

	"singularity_world/store"
)

// InsertNPC 新增一筆 NPC 實體；store 啟用時寫入 store 並持久化 entities.json。
func InsertNPC(db *sql.DB, id, displayChar, gender, displayTitle string) error {
	if displayChar == "" {
		r := []rune(id)
		if len(r) > 0 {
			displayChar = string(r[0:1])
		} else {
			displayChar = "人"
		}
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
	if store.Default != nil {
		return store.Default.PutEntity(&store.Entity{
			ID: id, Kind: "npc", DisplayChar: displayChar,
			X: 0, Y: 0, MoveState: "idle",
			Vit: vit, Qi: qi, Dex: dex, Magnesium: 100,
			CreatedAt: now, Gender: gender, SoulSeed: &seed,
			DisplayTitle: displayTitle, EquipmentSlots: equip,
			Inventory: "[]", ActivatedNodes: `["N000"]`,
		})
	}
	_, err = db.Exec(
		`INSERT INTO entities (id, kind, display_char, x, y, move_state, vit, qi, dex, magnesium, created_at, gender, soul_seed, display_title, equipment_slots)
		 VALUES (?, 'npc', ?, 0, 0, 'idle', ?, ?, ?, 100, ?, ?, ?, ?, ?)`,
		id, displayChar, vit, qi, dex, now, gender, seed, displayTitle, equip,
	)
	return err
}

// InsertSchedule 設定 NPC 排班；store 啟用時寫入 store 並持久化 data/schedules.json。
func InsertSchedule(db *sql.DB, entityID, workRoom, restRoom string, shiftStart, shiftEnd int) error {
	if store.Default != nil {
		return store.Default.InsertSchedule(entityID, workRoom, restRoom, shiftStart, shiftEnd)
	}
	_, err := db.Exec(
		"INSERT OR REPLACE INTO npc_schedules (entity_id, work_room, rest_room, shift_start, shift_end) VALUES (?, ?, ?, ?, ?)",
		entityID, workRoom, restRoom, shiftStart, shiftEnd,
	)
	return err
}

// npcDef 描述一名預設 NPC 的全部資料。
type npcDef struct {
	id, displayChar, gender, title string
	workRoom, restRoom             string
	shiftStart, shiftEnd           int
}

// defaultNPCs 全體預設 NPC；先人後嘴用：至少 1 名在創生房，供 Talk（I3）驗收。
// work_room/rest_room 使用創生房 id（界壁 = start_boundary），與新玩家同房。
var defaultNPCs = []npcDef{
	{id: "試話", displayChar: "試", gender: "M", title: "", workRoom: "start_boundary", restRoom: "start_boundary", shiftStart: 0, shiftEnd: 24},
}

// SeedNPCs 逐一檢查預設 NPC，不存在才建立；並建立排班與指派（SQLite 模式用）。依 DB 查 entities 表。
func SeedNPCs(db *sql.DB) error {
	const venueLifeInn = "venue_life_inn"
	for _, npc := range defaultNPCs {
		var exists int
		if err := db.QueryRow("SELECT COUNT(*) FROM entities WHERE id = ?", npc.id).Scan(&exists); err != nil {
			return err
		}
		if exists > 0 {
			_ = InsertSchedule(db, npc.id, npc.workRoom, npc.restRoom, npc.shiftStart, npc.shiftEnd)
			_ = InsertAssignment(db, npc.id, npc.title, venueLifeInn, "")
			continue
		}
		if err := InsertNPC(db, npc.id, npc.displayChar, npc.gender, ""); err != nil {
			return err
		}
		if err := SetEntityRoom(db, npc.id, npc.workRoom); err != nil {
			return err
		}
		if err := InsertSchedule(db, npc.id, npc.workRoom, npc.restRoom, npc.shiftStart, npc.shiftEnd); err != nil {
			return err
		}
		if err := InsertAssignment(db, npc.id, npc.title, venueLifeInn, ""); err != nil {
			return err
		}
	}
	return nil
}

// SeedNPCsForStore 在 store 模式（無 DB）下確保預設 NPC 存在；不存在則建立並設房間／排班／指派。main 啟動時 store.Init 後呼叫。
func SeedNPCsForStore(db *sql.DB) error {
	if store.Default == nil {
		return nil
	}
	const venueLifeInn = "venue_life_inn"
	for _, npc := range defaultNPCs {
		if store.Default.GetEntity(npc.id) != nil {
			_ = InsertSchedule(db, npc.id, npc.workRoom, npc.restRoom, npc.shiftStart, npc.shiftEnd)
			_ = InsertAssignment(db, npc.id, npc.title, venueLifeInn, "")
			continue
		}
		if err := InsertNPC(db, npc.id, npc.displayChar, npc.gender, ""); err != nil {
			return err
		}
		if err := SetEntityRoom(db, npc.id, npc.workRoom); err != nil {
			return err
		}
		if err := InsertSchedule(db, npc.id, npc.workRoom, npc.restRoom, npc.shiftStart, npc.shiftEnd); err != nil {
			return err
		}
		if err := InsertAssignment(db, npc.id, npc.title, venueLifeInn, ""); err != nil {
			return err
		}
	}
	return nil
}

// GetNPCGenderCounts 回傳有房間的 NPC 中男(M)、女(F) 的數量，供生成時男女持平用。
func GetNPCGenderCounts(db *sql.DB) (male, female int) {
	if store.Default != nil {
		for _, eid := range store.Default.GetNPCIDsWithRoom() {
			e := store.Default.GetEntity(eid)
			if e == nil {
				continue
			}
			switch e.Gender {
			case "M":
				male++
			case "F":
				female++
			}
		}
		return male, female
	}
	rows, err := db.Query(
		`SELECT e.gender FROM entity_room er JOIN entities e ON e.id = er.entity_id WHERE e.kind = 'npc'`)
	if err != nil {
		return 0, 0
	}
	defer rows.Close()
	for rows.Next() {
		var g sql.NullString
		if err := rows.Scan(&g); err != nil {
			continue
		}
		if g.Valid {
			switch g.String {
			case "M":
				male++
			case "F":
				female++
			}
		}
	}
	return male, female
}

// SpawnOneNPCFromPool 生成一名新 NPC 並放入指定房間（無排班、腦驅動）。用於 NPC 池自動補滿。回傳 entityID。
// 名字為姓＋名 2～4 字（見 GenerateNPCName），DisplayTitle 即該名，DisplayChar 取名字首字。性別依男女數持平擇一。
func SpawnOneNPCFromPool(db *sql.DB, spawnRoomID string) (string, error) {
	id := fmt.Sprintf("npc_%s", strconv.FormatInt(time.Now().UnixNano(), 36))
	if store.Default != nil {
		for store.Default.GetEntity(id) != nil {
			id = fmt.Sprintf("npc_%d_%s", time.Now().UnixNano(), strconv.FormatInt(time.Now().UnixNano()%10000, 36))
		}
	}
	name := GenerateNPCName("")
	displayChar := FirstRune(name)
	male, female := GetNPCGenderCounts(db)
	gender := "M"
	if male > female {
		gender = "F"
	} else if female > male {
		gender = "M"
	} else if rand.Intn(2) == 1 {
		gender = "F"
	}
	if err := InsertNPC(db, id, displayChar, gender, name); err != nil {
		return "", err
	}
	if err := SetEntityRoom(db, id, spawnRoomID); err != nil {
		return id, err // 已建立實體，僅房間設定失敗
	}
	return id, nil
}
