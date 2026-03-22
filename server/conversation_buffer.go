// Package server 對話緩衝與結束時 consolidation：整場壓成 1～3 條寫入 archival，體感為「記住這場對話」。
package server

import (
	"sync"

	"singularity_world/db"
)

const (
	conversationGapSec = 120 // 超過 2 分鐘沒對該 NPC 說話視為「上一場結束」
	maxTurnsPerBuffer  = 20  // 單場最多保留輪數
	maxConsolidate     = 3   // 整場最多寫入 3 條
)

type conversationTurn struct {
	PlayerInput string
	NpcReply    string
	At          int64
}

var (
	bufMu   sync.RWMutex
	buffers = make(map[string][]conversationTurn)
)

func bufferKey(playerID, npcID string) string {
	return playerID + "\x00" + npcID
}

// consolidateTurns 將一場對話壓成 1～3 條精華（規則：取最後 3 輪或全場若不足 3 輪）。
func consolidateTurns(turns []conversationTurn) []string {
	if len(turns) == 0 {
		return nil
	}
	if len(turns) <= maxConsolidate {
		out := make([]string, len(turns))
		for i, t := range turns {
			out[i] = "玩家說：" + t.PlayerInput + "；NPC回：" + t.NpcReply
		}
		return out
	}
	// 取最後 3 輪，讓「最近發生的事」被記住
	start := len(turns) - maxConsolidate
	out := make([]string, maxConsolidate)
	for i := 0; i < maxConsolidate; i++ {
		t := turns[start+i]
		out[i] = "玩家說：" + t.PlayerInput + "；NPC回：" + t.NpcReply
	}
	return out
}

// summaryFromTurns 產出一句「與玩家的最近印象」供背版用，體感為 NPC 記得你。
func summaryFromTurns(turns []conversationTurn) string {
	if len(turns) == 0 {
		return ""
	}
	last := turns[len(turns)-1]
	s := last.PlayerInput
	if len(s) > 30 {
		s = s[:30] + "…"
	}
	if s == "" || s == "（搭話）" {
		return "最近曾與玩家對話。"
	}
	return "最近聊過：「" + s + "」。"
}

// FlushConversationAndAppend 若上一場已逾時則整場 consolidation 寫入 archival 並更新該 NPC 的 summary，再 append 本輪；否則只 append。呼叫端不再對本輪呼叫 InsertArchival。
func FlushConversationAndAppend(playerID, npcID, playerInput, npcReply string, now int64) {
	key := bufferKey(playerID, npcID)
	bufMu.Lock()
	defer bufMu.Unlock()
	buf := buffers[key]
	if len(buf) > 0 && now-buf[len(buf)-1].At > conversationGapSec {
		contents := consolidateTurns(buf)
		for _, c := range contents {
			_ = db.InsertArchival(npcID, c, "event")
		}
		if sum := summaryFromTurns(buf); sum != "" {
			db.SetNpcSummary(npcID, sum)
		}
		buf = nil
	}
	buf = append(buf, conversationTurn{PlayerInput: playerInput, NpcReply: npcReply, At: now})
	if len(buf) > maxTurnsPerBuffer {
		buf = buf[len(buf)-maxTurnsPerBuffer:]
	}
	buffers[key] = buf
}
