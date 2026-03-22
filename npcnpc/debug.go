package npcnpc

import (
	"encoding/json"
	"fmt"
	"math/rand"
	"net/http"
	"strings"
	"time"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/game"
	"singularity_world/gametext"
	"singularity_world/npc"
)

// HandleSocialDebug 處理 GET /api/debug/npc-social。
func HandleSocialDebug(w http.ResponseWriter, r *http.Request, cfg config.Server) {
	if r.Method != http.MethodGet {
		http.Error(w, `{"error":"method not allowed"}`, http.StatusMethodNotAllowed)
		return
	}
	roomID := strings.TrimSpace(r.URL.Query().Get("room_id"))
	if roomID == "" {
		http.Error(w, `{"error":"need room_id"}`, http.StatusBadRequest)
		return
	}
	roomName, _ := db.GetRoomName(roomID)
	gh := 12
	if cfg.GameTimeEpochUnix != 0 {
		_, gh, _, _ = game.GameTimeNow(time.Now().Unix(), cfg.GameTimeEpochUnix, cfg.GameTimeScale)
	}
	entities, _ := db.GetEntitiesInRoom(roomID, gh)
	var npcs []*entity.Character
	for _, e := range entities {
		if e != nil && e.Kind == "npc" {
			npcs = append(npcs, e)
		}
	}
	resp := DebugResp{
		RoomID:     roomID,
		RoomName:   roomName,
		Events:     RecentRoomEvents(roomID, 5),
		ServerUnix: time.Now().Unix(),
	}
	if lc := LastChoiceSnapshot(); lc != nil {
		resp.LastChoice = lc
	}
	resp.Stats = StatsSnapshot()
	nowUnix := time.Now().Unix()
	hour := 12
	if s := config.DefaultServer(); s.GameTimeEpochUnix != 0 {
		_, hour, _, _ = game.GameTimeNow(nowUnix, s.GameTimeEpochUnix, s.GameTimeScale)
	}
	mask := TopicMaskForRoom(roomID, hour)
	if room, _ := db.GetRoom(roomID); room != nil {
		for _, rum := range db.TopNpcRumors(roomID, room.Zone, nowUnix, 3) {
			resp.Rumors = append(resp.Rumors, rum.Text)
		}
		for _, rr := range db.DebugNpcRumors(roomID, room.Zone, nowUnix, 12) {
			status := "active"
			if rr.BlockedUntil > nowUnix {
				status = "blocked"
			}
			resp.RumorDetails = append(resp.RumorDetails, RumorDebugItem{
				Text:              rr.Text,
				Source:            rr.Source,
				Weight:            rr.Weight,
				SourceScore:       rr.SourceScore,
				MentionCount:      rr.MentionCount,
				LastUsedAt:        rr.LastUsedAt,
				BlockedUntil:      rr.BlockedUntil,
				PenaltyCount:      rr.PenaltyCount,
				LastPenaltyAt:     rr.LastPenaltyAt,
				LastPenaltyReason: rr.LastPenaltyReason,
				Status:            status,
			})
		}
	}
	if d := db.GetNpcRumorDigest(); d != nil {
		resp.RumorDigest = d.Text
	}
	for _, n := range npcs {
		resp.NpcIDs = append(resp.NpcIDs, n.ID)
	}
	for i := 0; i < len(npcs); i++ {
		for j := i + 1; j < len(npcs); j++ {
			a, b := npcs[i], npcs[j]
			if a == nil || b == nil {
				continue
			}
			thread := db.GetNpcNpcThread(a.ID, b.ID)
			dyad := db.GetNpcNpcDyad(a.ID, b.ID)
			score := 0
			detail := make([]string, 0, 8)
			nb := config.Sim().NpcNpcPairPick.ScoreBaseNoise
			if nb <= 0 {
				nb = 4
			}
			noise := rand.Intn(nb)
			score += noise
			detail = append(detail, fmt.Sprintf("base_noise=%d", noise))
			if thread != nil && thread.Phase != "cooling" {
				score += 100
				detail = append(detail, "active_thread=+100")
			}
			if dyad != nil {
				v := dyad.Familiarity / 20
				score += v
				detail = append(detail, fmt.Sprintf("familiarity=%d/20 => +%d", dyad.Familiarity, v))
				if dyad.Sentiment <= -30 {
					score -= 2
					detail = append(detail, "sentiment<=-30 => -2")
				}
				if HasTag(dyad.Tags, gametext.DyadTag("same_venue")) {
					score += 2
					detail = append(detail, gametext.DebugPairTagSameVenue())
				}
			}
			if NpcPairSameVenue(a.ID, b.ID) {
				score += 3
				detail = append(detail, "sameVenue => +3")
			}
			if last := GetPairLastTalk(a.ID, b.ID); last > 0 && nowUnix-last < 120 {
				score -= 20
				detail = append(detail, "talked<120s => -20")
			}
			fam, sen := 0, 0
			var tags []string
			if dyad != nil {
				fam, sen, tags = dyad.Familiarity, dyad.Sentiment, dyad.Tags
			}
			resp.Pairs = append(resp.Pairs, PairDebug{
				AID:          a.ID,
				BID:          b.ID,
				LastTalkAt:   GetPairLastTalk(a.ID, b.ID),
				Thread:       thread,
				Dyad:         dyad,
				ScoreTotal:   score,
				ScoreDetail:  detail,
				TopicWeights: npc.DebugTopicWeightsForPair(mask, "", fam, sen, tags),
			})
		}
	}
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	_ = json.NewEncoder(w).Encode(resp)
}

// HandleSocialDebugReset 處理 POST /api/debug/npc-social/reset。
func HandleSocialDebugReset(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, `{"error":"method not allowed"}`, http.StatusMethodNotAllowed)
		return
	}
	ResetStats()
	ClearLastChoice()

	resetRumors := r.URL.Query().Get("reset_rumors")
	rumorsReset := false
	if resetRumors == "1" || strings.EqualFold(resetRumors, "true") || strings.EqualFold(resetRumors, "yes") {
		_ = db.ResetNpcRumorSignals()
		rumorsReset = true
	}

	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	_ = json.NewEncoder(w).Encode(map[string]any{
		"ok":           true,
		"message":      "npc social debug stats reset",
		"rumors_reset": rumorsReset,
	})
}
