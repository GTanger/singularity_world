// Package gametext loads client copy, narrative templates, and rule word lists from data/config/gametext.json.
package gametext

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
)

const defaultPath = "data/config/gametext.json"

var (
	mu     sync.RWMutex
	loaded bool
	root   rootJSON
	path   = defaultPath
)

type rootJSON struct {
	ClientErrors       map[string]string   `json:"client_errors"`
	BrainArrival       map[string]string   `json:"brain_arrival"`
	DyadRelation       dyadRelationJSON    `json:"dyad_relation"`
	TimeOfDay          []timeBandJSON      `json:"time_of_day"`
	DyadTags           map[string]string   `json:"dyad_tags"`
	TopicIDs           map[string]string   `json:"topic_ids"`
	NpcSocial          npcSocialJSON       `json:"npc_social"`
	RuntimeFmt         runtimeFmtJSON      `json:"runtime_fmt"`
	BrainEffects       brainEffectsJSON    `json:"brain_effects"`
	EconomyRumors      []string            `json:"economy_rumor_lines"`
	DialogueScorer     dialogueScorerJSON  `json:"dialogue_scorer"`
	Sanitize           sanitizeJSON        `json:"sanitize"`
	QualityGate        qualityGateJSON     `json:"quality_gate"`
	NarrationHints     string              `json:"narration_dialogue_hints"`
	AnchorNegators     []string            `json:"anchor_negators"`
	Sentiment          sentimentJSON       `json:"sentiment"`
	RawEventTrimPrefix []string            `json:"raw_event_trim_prefixes"`
	DefaultNPCTitle    string              `json:"default_npc_title"`
	OccupationDefault  string              `json:"occupation_default_waiter"`
	OccClerk           string              `json:"occupation_clerk"`
	AdminWipeOK        string              `json:"admin_wipe_entities_response"`
	DefaultManagerTitle string             `json:"default_manager_title"`
	DebugPairTagSameVenue string           `json:"debug_pair_tag_same_venue"`
	DialogueMarkers    string              `json:"dialogue_markers"`
}

type dyadRelationJSON struct {
	NilDyad       string `json:"nil_dyad"`
	SentimentLow  string `json:"sentiment_low"`
	SentimentHigh string `json:"sentiment_high"`
	FamHigh       string `json:"familiarity_high"`
	FamMid        string `json:"familiarity_mid"`
	FamLow        string `json:"familiarity_low"`
}

type timeBandJSON struct {
	MinHour int    `json:"min_hour"`
	MaxHour int    `json:"max_hour"`
	Label   string `json:"label"`
}

type npcSocialJSON struct {
	RumorPrefix           string `json:"rumor_prefix"`
	PlayerPresentLine     string `json:"player_present_context_line"`
	KnownAnchorsPrefix    string `json:"known_anchors_prefix"`
	ToneSeedPrefix        string `json:"tone_seed_prefix"`
	PlayerGossipSuffix    string `json:"player_gossip_topic_suffix"`
	RumorEchoLineFmt      string `json:"rumor_echo_line_fmt"`
	SummaryFmt            string `json:"summary_fmt"`
	ArchivalWithFmt       string `json:"archival_with_fmt"`
	BroadcastFmt          string `json:"broadcast_fmt"`
	NewcomerRumor         string `json:"newcomer_rumor"`
	JobMatchRumorFmt      string `json:"job_match_rumor_fmt"`
}

type runtimeFmtJSON struct {
	ShiftLeaveFmt            string `json:"shift_leave_fmt"`
	ShiftLeaveToShopFmt      string `json:"shift_leave_to_shop_fmt"`
	ShiftArrive              string `json:"shift_arrive"`
	TravelLeaveFmt           string `json:"travel_leave_fmt"`
	TravelArriveFmt          string `json:"travel_arrive_fmt"`
	PushLeaveFmt             string `json:"push_leave_fmt"`
	PushArriveFmt            string `json:"push_arrive_fmt"`
	PushShiftLeaveDetailFmt  string `json:"push_shift_leave_detail_fmt"`
	ShiftLeaveWork           string `json:"shift_leave_work"`
	WanderArriveHome         string `json:"wander_arrive_home"`
}

type brainEffectsJSON struct {
	BegEventFmt    string `json:"beg_event_fmt"`
	GatherEventFmt string `json:"gather_event_fmt"`
	HiredEventFmt  string `json:"hired_event_fmt"`
	TalkEvent      string `json:"talk_social"`
	TradeNoStock   string `json:"trade_no_stock"`
}

type dialogueScorerJSON struct {
	Poison               []string `json:"poison"`
	IntimateWhenCold     []string `json:"intimate_when_cold"`
	FormalWhenWarm       []string `json:"formal_when_warm"`
	PickupSignals        []string `json:"pickup_signals"`
	QuestionMarkers      []string `json:"question_markers"`
	WastelandMarkers     []string `json:"wasteland_markers"`
	RelationColdMarkers  []string `json:"relation_cold_markers"`
	RelationWarmMarkers  []string `json:"relation_warm_markers"`
}

type sanitizeJSON struct {
	ThinkTokens        []string `json:"think_tokens"`
	CodeFence          string   `json:"code_fence"`
	SelfCorrectMarkers []string `json:"self_correct_markers"`
	AssistantMarkers   []string `json:"assistant_markers"`
}

type qualityGateJSON struct {
	BadMeta []string `json:"bad_meta"`
}

type sentimentJSON struct {
	ShiftOrChatSubstrings []string `json:"shift_or_chat_substrings"`
	PrySubstring          string   `json:"pry_substring"`
	OutsideWildSubstrings []string `json:"outside_wild_substrings"`
	WeatherSubstring      string   `json:"weather_substring"`
	NegWords              []string `json:"neg_words"`
	PosWords              []string `json:"pos_words"`
}

// SetPathForTest overrides the JSON path; call before Load, clears cache.
func SetPathForTest(p string) {
	mu.Lock()
	defer mu.Unlock()
	path = p
	loaded = false
	root = rootJSON{}
}

// Load reads gametext JSON. Safe to call multiple times (idempotent).
func Load(customPath string) error {
	mu.Lock()
	defer mu.Unlock()
	p := path
	if customPath != "" {
		p = customPath
	}
	raw, err := os.ReadFile(p)
	if err != nil {
		if raw2, err2 := os.ReadFile(filepath.Join("..", p)); err2 == nil {
			raw, err = raw2, nil
		}
	}
	if err != nil {
		return fmt.Errorf("gametext: read %q: %w", p, err)
	}
	var r rootJSON
	if err := json.Unmarshal(raw, &r); err != nil {
		return fmt.Errorf("gametext: parse: %w", err)
	}
	root = r
	loaded = true
	return nil
}

// Client returns a client-facing error string by key; falls back to key if missing.
func Client(key string) string {
	mu.RLock()
	defer mu.RUnlock()
	if s := root.ClientErrors[key]; s != "" {
		return s
	}
	return key
}

// Clientf formats a client-facing message from client_errors[key] with fmt.Sprintf.
func Clientf(key string, args ...any) string {
	mu.RLock()
	tpl := root.ClientErrors[key]
	mu.RUnlock()
	if tpl == "" {
		return key
	}
	return fmt.Sprintf(tpl, args...)
}

// BrainArrival formats narrative for intent key (e.g. "beg", "gather").
func BrainArrival(intentKey, npcName string) string {
	mu.RLock()
	tpl := root.BrainArrival[intentKey]
	mu.RUnlock()
	if tpl == "" {
		return ""
	}
	return fmt.Sprintf(tpl, npcName)
}

func DyadRelationHint(nilDyad bool, sentiment, familiarity int) string {
	mu.RLock()
	d := root.DyadRelation
	mu.RUnlock()
	if nilDyad {
		return d.NilDyad
	}
	if sentiment <= -40 {
		return d.SentimentLow
	}
	if sentiment >= 40 {
		return d.SentimentHigh
	}
	switch {
	case familiarity >= 70:
		return d.FamHigh
	case familiarity >= 35:
		return d.FamMid
	default:
		return d.FamLow
	}
}

func TimeOfDayLabel(hour int) string {
	mu.RLock()
	bands := root.TimeOfDay
	mu.RUnlock()
	for _, b := range bands {
		if hour >= b.MinHour && hour < b.MaxHour {
			return b.Label
		}
	}
	if len(bands) > 0 {
		return bands[len(bands)-1].Label
	}
	return ""
}

func DyadTag(key string) string {
	mu.RLock()
	s := root.DyadTags[key]
	mu.RUnlock()
	return s
}

func TopicID(key string) string {
	mu.RLock()
	s := root.TopicIDs[key]
	mu.RUnlock()
	return s
}

func NpcSocial() npcSocialJSON {
	mu.RLock()
	v := root.NpcSocial
	mu.RUnlock()
	return v
}

func RuntimeFmt() runtimeFmtJSON {
	mu.RLock()
	v := root.RuntimeFmt
	mu.RUnlock()
	return v
}

func BrainEffects() brainEffectsJSON {
	mu.RLock()
	v := root.BrainEffects
	mu.RUnlock()
	return v
}

func EconomyRumorLines() []string {
	mu.RLock()
	v := root.EconomyRumors
	mu.RUnlock()
	return v
}

func DialogueScorer() dialogueScorerJSON {
	mu.RLock()
	v := root.DialogueScorer
	mu.RUnlock()
	return v
}

func Sanitize() sanitizeJSON {
	mu.RLock()
	v := root.Sanitize
	mu.RUnlock()
	return v
}

func QualityGateBadMeta() []string {
	mu.RLock()
	v := root.QualityGate.BadMeta
	mu.RUnlock()
	return v
}

func NarrationDialogueHints() string {
	mu.RLock()
	s := root.NarrationHints
	mu.RUnlock()
	return s
}

func AnchorNegators() []string {
	mu.RLock()
	v := root.AnchorNegators
	mu.RUnlock()
	return v
}

func Sentiment() sentimentJSON {
	mu.RLock()
	v := root.Sentiment
	mu.RUnlock()
	return v
}

func RawEventTrimPrefixes() []string {
	mu.RLock()
	v := root.RawEventTrimPrefix
	mu.RUnlock()
	return v
}

func DefaultNPCTitle() string {
	mu.RLock()
	s := root.DefaultNPCTitle
	mu.RUnlock()
	return s
}

func OccupationWaiter() string {
	mu.RLock()
	s := root.OccupationDefault
	mu.RUnlock()
	return s
}

func OccupationClerk() string {
	mu.RLock()
	s := root.OccClerk
	mu.RUnlock()
	return s
}

func AdminWipeResponse() string {
	mu.RLock()
	s := root.AdminWipeOK
	mu.RUnlock()
	return s
}

func DefaultManagerTitle() string {
	mu.RLock()
	s := root.DefaultManagerTitle
	mu.RUnlock()
	return s
}

func DebugPairTagSameVenue() string {
	mu.RLock()
	s := root.DebugPairTagSameVenue
	mu.RUnlock()
	return s
}

// DialogueMarkers is a string of runes; if line narration check finds none of these, line may be narration.
func DialogueMarkers() string {
	mu.RLock()
	s := root.DialogueMarkers
	mu.RUnlock()
	return s
}

// MustLoad exits the process if JSON is missing (call from main).
func MustLoad() {
	if err := Load(""); err != nil {
		fmt.Fprintf(os.Stderr, "gametext: %v\n", err)
		os.Exit(1)
	}
}

// Loaded reports whether Load succeeded at least once.
func Loaded() bool {
	mu.RLock()
	defer mu.RUnlock()
	return loaded
}

// Reload for tests.
func Reload() error {
	mu.Lock()
	loaded = false
	root = rootJSON{}
	mu.Unlock()
	return Load("")
}

// TopicContainsAny returns true if hint contains any of substrings.
func TopicContainsAny(hint string, subs []string) bool {
	for _, s := range subs {
		if s != "" && strings.Contains(hint, s) {
			return true
		}
	}
	return false
}
