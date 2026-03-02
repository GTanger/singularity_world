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

	npcs := []struct {
		id   string
		room string
	}{
		{"陳正明", "lobby"},
		{"林小雯", "lobby"},
		{"張明德", "lobby"},
		{"王阿財", "lobby"},
	}
	for _, npc := range npcs {
		room, _ := GetEntityRoom(database, npc.id)
		if room == "" {
			t.Fatalf("NPC %s has no room", npc.id)
		}
	}

	schedules, err := GetAllSchedules(database)
	if err != nil {
		t.Fatalf("GetAllSchedules: %v", err)
	}
	if len(schedules) != 4 {
		t.Fatalf("expected 4 schedules, got %d", len(schedules))
	}

	// Hour 12: day shift on, night shift off
	if err := ApplySchedules(database, 12); err != nil {
		t.Fatalf("ApplySchedules(12): %v", err)
	}
	for _, id := range []string{"陳正明", "林小雯"} {
		room, _ := GetEntityRoom(database, id)
		if room != "lobby" {
			t.Errorf("hour 12: %s should be in lobby, got %s", id, room)
		}
	}
	for _, id := range []string{"張明德", "王阿財"} {
		room, _ := GetEntityRoom(database, id)
		if room != "back_storage" {
			t.Errorf("hour 12: %s should be in back_storage, got %s", id, room)
		}
	}

	// Hour 22: night shift on, day shift off
	if err := ApplySchedules(database, 22); err != nil {
		t.Fatalf("ApplySchedules(22): %v", err)
	}
	for _, id := range []string{"陳正明", "林小雯"} {
		room, _ := GetEntityRoom(database, id)
		if room != "back_storage" {
			t.Errorf("hour 22: %s should be in back_storage, got %s", id, room)
		}
	}
	for _, id := range []string{"張明德", "王阿財"} {
		room, _ := GetEntityRoom(database, id)
		if room != "lobby" {
			t.Errorf("hour 22: %s should be in lobby, got %s", id, room)
		}
	}

	// Hour 18: overlap - both shifts on
	if err := ApplySchedules(database, 18); err != nil {
		t.Fatalf("ApplySchedules(18): %v", err)
	}
	for _, id := range []string{"陳正明", "林小雯", "張明德", "王阿財"} {
		room, _ := GetEntityRoom(database, id)
		if room != "lobby" {
			t.Errorf("hour 18 (overlap): %s should be in lobby, got %s", id, room)
		}
	}

	// Hour 6: overlap - both shifts on
	if err := ApplySchedules(database, 6); err != nil {
		t.Fatalf("ApplySchedules(6): %v", err)
	}
	for _, id := range []string{"陳正明", "林小雯", "張明德", "王阿財"} {
		room, _ := GetEntityRoom(database, id)
		if room != "lobby" {
			t.Errorf("hour 6 (overlap): %s should be in lobby, got %s", id, room)
		}
	}
}
