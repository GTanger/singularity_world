package db

import (
	"database/sql"
	"os"
	"testing"
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
		// Overlap: both on at 18, both on at 06
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

func setupTestDB(t *testing.T) *sql.DB {
	t.Helper()
	path := t.TempDir() + "/test.db"
	database, err := OpenDB(path)
	if err != nil {
		t.Fatalf("OpenDB: %v", err)
	}
	t.Cleanup(func() {
		database.Close()
		os.Remove(path)
	})
	return database
}

func TestApplySchedules(t *testing.T) {
	database := setupTestDB(t)

	workRoom := "life_hall"
	restRoom := "life_storage"

	// 預設四名 NPC 已從 seed 移除，測試內手動建立實體與排班（ApplySchedules 只回傳「應前往」清單，不傳送）
	for _, id := range []string{"陳正明", "林小雯", "張明德", "王阿財"} {
		if err := InsertNPC(database, id, string([]rune(id)[0]), "M", ""); err != nil {
			t.Fatalf("InsertNPC %s: %v", id, err)
		}
		_ = SetEntityRoom(database, id, workRoom)
	}
	// 日班 06-19、夜班 18-07
	_ = InsertSchedule(database, "陳正明", workRoom, restRoom, 6, 19)
	_ = InsertSchedule(database, "林小雯", workRoom, restRoom, 6, 19)
	_ = InsertSchedule(database, "張明德", workRoom, restRoom, 18, 7)
	_ = InsertSchedule(database, "王阿財", workRoom, restRoom, 18, 7)

	schedules, err := GetAllSchedules(database)
	if err != nil {
		t.Fatalf("GetAllSchedules: %v", err)
	}
	if len(schedules) != 4 {
		t.Fatalf("expected 4 schedules, got %d", len(schedules))
	}

	// Hour 12: 日班應在 work、夜班應在 rest；所有人目前在 work，故夜班兩人會進 moves（應前往 rest）
	moves, err := ApplySchedules(database, 12)
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
	// 不傳送：實體仍留在原房間
	for _, id := range []string{"張明德", "王阿財"} {
		room, _ := GetEntityRoom(database, id)
		if room != workRoom {
			t.Errorf("hour 12 (no teleport): %s should still be in %s, got %s", id, workRoom, room)
		}
	}

	// Hour 22: 夜班應在 work、日班應在 rest；若仍在 work 則日班兩人進 moves
	moves22, _ := ApplySchedules(database, 22)
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
	database := setupTestDB(t)
	workRoom := "life_hall"
	restRoom := "life_storage"
	_ = InsertNPC(database, "試算", "試", "M", "")
	_ = SetEntityRoom(database, "試算", workRoom)
	_ = InsertSchedule(database, "試算", workRoom, restRoom, 6, 19)

	target, ok := GetScheduleTarget(database, "試算", 12)
	if !ok {
		t.Fatal("GetScheduleTarget(12): want ok true")
	}
	if target.Room != workRoom || !target.IsWork {
		t.Errorf("hour 12: want room=%s isWork=true, got room=%s isWork=%v", workRoom, target.Room, target.IsWork)
	}

	target, ok = GetScheduleTarget(database, "試算", 22)
	if !ok {
		t.Fatal("GetScheduleTarget(22): want ok true")
	}
	if target.Room != restRoom || target.IsWork {
		t.Errorf("hour 22: want room=%s isWork=false, got room=%s isWork=%v", restRoom, target.Room, target.IsWork)
	}

	_, ok = GetScheduleTarget(database, "不存在", 12)
	if ok {
		t.Error("GetScheduleTarget(不存在): want ok false")
	}
}
