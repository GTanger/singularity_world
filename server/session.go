// Package server 負責 WebSocket 連線管理與玩家 session。本檔為玩家 session 綁定。
package server

import (
	"database/sql"
	"strings"
	"sync"
	"time"

	"singularity_world/db"
)

// Session 代表一名玩家的連線會話，綁定 Client 與玩家實體 ID，供遊戲邏輯辨識誰在操作。
type Session struct {
	Client          *Client
	PlayerID        string
	LastTalkAt      time.Time // 上次執行 Talk 動作的時間；用於同房玩家優先 LLM，無對話行為時才觸發 NPC 間對話
	LastTalkRoom    string    // 上次 Talk 時所在房間（供 NPC 閒聊「餘音」）
	LastTalkSnippet string    // 玩家輸入節錄（供同房 NPC 對話側面呼應）
}

// SessionStore 以 PlayerID 為 key 儲存目前上線的 session；登入時寫入、斷線時移除。
type SessionStore struct {
	mu       sync.RWMutex
	sessions map[string]*Session
}

// NewSessionStore 建立空的 SessionStore。回傳 *SessionStore，無副作用。
func NewSessionStore() *SessionStore {
	return &SessionStore{
		sessions: make(map[string]*Session),
	}
}

// Set 將 playerID 與 session 綁定；若該 playerID 已有 session 則覆寫。
// 參數：playerID 為實體 ID；s 為 *Session。
// 副作用：寫入 sessions map。
func (st *SessionStore) Set(playerID string, s *Session) {
	st.mu.Lock()
	defer st.mu.Unlock()
	st.sessions[playerID] = s
}

// Get 依 playerID 取得 Session，不存在則回傳 nil。
func (st *SessionStore) Get(playerID string) *Session {
	st.mu.RLock()
	defer st.mu.RUnlock()
	return st.sessions[playerID]
}

// Remove 移除該 playerID 的 session。
func (st *SessionStore) Remove(playerID string) {
	st.mu.Lock()
	defer st.mu.Unlock()
	delete(st.sessions, playerID)
}

// AllPlayerIDs 回傳目前所有已綁定之玩家 ID，供取得觀測者座標用。
func (st *SessionStore) AllPlayerIDs() []string {
	st.mu.RLock()
	defer st.mu.RUnlock()
	ids := make([]string, 0, len(st.sessions))
	for id := range st.sessions {
		ids = append(ids, id)
	}
	return ids
}

// AllSessions 回傳目前所有 session 的快照（slice），供外部遍歷推送用。
func (st *SessionStore) AllSessions() []*Session {
	st.mu.RLock()
	defer st.mu.RUnlock()
	list := make([]*Session, 0, len(st.sessions))
	for _, s := range st.sessions {
		list = append(list, s)
	}
	return list
}

// UpdateLastTalkAt 更新該玩家上次執行 Talk 的時間；同房玩家優先 LLM 時由 handler 呼叫。
func (st *SessionStore) UpdateLastTalkAt(playerID string) {
	st.mu.Lock()
	defer st.mu.Unlock()
	if s := st.sessions[playerID]; s != nil {
		s.LastTalkAt = time.Now()
	}
}

// sanitizeTalkSnippet 節錄玩家對白供 NPC 閒聊餘音（去換行、限長）。
func sanitizeTalkSnippet(s string) string {
	s = strings.TrimSpace(s)
	if s == "" || s == "（搭話）" {
		return ""
	}
	s = strings.ReplaceAll(s, "\r", " ")
	s = strings.ReplaceAll(s, "\n", " ")
	for strings.Contains(s, "  ") {
		s = strings.ReplaceAll(s, "  ", " ")
	}
	if len(s) > 256 {
		s = s[:256]
	}
	r := []rune(s)
	if len(r) > 48 {
		s = string(r[:48]) + "…"
	}
	return s
}

// RecordPlayerTalkForRoomEcho 記錄玩家 Talk 的時間、房間與對白節錄，供稍後同房 NPC 閒聊帶過一句「餘音」。
func (st *SessionStore) RecordPlayerTalkForRoomEcho(playerID, roomID, playerInput string) {
	st.mu.Lock()
	defer st.mu.Unlock()
	s := st.sessions[playerID]
	if s == nil {
		return
	}
	s.LastTalkAt = time.Now()
	s.LastTalkRoom = roomID
	s.LastTalkSnippet = sanitizeTalkSnippet(playerInput)
}

// PlayerTalkEchoForNpcSocial 若該房內有玩家曾在本房 Talk，且已超過 NPC 閒聊冷卻（60s）、未過期（4min），回傳節錄字串供注入 prompt。
func (st *SessionStore) PlayerTalkEchoForNpcSocial(database *sql.DB, roomID string) string {
	if roomID == "" {
		return ""
	}
	st.mu.RLock()
	defer st.mu.RUnlock()
	var bestSnippet string
	var bestAt time.Time
	for _, s := range st.sessions {
		if s == nil || s.LastTalkSnippet == "" || s.LastTalkRoom != roomID {
			continue
		}
		rid, _ := db.GetEntityRoom(database, s.PlayerID)
		if rid != roomID {
			continue
		}
		age := time.Since(s.LastTalkAt)
		if age < 60*time.Second || age > 4*time.Minute {
			continue
		}
		if bestSnippet == "" || s.LastTalkAt.After(bestAt) {
			bestSnippet = s.LastTalkSnippet
			bestAt = s.LastTalkAt
		}
	}
	return bestSnippet
}

// RoomHasPlayer 回傳該房內是否有玩家連線在場（不論是否剛 Talk）。
func (st *SessionStore) RoomHasPlayer(database *sql.DB, roomID string) bool {
	st.mu.RLock()
	defer st.mu.RUnlock()
	for _, s := range st.sessions {
		rid, _ := db.GetEntityRoom(database, s.PlayerID)
		if rid == roomID {
			return true
		}
	}
	return false
}

// RoomHasPlayerWithRecentTalk 回傳該房內是否有玩家在 within 時間內執行過 Talk；有則不觸發 NPC 間 AI 對話。
func (st *SessionStore) RoomHasPlayerWithRecentTalk(database *sql.DB, roomID string, within time.Duration) bool {
	st.mu.RLock()
	defer st.mu.RUnlock()
	for _, s := range st.sessions {
		rid, _ := db.GetEntityRoom(database, s.PlayerID)
		if rid != roomID {
			continue
		}
		if !s.LastTalkAt.IsZero() && time.Since(s.LastTalkAt) < within {
			return true
		}
	}
	return false
}
