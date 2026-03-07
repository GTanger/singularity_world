package db

import (
	"database/sql"
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

// defaultNPCs 全體預設 NPC；浮生客棧四名已移除，目前為空。
var defaultNPCs = []npcDef{}

// SeedNPCs 逐一檢查預設 NPC，不存在才建立；並為四人建立指派（經理/服務生 @ 浮生客棧），對齊討論 001。
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

