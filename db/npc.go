package db

import (
	"fmt"
	"math/rand"
	"strconv"
	"time"

	"singularity_world/store"
)

// InsertNPC 新增一筆 NPC 實體；寫入 store 並持久化 entities.json。
func InsertNPC(id, displayChar, gender, displayTitle string) error {
	if store.Default == nil {
		return ErrNoStore
	}
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
	return store.Default.PutEntity(&store.Entity{
		ID: id, Kind: "npc", DisplayChar: displayChar,
		X: 0, Y: 0, MoveState: "idle",
		Vit: vit, Qi: qi, Dex: dex, Magnesium: 100,
		CreatedAt: now, Gender: gender, SoulSeed: &seed,
		DisplayTitle: displayTitle, EquipmentSlots: equip,
		Inventory: "[]", ActivatedNodes: `["N000"]`,
	})
}

// InsertSchedule 設定 NPC 排班；寫入 store 並持久化 data/schedules.json。
func InsertSchedule(entityID, workRoom, restRoom string, shiftStart, shiftEnd int) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.InsertSchedule(entityID, workRoom, restRoom, shiftStart, shiftEnd)
}

// npcDef 描述一名預設 NPC 的全部資料。
type npcDef struct {
	id, displayChar, gender, title string
	workRoom, restRoom             string
	shiftStart, shiftEnd           int
}

// defaultNPCs 全體預設 NPC；先人後嘴用：至少 1 名在創生房，供 Talk（I3）驗收。
// work_room/rest_room 使用創生房 id（界壁 = start_boundary），與新玩家同房。
// （試話等測試用 NPC 已移除；職稱「服務生」仍為正式 occupation，見 occupations.json）
var defaultNPCs = []npcDef{}

// SeedNPCs 逐一檢查預設 NPC（舊 SQLite 流程已移除；請用 SeedNPCsForStore）。
func SeedNPCs() error {
	return SeedNPCsForStore()
}

// SeedNPCsForStore 確保預設 NPC 存在；不存在則建立並設房間／排班／指派。main 啟動時 store.Init 後呼叫。
func SeedNPCsForStore() error {
	if store.Default == nil {
		return nil
	}
	const venueLifeInn = "venue_life_inn"
	for _, npc := range defaultNPCs {
		if store.Default.GetEntity(npc.id) != nil {
			_ = InsertSchedule(npc.id, npc.workRoom, npc.restRoom, npc.shiftStart, npc.shiftEnd)
			_ = InsertAssignment(npc.id, npc.title, venueLifeInn, "")
			continue
		}
		if err := InsertNPC(npc.id, npc.displayChar, npc.gender, ""); err != nil {
			return err
		}
		if err := SetEntityRoom(npc.id, npc.workRoom); err != nil {
			return err
		}
		if err := InsertSchedule(npc.id, npc.workRoom, npc.restRoom, npc.shiftStart, npc.shiftEnd); err != nil {
			return err
		}
		if err := InsertAssignment(npc.id, npc.title, venueLifeInn, ""); err != nil {
			return err
		}
	}
	return nil
}

// GetNPCGenderCounts 回傳有房間的 NPC 中男(M)、女(F) 的數量，供生成時男女持平用。
func GetNPCGenderCounts() (male, female int) {
	if store.Default == nil {
		return 0, 0
	}
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

// EnsureAllNPCsHaveSoulSeed 保證所有 NPC 都有 soul_seed；對缺漏者補寫 GenerateSoulSeed()，不改動 vit/qi/dex。
func EnsureAllNPCsHaveSoulSeed() (fixed int, err error) {
	if store.Default == nil {
		return 0, ErrNoStore
	}
	ids := store.Default.NPCIDsWithMissingSoulSeed()
	for _, id := range ids {
		seed, e := GenerateSoulSeed()
		if e != nil {
			return fixed, e
		}
		if store.Default.UpdateEntity(id, func(e *store.Entity) { e.SoulSeed = &seed }) != nil {
			continue
		}
		fixed++
	}
	return fixed, nil
}

// SpawnOneNPCFromPool 生成一名新 NPC 並放入指定房間（無排班、腦驅動）。用於 NPC 池自動補滿。回傳 entityID。
func SpawnOneNPCFromPool(spawnRoomID string) (string, error) {
	if store.Default == nil {
		return "", ErrNoStore
	}
	id := fmt.Sprintf("npc_%s", strconv.FormatInt(time.Now().UnixNano(), 36))
	for store.Default.GetEntity(id) != nil {
		id = fmt.Sprintf("npc_%d_%s", time.Now().UnixNano(), strconv.FormatInt(time.Now().UnixNano()%10000, 36))
	}
	name := GenerateNPCName("")
	displayChar := FirstRune(name)
	male, female := GetNPCGenderCounts()
	gender := "M"
	if male > female {
		gender = "F"
	} else if female > male {
		gender = "M"
	} else if rand.Intn(2) == 1 {
		gender = "F"
	}
	if err := InsertNPC(id, displayChar, gender, name); err != nil {
		return "", err
	}
	if err := SetEntityRoom(id, spawnRoomID); err != nil {
		return id, err
	}
	return id, nil
}
