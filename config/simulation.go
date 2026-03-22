package config

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sync"
)

// SimulationParams holds main-loop and NPC-social tunables (data/config/simulation.json).
type SimulationParams struct {
	MaxAssignmentsPerVenue int `json:"max_assignments_per_venue"`
	RoomEvent            struct {
		MaxEvents    int   `json:"max_events"`
		TTLSec       int64 `json:"ttl_sec"`
		RumorTTLSec  int64 `json:"rumor_ttl_sec"`
	} `json:"room_event"`
	QualityGateDefaultMaxRunes int `json:"quality_gate_default_max_runes"`
	NpcNpcPairPick             struct {
		ThreadStaleSec               int `json:"thread_stale_sec"`
		ThreadCooldownSec            int `json:"thread_cooldown_sec"`
		ScoreBaseNoise               int `json:"score_base_noise"`
		DyadSentimentPenaltyThreshold int `json:"dyad_sentiment_penalty_threshold"`
		DyadSentimentPenalty         int `json:"dyad_sentiment_penalty"`
		SameVenueScoreBonus          int `json:"same_venue_score_bonus"`
		PairLastTalkPenaltySec       int `json:"pair_last_talk_penalty_sec"`
		PairLastTalkPenalty          int `json:"pair_last_talk_penalty"`
		SameDisplayNamePenalty       int `json:"same_display_name_penalty"`
		FamiliarityScoreDivisor      int `json:"familiarity_score_divisor"`
		SameDyadVenueTagBonus        int `json:"same_dyad_venue_tag_bonus"`
	} `json:"npc_npc_pair_pick"`
	NpcNpcSocial struct {
		RoomRecentPlayerTalkCooldownSec int `json:"room_recent_player_talk_cooldown_sec"`
		BroadcastSkipRecentTalkSec      int `json:"broadcast_skip_recent_talk_sec"`
		DefaultDialogueScoreThreshold   int `json:"default_dialogue_score_threshold"`
		PlayerGossipBiasPercent         int `json:"player_gossip_bias_percent"`
		RumorTopKDefault                int `json:"rumor_top_k_default"`
		RumorTopKPlayerInRoom           int `json:"rumor_top_k_player_in_room"`
		RecentRoomEventsSlidingMax      int `json:"recent_room_events_sliding_max"`
	} `json:"npc_npc_social"`
	DialogueAnchorRumor struct {
		MinAnchorRunes int `json:"min_anchor_runes"`
		TTLSec         int `json:"ttl_sec"`
		Weight         int `json:"weight"`
		SourceScore    int `json:"source_score"`
	} `json:"dialogue_anchor_rumor"`
	DyadUpdate struct {
		FamiliarityDeltaPerExchange int `json:"familiarity_delta_per_exchange"`
		SentimentCap                int `json:"sentiment_cap"`
		QuarrelTagSentimentMax      int `json:"quarrel_tag_sentiment_max"`
		ClearQuarrelSentimentMin    int `json:"clear_quarrel_sentiment_min"`
	} `json:"dyad_update"`
	BrainBeg struct {
		MagnesiumMin          int `json:"magnesium_min"`
		MagnesiumRandExclusive int `json:"magnesium_rand_exclusive"`
	} `json:"brain_beg"`
	EconomyPulse struct {
		GameDayModulo int `json:"game_day_modulo"`
		RumorTTLSec   int `json:"rumor_ttl_sec"`
	} `json:"economy_pulse"`
	SpawnRumorTTLSec    int `json:"spawn_rumor_ttl_sec"`
	JobMatchRumorTTLSec int `json:"job_match_rumor_ttl_sec"`
	RumorDecayIntervalSec int `json:"rumor_decay_interval_sec"`
	RumorDigestIntervalMin int `json:"rumor_digest_interval_min"`
	Idle                   struct {
		FirstTriggerMin  int `json:"first_trigger_min"`
		FirstTriggerSpan int `json:"first_trigger_span"`
		IntervalMin      int `json:"interval_min"`
		IntervalSpan     int `json:"interval_span"`
	} `json:"idle"`
	RandomNpcDialogueTicks struct {
		InitialMin              int `json:"initial_min"`
		InitialSpan             int `json:"initial_span"`
		ResetMinWithPlayer      int `json:"reset_min_with_player"`
		ResetSpanWithPlayer     int `json:"reset_span_with_player"`
	} `json:"random_npc_dialogue_ticks"`
	JobMatchingIntervalSec       int `json:"job_matching_interval_sec"`
	TravelTickInterval           int `json:"travel_tick_interval"`
	WanderRollMax                int `json:"wander_roll_max"`
	MicroInteractionChancePercent int `json:"micro_interaction_chance_percent"`
	DispositionTimeOfDay       struct {
		DawnHourStart    int `json:"dawn_hour_start"`
		DawnHourEnd      int `json:"dawn_hour_end"`
		LateNightHourEnd int `json:"late_night_hour_end"`
		DawnDelta        int `json:"dawn_delta"`
		LateNightDelta   int `json:"late_night_delta"`
	} `json:"disposition_time_of_day"`
}

const defaultSimulationPath = "data/config/simulation.json"

var (
	simMu       sync.RWMutex
	simulation  SimulationParams
	simLoaded   bool
	simLoadPath = defaultSimulationPath
)

// SetSimulationPathForTest overrides the JSON path (before LoadSimulation).
func SetSimulationPathForTest(p string) {
	simMu.Lock()
	defer simMu.Unlock()
	simLoadPath = p
	simLoaded = false
	simulation = SimulationParams{}
}

func defaultSimulation() SimulationParams {
	var s SimulationParams
	s.MaxAssignmentsPerVenue = 2
	s.RoomEvent.MaxEvents = 5
	s.RoomEvent.TTLSec = 120
	s.RoomEvent.RumorTTLSec = 1800
	s.QualityGateDefaultMaxRunes = 52
	s.NpcNpcPairPick.ThreadStaleSec = 90
	s.NpcNpcPairPick.ThreadCooldownSec = 300
	s.NpcNpcPairPick.ScoreBaseNoise = 4
	s.NpcNpcPairPick.DyadSentimentPenaltyThreshold = -30
	s.NpcNpcPairPick.DyadSentimentPenalty = 2
	s.NpcNpcPairPick.SameVenueScoreBonus = 3
	s.NpcNpcPairPick.PairLastTalkPenaltySec = 120
	s.NpcNpcPairPick.PairLastTalkPenalty = 20
	s.NpcNpcPairPick.SameDisplayNamePenalty = 15
	s.NpcNpcPairPick.FamiliarityScoreDivisor = 20
	s.NpcNpcPairPick.SameDyadVenueTagBonus = 2
	s.NpcNpcSocial.RoomRecentPlayerTalkCooldownSec = 60
	s.NpcNpcSocial.BroadcastSkipRecentTalkSec = 15
	s.NpcNpcSocial.DefaultDialogueScoreThreshold = 35
	s.NpcNpcSocial.PlayerGossipBiasPercent = 40
	s.NpcNpcSocial.RumorTopKDefault = 2
	s.NpcNpcSocial.RumorTopKPlayerInRoom = 3
	s.NpcNpcSocial.RecentRoomEventsSlidingMax = 3
	s.DialogueAnchorRumor.MinAnchorRunes = 4
	s.DialogueAnchorRumor.TTLSec = 3600
	s.DialogueAnchorRumor.Weight = 2
	s.DialogueAnchorRumor.SourceScore = 1
	s.DyadUpdate.FamiliarityDeltaPerExchange = 2
	s.DyadUpdate.SentimentCap = 100
	s.DyadUpdate.QuarrelTagSentimentMax = -30
	s.DyadUpdate.ClearQuarrelSentimentMin = 25
	s.BrainBeg.MagnesiumMin = 3
	s.BrainBeg.MagnesiumRandExclusive = 8
	s.EconomyPulse.GameDayModulo = 3
	s.EconomyPulse.RumorTTLSec = 5400
	s.SpawnRumorTTLSec = 3600
	s.JobMatchRumorTTLSec = 7200
	s.RumorDecayIntervalSec = 30
	s.RumorDigestIntervalMin = 2
	s.Idle.FirstTriggerMin = 25
	s.Idle.FirstTriggerSpan = 35
	s.Idle.IntervalMin = 60
	s.Idle.IntervalSpan = 60
	s.RandomNpcDialogueTicks.InitialMin = 80
	s.RandomNpcDialogueTicks.InitialSpan = 40
	s.RandomNpcDialogueTicks.ResetMinWithPlayer = 115
	s.RandomNpcDialogueTicks.ResetSpanWithPlayer = 75
	s.JobMatchingIntervalSec = 30
	s.TravelTickInterval = 30
	s.WanderRollMax = 10
	s.MicroInteractionChancePercent = 15
	s.DispositionTimeOfDay.DawnHourStart = 6
	s.DispositionTimeOfDay.DawnHourEnd = 9
	s.DispositionTimeOfDay.LateNightHourEnd = 5
	s.DispositionTimeOfDay.DawnDelta = 1
	s.DispositionTimeOfDay.LateNightDelta = -1
	return s
}

// LoadSimulation reads simulation JSON; merges onto built-in defaults (missing keys keep defaults).
func LoadSimulation(customPath string) error {
	simMu.Lock()
	defer simMu.Unlock()
	p := simLoadPath
	if customPath != "" {
		p = customPath
	}
	s := defaultSimulation()
	raw, err := os.ReadFile(p)
	if err != nil {
		if raw2, err2 := os.ReadFile(filepath.Join("..", p)); err2 == nil {
			raw, err = raw2, nil
		}
	}
	if err != nil {
		simulation = s
		simLoaded = true
		return fmt.Errorf("config: simulation %q: %w (using built-in defaults)", p, err)
	}
	if err := json.Unmarshal(raw, &s); err != nil {
		return fmt.Errorf("config: simulation parse %q: %w", p, err)
	}
	simulation = s
	simLoaded = true
	return nil
}

// Sim returns loaded tunables (built-in defaults if LoadSimulation was never called or file missing).
func Sim() SimulationParams {
	simMu.RLock()
	defer simMu.RUnlock()
	if !simLoaded {
		return defaultSimulation()
	}
	return simulation
}
