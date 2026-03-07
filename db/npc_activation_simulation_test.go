// Package db 的 NPC 活化系統模擬測試：從生成、soul_seed 展開、性格、排班到移動 Tick 的端到端驗證。
// 對應文件：docs/testing/NPC活化系統模擬測試報告.md

package db

import (
	"database/sql"
	"os"
	"testing"
)

// setupSimDB 建立測試用 DB（與 schedule_test 相同，OpenDB 會執行 schema 與遷移）。
func setupSimDB(t *testing.T) *sql.DB {
	t.Helper()
	path := t.TempDir() + "/sim.db"
	db, err := OpenDB(path)
	if err != nil {
		t.Fatalf("OpenDB: %v", err)
	}
	t.Cleanup(func() {
		db.Close()
		os.Remove(path)
	})
	return db
}

// TestSim_NPCGenerationWithSoulSeed 模擬：NPC 生成後必帶 soul_seed，且體敏氣由該 seed 展開。
func TestSim_NPCGenerationWithSoulSeed(t *testing.T) {
	db := setupSimDB(t)
	// 生成一名 NPC，流程同 InsertNPC：GenerateSoulSeed → ExpandSoulSeedToBaseStats → 寫入 entities
	err := InsertNPC(db, "模擬甲", "模", "M", "")
	if err != nil {
		t.Fatalf("InsertNPC: %v", err)
	}
	ent, err := GetEntity(db, "模擬甲")
	if err != nil || ent == nil {
		t.Fatalf("GetEntity: %v", err)
	}
	// 必須有 SoulSeed
	if ent.SoulSeed == nil {
		t.Fatal("NPC 生成後應帶 soul_seed，實作規定創角即寫入")
	}
	// 體敏氣應與同 seed 展開結果一致
	seed := *ent.SoulSeed
	vit, qi, dex := ExpandSoulSeedToBaseStats(seed)
	if ent.Vit != vit || ent.Qi != qi || ent.Dex != dex {
		t.Errorf("體敏氣應與 ExpandSoulSeedToBaseStats(seed) 一致: got vit=%d qi=%d dex=%d, want vit=%d qi=%d dex=%d",
			ent.Vit, ent.Qi, ent.Dex, vit, qi, dex)
	}
}

// TestSim_SoulSeedDeterminism 模擬：同一 seed 多次展開，BaseStats / OriginSentence / Personality 皆確定性一致。
func TestSim_SoulSeedDeterminism(t *testing.T) {
	const seed int64 = 12345
	vit1, qi1, dex1 := ExpandSoulSeedToBaseStats(seed)
	vit2, qi2, dex2 := ExpandSoulSeedToBaseStats(seed)
	if vit1 != vit2 || qi1 != qi2 || dex1 != dex2 {
		t.Errorf("BaseStats 應確定性: (%d,%d,%d) vs (%d,%d,%d)", vit1, qi1, dex1, vit2, qi2, dex2)
	}
	origin1 := ExpandSoulSeedToOriginSentence(seed)
	origin2 := ExpandSoulSeedToOriginSentence(seed)
	if origin1 != origin2 {
		t.Errorf("OriginSentence 應確定性: %q vs %q", origin1, origin2)
	}
	p1 := ExpandSoulSeedToPersonality(seed)
	p2 := ExpandSoulSeedToPersonality(seed)
	if p1.Boldness != p2.Boldness || p1.Sensitivity != p2.Sensitivity || p1.Orderliness != p2.Orderliness {
		t.Errorf("Personality 應確定性: %+v vs %+v", p1, p2)
	}
	// 性格三軸應落在 [0,1]
	if p1.Boldness < 0 || p1.Boldness > 1 || p1.Sensitivity < 0 || p1.Sensitivity > 1 || p1.Orderliness < 0 || p1.Orderliness > 1 {
		t.Errorf("Personality 應在 [0,1]: %+v", p1)
	}
}

// TestSim_GetPersonalityForEntity 模擬：有 soul_seed 的實體可取得 Personality；無則回傳零值與 false。
func TestSim_GetPersonalityForEntity(t *testing.T) {
	db := setupSimDB(t)
	_ = InsertNPC(db, "有種子", "有", "M", "")
	// 有 seed → 應取得性格
	p, ok := GetPersonalityForEntity(db, "有種子")
	if !ok {
		t.Fatal("有 soul_seed 的 NPC 應回傳 ok=true")
	}
	if p.Boldness < 0 || p.Boldness > 1 {
		t.Errorf("Boldness 應在 [0,1]: %f", p.Boldness)
	}
	// 不存在的實體
	_, ok = GetPersonalityForEntity(db, "不存在ID")
	if ok {
		t.Error("不存在的實體應回傳 ok=false")
	}
}

// TestSim_MovementDefForTitle 模擬：依職稱取得移動定義（含 speed）；無職稱時預設 Speed=1。
func TestSim_MovementDefForTitle(t *testing.T) {
	// 須先載入行為檔，測試環境可能無檔案，故只驗證預設與結構
	LoadBehaviors("data/npc_behaviors.json")
	def := GetMovementDefForTitle("經理")
	if def.Speed < 1 {
		t.Errorf("MovementDef.Speed 至少為 1，got %d", def.Speed)
	}
	if def.Type != MoveRegional {
		t.Errorf("無 movement 時預設 Type 為 MoveRegional，got %s", def.Type)
	}
	defUnknown := GetMovementDefForTitle("不存在的職稱")
	if defUnknown.Speed != 1 {
		t.Errorf("未知職稱應預設 Speed=1，got %d", defUnknown.Speed)
	}
}

// TestSim_ScheduleAndTravelerTick 模擬：排班型 NPC 註冊後，Tick 依目標房間尋路並回傳一步。
// 建立兩房一出口、一名在 A 的 NPC、排班目標 B（在班），Tick 後應產生一步 A→B。
func TestSim_ScheduleAndTravelerTick(t *testing.T) {
	db := setupSimDB(t)
	// 兩房一出口，供 BFS 尋路
	_, _ = db.Exec("INSERT OR REPLACE INTO rooms (id, name, description, tags, zone) VALUES ('sim_room_a', 'A', '', '[]', ''), ('sim_room_b', 'B', '', '[]', '')")
	_, _ = db.Exec("INSERT OR REPLACE INTO exits (from_room_id, direction, to_room_id) VALUES ('sim_room_a', 'east', 'sim_room_b'), ('sim_room_b', 'west', 'sim_room_a')")
	// 一名 NPC，目前在 A，排班：工作區 B、休息區 A，日班 6–19
	_ = InsertNPC(db, "模擬班", "班", "M", "")
	_ = SetEntityRoom(db, "模擬班", "sim_room_a")
	_ = InsertSchedule(db, "模擬班", "sim_room_b", "sim_room_a", 6, 19)
	// 重建圖（使用當前 DB 的 rooms/exits）
	g := GetGraph()
	if err := g.BuildGraph(db); err != nil {
		t.Fatalf("BuildGraph: %v", err)
	}
	LoadBehaviors("data/npc_behaviors.json")
	mgr := NewTravelerManager()
	def := GetMovementDefForTitle("經理") // 須先 LoadBehaviors，經理有 movement.speed
	def.Type = MoveSchedule
	mgr.Register("模擬班", def)
	// 遊戲時 12 → 在班 → 目標 work_room = sim_room_b
	steps := mgr.Tick(db, g, 12)
	if len(steps) != 1 {
		t.Fatalf("預期 Tick 後產生 1 步（A→B），got %d 步", len(steps))
	}
	if steps[0].OldRoom != "sim_room_a" || steps[0].NewRoom != "sim_room_b" {
		t.Errorf("預期一步 sim_room_a → sim_room_b，got %s → %s", steps[0].OldRoom, steps[0].NewRoom)
	}
	// 實體應已寫回 store 或 DB
	room, _ := GetEntityRoom(db, "模擬班")
	if room != "sim_room_b" {
		t.Errorf("Tick 後實體應在 sim_room_b，got %s", room)
	}
}
