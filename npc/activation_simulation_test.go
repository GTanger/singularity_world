// 與 docs/testing/NPC活化系統模擬測試報告.md 對齊：移動／Traveler 相關模擬置於 npc 套件，避免 db 測試 import npc 造成循環。

package npc

import (
	"os"
	"path/filepath"
	"testing"

	"singularity_world/db"
	"singularity_world/model"
	"singularity_world/store"
)

func setupNPCActivationSimStore(t *testing.T) {
	t.Helper()
	roomsPath := filepath.Join("..", "data", "rooms")
	if _, err := os.Stat(roomsPath); err != nil {
		t.Skip("need ../data/rooms for store tests:", err)
	}
	tmp := t.TempDir()
	rt := filepath.Join(tmp, "runtime")
	dd := filepath.Join(tmp, "data")
	if err := os.MkdirAll(rt, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(dd, 0755); err != nil {
		t.Fatal(err)
	}
	old := store.Default
	if err := store.Init(roomsPath, rt, dd); err != nil {
		t.Fatalf("store.Init: %v", err)
	}
	t.Cleanup(func() { store.Default = old })
}

// TestSim_MovementDefForTitle 模擬：依職稱取得移動定義（含 speed）；無職稱時預設 Speed=1。
func TestSim_MovementDefForTitle(t *testing.T) {
	LoadBehaviors("../data/npc_behaviors.json")
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
func TestSim_ScheduleAndTravelerTick(t *testing.T) {
	setupNPCActivationSimStore(t)
	store.Default.UpsertRoomData(&model.Room{ID: "sim_room_a", Name: "A", Description: "t"}, []model.Exit{
		{Direction: "east", ToRoomID: "sim_room_b", ToRoomName: "B"},
	})
	store.Default.UpsertRoomData(&model.Room{ID: "sim_room_b", Name: "B", Description: "t"}, []model.Exit{
		{Direction: "west", ToRoomID: "sim_room_a", ToRoomName: "A"},
	})
	g := db.GetGraph()
	if err := g.BuildGraph(); err != nil {
		t.Fatalf("BuildGraph: %v", err)
	}
	LoadBehaviors("../data/npc_behaviors.json")
	_ = db.InsertNPC("模擬班", "班", "M", "")
	_ = db.SetEntityRoom("模擬班", "sim_room_a")
	_ = db.InsertSchedule("模擬班", "sim_room_b", "sim_room_a", 6, 19)
	mgr := NewTravelerManager()
	def := GetMovementDefForTitle("經理")
	def.Type = MoveSchedule
	mgr.Register("模擬班", def)
	steps := mgr.Tick(g, 12, nil)
	if len(steps) != 1 {
		t.Fatalf("預期 Tick 後產生 1 步（A→B），got %d 步", len(steps))
	}
	if steps[0].OldRoom != "sim_room_a" || steps[0].NewRoom != "sim_room_b" {
		t.Errorf("預期一步 sim_room_a → sim_room_b，got %s → %s", steps[0].OldRoom, steps[0].NewRoom)
	}
	room, _ := db.GetEntityRoom("模擬班")
	if room != "sim_room_b" {
		t.Errorf("Tick 後實體應在 sim_room_b，got %s", room)
	}
}
