package db

import (
	"os"
	"path/filepath"
	"testing"

	"singularity_world/store"
)

func TestIsOnDuty(t *testing.T) {
	day := NPCSchedule{ShiftStart: 6, ShiftEnd: 19}
	night := NPCSchedule{ShiftStart: 18, ShiftEnd: 7}

	tests := []struct {
		name  string
		sched NPCSchedule
		hour  int
		want  bool
	}{
		{"day@05", day, 5, false},
		{"day@06", day, 6, true},
		{"day@12", day, 12, true},
		{"day@18", day, 18, true},
		{"day@19", day, 19, false},
		{"day@23", day, 23, false},
		{"night@17", night, 17, false},
		{"night@18", night, 18, true},
		{"night@23", night, 23, true},
		{"night@00", night, 0, true},
		{"night@06", night, 6, true},
		{"night@07", night, 7, false},
		{"night@12", night, 12, false},
		{"overlap_day@18", day, 18, true},
		{"overlap_night@18", night, 18, true},
		{"overlap_day@06", day, 6, true},
		{"overlap_night@06", night, 6, true},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := tc.sched.IsOnDuty(tc.hour)
			if got != tc.want {
				t.Errorf("IsOnDuty(%d) = %v, want %v", tc.hour, got, tc.want)
			}
		})
	}
}

// setupTestStore 以專案 data/rooms 與暫存 runtime/data 初始化 store（SQLite 已移除）。
func setupTestStore(t *testing.T) {
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

func TestApplySchedules(t *testing.T) {
	setupTestStore(t)

	workRoom := "life_hall"
	restRoom := "life_storage"

	for _, id := range []string{"陳正明", "林小雯", "張明德", "王阿財"} {
		if err := InsertNPC(id, string([]rune(id)[0]), "M", ""); err != nil {
			t.Fatalf("InsertNPC %s: %v", id, err)
		}
		_ = SetEntityRoom(id, workRoom)
	}
	_ = InsertSchedule("陳正明", workRoom, restRoom, 6, 19)
	_ = InsertSchedule("林小雯", workRoom, restRoom, 6, 19)
	_ = InsertSchedule("張明德", workRoom, restRoom, 18, 7)
	_ = InsertSchedule("王阿財", workRoom, restRoom, 18, 7)

	schedules, err := GetAllSchedules()
	if err != nil {
		t.Fatalf("GetAllSchedules: %v", err)
	}
	if len(schedules) < 4 {
		t.Fatalf("expected at least 4 schedules (test created 4), got %d", len(schedules))
	}

	moves, err := ApplySchedules(12)
	if err != nil {
		t.Fatalf("ApplySchedules(12): %v", err)
	}
	if len(moves) != 2 {
		t.Errorf("hour 12: expected 2 moves (night shift to rest), got %d", len(moves))
	}
	for _, m := range moves {
		if m.NewRoom != restRoom {
			t.Errorf("hour 12 move: %s NewRoom want %s, got %s", m.EntityID, restRoom, m.NewRoom)
		}
	}
	for _, id := range []string{"張明德", "王阿財"} {
		room, _ := GetEntityRoom(id)
		if room != workRoom {
			t.Errorf("hour 12 (no teleport): %s should still be in %s, got %s", id, workRoom, room)
		}
	}

	moves22, _ := ApplySchedules(22)
	if len(moves22) != 2 {
		t.Errorf("hour 22: expected 2 moves (day shift to rest), got %d", len(moves22))
	}
	for _, m := range moves22 {
		if m.NewRoom != restRoom {
			t.Errorf("hour 22 move: %s NewRoom want %s, got %s", m.EntityID, restRoom, m.NewRoom)
		}
	}
}

func TestGetScheduleTarget(t *testing.T) {
	setupTestStore(t)
	workRoom := "life_hall"
	restRoom := "life_storage"
	_ = InsertNPC("試算", "試", "M", "")
	_ = SetEntityRoom("試算", workRoom)
	_ = InsertSchedule("試算", workRoom, restRoom, 6, 19)

	target, ok := GetScheduleTarget("試算", 12)
	if !ok {
		t.Fatal("GetScheduleTarget(12): want ok true")
	}
	if target.Room != workRoom || !target.IsWork {
		t.Errorf("hour 12: want room=%s isWork=true, got room=%s isWork=%v", workRoom, target.Room, target.IsWork)
	}

	target, ok = GetScheduleTarget("試算", 22)
	if !ok {
		t.Fatal("GetScheduleTarget(22): want ok true")
	}
	if target.Room != restRoom || target.IsWork {
		t.Errorf("hour 22: want room=%s isWork=false, got room=%s isWork=%v", restRoom, target.Room, target.IsWork)
	}

	_, ok = GetScheduleTarget("不存在", 12)
	if ok {
		t.Error("GetScheduleTarget(不存在): want ok false")
	}
}
