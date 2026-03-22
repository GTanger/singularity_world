// Package db NPC 短期記憶（10.18）：見面次數、好感度。
package db

import (
	"strconv"

	"singularity_world/store"
)

// NpcMemoryRow 對齊 store.NpcMemory。
type NpcMemoryRow struct {
	EntityID     string
	SubjectID    string
	MeetCount    int
	Favorability int
}

// GetNpcMemory 回傳該 NPC 對某玩家的短期記憶；無則 nil。
func GetNpcMemory(entityID, subjectID string) *NpcMemoryRow {
	if store.Default == nil {
		return nil
	}
	m := store.Default.GetNpcMemory(entityID, subjectID)
	if m == nil {
		return nil
	}
	return &NpcMemoryRow{EntityID: m.EntityID, SubjectID: m.SubjectID, MeetCount: m.MeetCount, Favorability: m.Favorability}
}

// RecordMeet 記錄一次見面：若無則建立 (meet_count=1)，若有則 meet_count+1。
func RecordMeet(entityID, subjectID string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.RecordMeet(entityID, subjectID)
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
func AdjustFavorability(entityID, subjectID string, delta int) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.AdjustFavorability(entityID, subjectID, delta)
}

// FormatNpcMemoryForBackstory 回傳一句「與這位來者的短期記憶」供拼進背版；無記憶或見面 0 次回傳空字串。
func FormatNpcMemoryForBackstory(entityID, subjectID string) string {
	m := GetNpcMemory(entityID, subjectID)
	if m == nil || m.MeetCount <= 0 {
		return ""
	}
	s := "你與這位來者見過" + strconv.Itoa(m.MeetCount) + "次。"
	if m.Favorability >= 30 {
		s += "你對其印象不錯。"
	} else if m.Favorability <= -30 {
		s += "你對其印象不佳。"
	} else {
		s += "你對其印象普通。"
	}
	return s
}
