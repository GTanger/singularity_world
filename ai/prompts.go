package ai

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
	"strings"
	"sync"
)

const defaultLLMPromptsPath = "data/templates/llm_prompts.json"

type llmPromptsJSON struct {
	WorldPhenomenaCognition string `json:"world_phenomena_cognition"`
	PlayerNPC               struct {
		PersonaLine            string `json:"persona_line"`
		RoomContextFmt         string `json:"room_context_fmt"`
		SpaceConsistencyRule   string `json:"space_consistency_rule"`
		BehaviorRules          string `json:"behavior_rules"`
		TraditionalChineseRule string `json:"traditional_chinese_rule"`
		IdentityPrefix         string `json:"identity_prefix"`
		SensitivityPrefix      string `json:"sensitivity_prefix"`
		StyleExamplesHeader    string `json:"style_examples_header"`
		StyleExampleBullet     string `json:"style_example_bullet"`
		UserPlainFmt           string `json:"user_plain_fmt"`
		UserWithMemoryFmt      string `json:"user_with_memory_fmt"`
	} `json:"player_npc"`
	NpcToNpc struct {
		TaskIntro              string   `json:"task_intro"`
		ToneAfterWorld         string   `json:"tone_after_world"`
		RulesBlock             string   `json:"rules_block"`
		PlayerPresentNote      string   `json:"player_present_note"`
		UserMessage            string   `json:"user_message"`
		SpeakerLabel           string   `json:"speaker_label"`
		ListenerLabel          string   `json:"listener_label"`
		IdentityPrefix         string   `json:"identity_prefix"`
		ContextLineFmt         string   `json:"context_line_fmt"`
		RoomAtmospherePrefix   string   `json:"room_atmosphere_prefix"`
		RoomTagsPrefix         string   `json:"room_tags_prefix"`
		RecentEventsHeader     string   `json:"recent_events_header"`
		RecentEventBullet      string   `json:"recent_event_bullet"`
		TopicLineFmt           string   `json:"topic_line_fmt"`
		RelationLineFmt        string   `json:"relation_line_fmt"`
		MemoryLineFmt          string   `json:"memory_line_fmt"`
		RoomDescTruncSuffix    string   `json:"room_desc_trunc_suffix"`
		FallbackListenerReply  string   `json:"fallback_listener_reply"`
		CollapseSaySeparators  []string `json:"collapse_say_separators"`
	} `json:"npc_to_npc"`
	Filters struct {
		MetaBadMarkers []string `json:"meta_bad_markers"`
	} `json:"filters"`
}

var (
	llmPromptsMu     sync.RWMutex
	llmPromptsLoaded bool
	cachedLLMPrompts llmPromptsJSON
	llmPromptsPath   = defaultLLMPromptsPath
)

func SetLLMPromptsPathForTest(path string) {
	llmPromptsMu.Lock()
	defer llmPromptsMu.Unlock()
	llmPromptsPath = path
	llmPromptsLoaded = false
	cachedLLMPrompts = llmPromptsJSON{}
}

func loadLLMPrompts() error {
	llmPromptsMu.Lock()
	defer llmPromptsMu.Unlock()
	if llmPromptsLoaded {
		return nil
	}
	raw, err := os.ReadFile(llmPromptsPath)
	if err != nil {
		alt := filepath.Join("..", llmPromptsPath)
		if raw2, err2 := os.ReadFile(alt); err2 == nil {
			raw = raw2
			err = nil
		}
	}
	if err != nil {
		log.Printf("[ai] llm_prompts.json: %v (path=%q)", err, llmPromptsPath)
		llmPromptsLoaded = true
		return err
	}
	var root llmPromptsJSON
	if err := json.Unmarshal(raw, &root); err != nil {
		log.Printf("[ai] llm_prompts.json parse: %v", err)
		llmPromptsLoaded = true
		return err
	}
	cachedLLMPrompts = root
	llmPromptsLoaded = true
	return nil
}

func prompts() *llmPromptsJSON {
	_ = loadLLMPrompts()
	llmPromptsMu.RLock()
	defer llmPromptsMu.RUnlock()
	return &cachedLLMPrompts
}

func WorldPhenomenaCognitionPrompt() string {
	s := strings.TrimSpace(prompts().WorldPhenomenaCognition)
	if s == "" {
		return ""
	}
	if !strings.HasSuffix(s, "\n") {
		s += "\n"
	}
	return s
}

func playerNPCPersonaLine() string     { return prompts().PlayerNPC.PersonaLine }
func playerNPCRoomContextFmt() string { return prompts().PlayerNPC.RoomContextFmt }
func playerNPCSpaceConsistencyRule() string {
	return prompts().PlayerNPC.SpaceConsistencyRule
}
func playerNPCBehaviorRules() string { return prompts().PlayerNPC.BehaviorRules }
func playerNPCTraditionalRule() string { return prompts().PlayerNPC.TraditionalChineseRule }
func playerNPCIdentityPrefix() string  { return prompts().PlayerNPC.IdentityPrefix }
func playerNPCSensitivityPrefix() string {
	return prompts().PlayerNPC.SensitivityPrefix
}
func playerNPCStyleExamplesHeader() string { return prompts().PlayerNPC.StyleExamplesHeader }
func playerNPCStyleExampleBullet() string  { return prompts().PlayerNPC.StyleExampleBullet }
func playerNPCUserPlainFmt() string        { return prompts().PlayerNPC.UserPlainFmt }
func playerNPCUserWithMemoryFmt() string   { return prompts().PlayerNPC.UserWithMemoryFmt }

func npcToNpcTaskIntro() string         { return prompts().NpcToNpc.TaskIntro }
func npcToNpcToneAfterWorld() string    { return prompts().NpcToNpc.ToneAfterWorld }
func npcToNpcRulesBlock() string        { return prompts().NpcToNpc.RulesBlock }
func npcToNpcPlayerPresentNote() string { return prompts().NpcToNpc.PlayerPresentNote }
func npcToNpcUserMessage() string       { return prompts().NpcToNpc.UserMessage }
func npcToNpcSpeakerLabel() string      { return prompts().NpcToNpc.SpeakerLabel }
func npcToNpcListenerLabel() string     { return prompts().NpcToNpc.ListenerLabel }
func npcToNpcIdentityPrefix() string    { return prompts().NpcToNpc.IdentityPrefix }
func npcToNpcContextLineFmt() string    { return prompts().NpcToNpc.ContextLineFmt }
func npcToNpcRoomAtmospherePrefix() string {
	return prompts().NpcToNpc.RoomAtmospherePrefix
}
func npcToNpcRoomTagsPrefix() string      { return prompts().NpcToNpc.RoomTagsPrefix }
func npcToNpcRecentEventsHeader() string  { return prompts().NpcToNpc.RecentEventsHeader }
func npcToNpcRecentEventBullet() string   { return prompts().NpcToNpc.RecentEventBullet }
func npcToNpcTopicLineFmt() string        { return prompts().NpcToNpc.TopicLineFmt }
func npcToNpcRelationLineFmt() string     { return prompts().NpcToNpc.RelationLineFmt }
func npcToNpcMemoryLineFmt() string       { return prompts().NpcToNpc.MemoryLineFmt }
func npcToNpcRoomDescTruncSuffix() string { return prompts().NpcToNpc.RoomDescTruncSuffix }
func npcToNpcFallbackListenerReply() string {
	return prompts().NpcToNpc.FallbackListenerReply
}
func npcToNpcCollapseSaySeparators() []string {
	return prompts().NpcToNpc.CollapseSaySeparators
}

func metaBadMarkers() []string {
	return prompts().Filters.MetaBadMarkers
}
