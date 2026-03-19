// Package db NPC 短期記憶（10.18）：見面次數、好感度。store 啟用時走 store，否則走 SQL。
package db

import (
	"database/sql"
	"strconv"

	"singularity_world/store"
)

// NpcMemoryRow 對齊 store.NpcMemory，供 SQL 查詢用。
type NpcMemoryRow struct {
	EntityID    string
	SubjectID   string
	MeetCount   int
	Favorability int
}

// GetNpcMemory 回傳該 NPC 對某玩家的短期記憶；無則 nil。
func GetNpcMemory(database *sql.DB, entityID, subjectID string) *NpcMemoryRow {
	if store.Default != nil {
		m := store.Default.GetNpcMemory(entityID, subjectID)
		if m == nil {
			return nil
		}
		return &NpcMemoryRow{EntityID: m.EntityID, SubjectID: m.SubjectID, MeetCount: m.MeetCount, Favorability: m.Favorability}
	}
	if database == nil {
		return nil
	}
	var meetCount, favorability int
	err := database.QueryRow(
		"SELECT meet_count, favorability FROM npc_memory WHERE entity_id = ? AND subject_id = ?",
		entityID, subjectID,
	).Scan(&meetCount, &favorability)
	if err == sql.ErrNoRows {
		return nil
	}
	if err != nil {
		return nil
	}
	return &NpcMemoryRow{EntityID: entityID, SubjectID: subjectID, MeetCount: meetCount, Favorability: favorability}
}

// RecordMeet 記錄一次見面：若無則建立 (meet_count=1)，若有則 meet_count+1。
func RecordMeet(database *sql.DB, entityID, subjectID string) error {
	if store.Default != nil {
		return store.Default.RecordMeet(entityID, subjectID)
	}
	if database == nil {
		return nil
	}
	_, err := database.Exec(
		`INSERT INTO npc_memory (entity_id, subject_id, meet_count, favorability) VALUES (?, ?, 1, 0)
		 ON CONFLICT(entity_id, subject_id) DO UPDATE SET meet_count = meet_count + 1`,
		entityID, subjectID,
	)
	return err
}

// Favorability 常數（與 disposition 分開：disposition=心境，favorability=對該玩家的好感）
const (
	FavTalk          = 5   // 對話一次
	FavAttack        = -15 // 被送行（舊稱攻擊）；與 FavSlay 同值
	FavSlay          = -15 // 被該玩家送行（致死意圖戰鬥）
	FavSubdue        = -8  // 被該玩家留人（制伏）
	FavBorrowCaught  = -12 // 借物當場被喝破
	FavBorrowSuccess = -3  // 借物得手（較輕）
)

// AdjustFavorability 調整該 NPC 對某玩家的好感度，clamp [-100, +100]。
func AdjustFavorability(database *sql.DB, entityID, subjectID string, delta int) error {
	if store.Default != nil {
		return store.Default.AdjustFavorability(entityID, subjectID, delta)
	}
	if database == nil {
		return nil
	}
	_, err := database.Exec(
		`INSERT INTO npc_memory (entity_id, subject_id, meet_count, favorability) VALUES (?, ?, 0, ?)
		 ON CONFLICT(entity_id, subject_id) DO UPDATE SET favorability = MAX(-100, MIN(100, favorability + ?))`,
		entityID, subjectID, delta, delta,
	)
	return err
}

// FormatNpcMemoryForBackstory 回傳一句「與這位來者的短期記憶」供拼進背版；無記憶或見面 0 次回傳空字串。
func FormatNpcMemoryForBackstory(database *sql.DB, entityID, subjectID string) string {
	m := GetNpcMemory(database, entityID, subjectID)
	if m == nil || m.MeetCount <= 0 {
		return ""
	}
	// 見面次數
	s := "你與這位來者見過" + strconv.Itoa(m.MeetCount) + "次。"
	// 好感度口語化
	if m.Favorability >= 30 {
		s += "你對其印象不錯。"
	} else if m.Favorability <= -30 {
		s += "你對其印象不佳。"
	} else {
		s += "你對其印象普通。"
	}
	return s
}
