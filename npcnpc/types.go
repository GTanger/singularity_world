package npcnpc

import (
	"singularity_world/ai"
	"singularity_world/npc"
	"singularity_world/store"
)

// LastSocialChoice 記錄最近一次 NPC↔NPC 社交決策（除錯 API 用）。
type LastSocialChoice struct {
	At                   int64    `json:"at"`
	RoomID               string   `json:"room_id"`
	AID                  string   `json:"a_id"`
	BID                  string   `json:"b_id"`
	ScoreTotal           int      `json:"score_total"`
	TopicHint            string   `json:"topic_hint"`
	TopicID              string   `json:"topic_id,omitempty"`
	TopicReason          string   `json:"topic_reason,omitempty"`
	AnchorsUsed          []string `json:"anchors_used,omitempty"`
	AnchorsWritten       []string `json:"anchors_written,omitempty"`
	AnchorConflict       bool     `json:"anchor_conflict"`
	AnchorConflictReason string   `json:"anchor_conflict_reason,omitempty"`
	DialogueScore        *ai.DialogueScoreDetail `json:"dialogue_score,omitempty"`
}

// PairDebug 除錯 API：一對 NPC 的配對分數細項。
type PairDebug struct {
	AID          string                `json:"a_id"`
	BID          string                `json:"b_id"`
	LastTalkAt   int64                 `json:"last_talk_at"`
	Thread       *store.NpcThread      `json:"thread,omitempty"`
	Dyad         *store.NpcDyad        `json:"dyad,omitempty"`
	ScoreTotal   int                   `json:"score_total"`
	ScoreDetail  []string              `json:"score_detail,omitempty"`
	TopicWeights []npc.TopicWeightDebug `json:"topic_weights,omitempty"`
}

// DebugResp 為 GET /api/debug/npc-social 回應。
type DebugResp struct {
	RoomID       string            `json:"room_id"`
	RoomName     string            `json:"room_name"`
	Events       []string          `json:"events"`
	Pairs        []PairDebug       `json:"pairs"`
	NpcIDs       []string          `json:"npc_ids"`
	ServerUnix   int64             `json:"server_unix"`
	LastChoice   *LastSocialChoice `json:"last_choice,omitempty"`
	Stats        map[string]int64  `json:"stats,omitempty"`
	Rumors       []string          `json:"rumors,omitempty"`
	RumorDigest  string            `json:"rumor_digest,omitempty"`
	RumorDetails []RumorDebugItem  `json:"rumor_details,omitempty"`
}

// RumorDebugItem 傳聞池除錯一筆。
type RumorDebugItem struct {
	Text              string `json:"text"`
	Source            string `json:"source,omitempty"`
	Weight            int    `json:"weight"`
	SourceScore       int    `json:"source_score"`
	MentionCount      int    `json:"mention_count"`
	LastUsedAt        int64  `json:"last_used_at,omitempty"`
	BlockedUntil      int64  `json:"blocked_until,omitempty"`
	PenaltyCount      int    `json:"penalty_count,omitempty"`
	LastPenaltyAt     int64  `json:"last_penalty_at,omitempty"`
	LastPenaltyReason string `json:"last_penalty_reason,omitempty"`
	Status            string `json:"status"`
}
