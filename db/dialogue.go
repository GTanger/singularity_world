// Package db 對話模板載入、抽句與佔位符替換，對齊 docs/implementation/對話池—從固定句到組合句.md。
// 支援：擴詞表與佔位符、池子混用、情境權重、微變體、片段組合、子槽位。
package db

import (
	"database/sql"
	"encoding/json"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"strings"
	"sync"
)

// DialogueFile 單一職業對話檔結構（greet / talk 必備；idle、trade_announce、talk_fragments 可選）。
type DialogueFile struct {
	Greet struct {
		Lines []string `json:"lines"`
	} `json:"greet"`
	Talk struct {
		Lines []string `json:"lines"`
	} `json:"talk"`
	// TalkFragments 片段組合：每條為 [][]string，每段從子陣列抽一項再拼接。可與 talk.lines 並存。
	TalkFragments [][][]string `json:"talk_fragments,omitempty"`
	Idle          map[string][]string `json:"idle,omitempty"`
	TradeAnnounce struct {
		Buy  []string `json:"buy,omitempty"`
		Sell []string `json:"sell,omitempty"`
	} `json:"trade_announce,omitempty"`
}

// DialogueSlots 佔位符詞表；含子槽位 verb_manner + verb_action、weather、topic。
type DialogueSlots struct {
	Verb        []string `json:"verb"`
	VerbManner  []string `json:"verb_manner"`
	VerbAction  []string `json:"verb_action"`
	Mood        []string `json:"mood"`
	Time        []string `json:"time"`
	Thing       []string `json:"thing"`
	Weather     []string `json:"weather"`
	Topic       []string `json:"topic"`
}

var (
	dialogueMu    sync.RWMutex
	dialogueCache = make(map[string]*DialogueFile)
	slotsOnce     sync.Once
	slotsCache    *DialogueSlots
)

const defaultTemplatesBase = "data/templates"

// 池子混用：talk 時抽到 greet 的機率約 12%、idle 約 8%，其餘為 talk。
const probGreet = 0.12
const probIdle = 0.08

// 片段組合：有 talk_fragments 時，約 25% 從片段抽一句。
const probFragment = 0.25

// LoadDialogueSlots 載入 dialogue_slots.json，僅執行一次。
func LoadDialogueSlots(path string) *DialogueSlots {
	slotsOnce.Do(func() {
		data, err := os.ReadFile(path)
		if err != nil {
			log.Printf("[dialogue] slots %s: %v", path, err)
			slotsCache = &DialogueSlots{
				Verb: []string{"嘆了口氣", "點點頭"}, Mood: []string{"隨口", "正色"},
				Time: []string{"清晨", "正午", "傍晚", "夜裡"}, Thing: []string{"這兒的規矩"},
				Weather: []string{"晴"}, Topic: []string{"這兒的規矩"},
			}
			return
		}
		var s DialogueSlots
		if err := json.Unmarshal(data, &s); err != nil {
			log.Printf("[dialogue] slots parse: %v", err)
			slotsCache = &DialogueSlots{Verb: []string{"嘆了口氣"}, Mood: []string{"隨口"}, Time: []string{"此時"}, Thing: []string{""}}
			return
		}
		slotsCache = &s
	})
	return slotsCache
}

// LoadDialogue 依 basePath + dialogueFile 載入對話檔並快取。
func LoadDialogue(basePath, dialogueFile string) (*DialogueFile, error) {
	key := filepath.Join(basePath, dialogueFile)
	dialogueMu.RLock()
	if d, ok := dialogueCache[key]; ok {
		dialogueMu.RUnlock()
		return d, nil
	}
	dialogueMu.RUnlock()

	data, err := os.ReadFile(key)
	if err != nil {
		return nil, err
	}
	var d DialogueFile
	if err := json.Unmarshal(data, &d); err != nil {
		return nil, err
	}
	dialogueMu.Lock()
	dialogueCache[key] = &d
	dialogueMu.Unlock()
	return &d, nil
}

// PlaceholderCtx 佔位符替換用情境；空字串表示由詞表隨機抽。
type PlaceholderCtx struct {
	Name       string
	RoomName   string
	TimeLabel  string
	Mood       string
	Verb       string
	Thing      string
	Goods      string
	Weather    string
	Topic      string
}

// timeLabelToIdleKey 遊戲時段對應 idle 的 key。
func timeLabelToIdleKey(timeLabel string) string {
	switch timeLabel {
	case "清晨":
		return "morning"
	case "正午":
		return "noon"
	case "傍晚":
		return "evening"
	case "夜裡":
		return "night"
	}
	return "noon"
}

// pick 從 list 依 rng 抽一項；空則回傳空字串。
func pick(list []string, rng *rand.Rand) string {
	if len(list) == 0 {
		return ""
	}
	return list[rng.Intn(len(list))]
}

// FillPlaceholders 將模板中的佔位符替換；空欄位從 slots 抽。子槽位：{verb} 有機率為 verb_manner+verb_action。
func FillPlaceholders(s string, ctx PlaceholderCtx, slots *DialogueSlots, seed int64) string {
	rng := rand.New(rand.NewSource(seed))
	replace := func(key, val string) {
		if val == "" && slots != nil {
			switch key {
			case "mood":
				val = pick(slots.Mood, rng)
			case "verb":
				if len(slots.VerbManner) > 0 && len(slots.VerbAction) > 0 && rng.Float32() < 0.4 {
					val = pick(slots.VerbManner, rng) + pick(slots.VerbAction, rng)
				} else {
					val = pick(slots.Verb, rng)
				}
			case "time":
				val = pick(slots.Time, rng)
			case "thing":
				val = pick(slots.Thing, rng)
			case "weather":
				val = pick(slots.Weather, rng)
			case "topic":
				val = pick(slots.Topic, rng)
			}
		}
		s = strings.ReplaceAll(s, "{"+key+"}", val)
	}
	replace("name", ctx.Name)
	replace("room", ctx.RoomName)
	replace("time", ctx.TimeLabel)
	replace("mood", ctx.Mood)
	replace("verb", ctx.Verb)
	replace("thing", ctx.Thing)
	replace("goods", ctx.Goods)
	replace("weather", ctx.Weather)
	replace("topic", ctx.Topic)
	return s
}

// ApplyMicroVariants 填完佔位符後做輕量口語變體（依 seed 可重現）：引號內句尾「。」→「啦。」、隨機「這兒」↔「那兒」。
func ApplyMicroVariants(s string, seed int64) string {
	rng := rand.New(rand.NewSource(seed))
	if rng.Float32() < 0.15 {
		if idx := strings.LastIndex(s, "。"); idx >= 0 {
			s = s[:idx] + "啦。" + s[idx+len("。"):]
		}
	}
	if rng.Float32() < 0.2 {
		if rng.Float32() < 0.5 {
			s = strings.Replace(s, "這兒", "那兒", 1)
		} else {
			s = strings.Replace(s, "那兒", "這兒", 1)
		}
	}
	return s
}

// PickLineWeighted 從 lines 中依 seed 與 personality 選一句；依 timeLabel、roomName 對含關鍵字的句加權（情境權重）。
func PickLineWeighted(lines []string, seed int64, personality *Personality, timeLabel, roomName string) string {
	if len(lines) == 0 {
		return ""
	}
	rng := rand.New(rand.NewSource(seed))
	weights := make([]float64, len(lines))
	for i := range weights {
		weights[i] = 1.0
		line := lines[i]
		if timeLabel != "" {
			if strings.Contains(line, timeLabel) || (timeLabel == "夜裡" && strings.Contains(line, "夜")) || (timeLabel == "清晨" && strings.Contains(line, "晨")) {
				weights[i] *= 1.6
			}
		}
		if roomName != "" && len(roomName) >= 2 && strings.Contains(line, roomName) {
			weights[i] *= 1.5
		}
	}
	var sum float64
	for _, w := range weights {
		sum += w
	}
	if sum <= 0 {
		sum = 1
	}
	x := rng.Float64() * sum
	for i, w := range weights {
		x -= w
		if x <= 0 {
			if personality != nil {
				shift := int(personality.Boldness * float64(len(lines)/2))
				if personality.Sensitivity > 0.6 {
					shift += len(lines) / 4
				} else if personality.Sensitivity < 0.3 {
					shift -= len(lines) / 4
				}
				i = (i + shift + len(lines)) % len(lines)
			}
			return lines[i]
		}
	}
	idx := rng.Intn(len(lines))
	if personality != nil {
		shift := int(personality.Boldness * float64(len(lines)/2))
		if personality.Sensitivity > 0.6 {
			shift += len(lines) / 4
		} else if personality.Sensitivity < 0.3 {
			shift -= len(lines) / 4
		}
		idx = (idx + shift + len(lines)) % len(lines)
	}
	return lines[idx]
}

// assembleFragment 從一組片段 [][]string 各抽一項拼接。
func assembleFragment(segments [][]string, rng *rand.Rand) string {
	var b strings.Builder
	for _, seg := range segments {
		if len(seg) > 0 {
			b.WriteString(seg[rng.Intn(len(seg))])
		}
	}
	return b.String()
}

// PickFromDialogue 依職業取得對話檔、抽一句（talk 或 greet）、填佔位符、微變體後回傳。
// 支援池子混用（小機率從 greet/idle 抽）、片段組合（有 talk_fragments 時）、情境權重。
func PickFromDialogue(database *sql.DB, entityID, roomID, occupationID, key string, personality *Personality, goods, roomName, timeLabel, weather string) string {
	occs := LoadOccupations(filepath.Join(defaultTemplatesBase, "occupations.json"))
	occ, ok := occs[occupationID]
	if !ok || occ.DialogueFile == "" {
		return ""
	}
	d, err := LoadDialogue(defaultTemplatesBase, occ.DialogueFile)
	if err != nil || d == nil {
		return ""
	}
	slots := LoadDialogueSlots(filepath.Join(defaultTemplatesBase, "dialogue_slots.json"))
	seed := int64(0)
	for _, r := range entityID {
		seed = seed*31 + int64(r)
	}
	seed += int64(len(roomID))<<8 + int64(len(key))<<16

	var line string
	var poolKey string = key

	// 池子混用：主 key 為 talk 時，有機率改從 greet 或 idle 抽
	if key == "talk" {
		rng := rand.New(rand.NewSource(seed + 99))
		p := rng.Float64()
		if p < probGreet && len(d.Greet.Lines) > 0 {
			poolKey = "greet"
		} else if p < probGreet+probIdle && d.Idle != nil {
			idleKey := timeLabelToIdleKey(timeLabel)
			if idles, ok := d.Idle[idleKey]; ok && len(idles) > 0 {
				line = PickLineWeighted(idles, seed+101, personality, timeLabel, roomName)
				poolKey = ""
			}
		}
	}

	if line == "" {
		if poolKey == "greet" {
			line = PickLineWeighted(d.Greet.Lines, seed+1, personality, timeLabel, roomName)
		} else if poolKey == "talk" {
			// 片段組合：有 talk_fragments 且隨機命中時，組裝一句
			if len(d.TalkFragments) > 0 && rand.New(rand.NewSource(seed+77)).Float32() < probFragment {
				rng := rand.New(rand.NewSource(seed + 17))
				segments := d.TalkFragments[rng.Intn(len(d.TalkFragments))]
				line = assembleFragment(segments, rng)
			}
			if line == "" && len(d.Talk.Lines) > 0 {
				line = PickLineWeighted(d.Talk.Lines, seed, personality, timeLabel, roomName)
			}
		}
	}

	if line == "" {
		return ""
	}

	ctx := PlaceholderCtx{
		Name:      entityID,
		RoomName:  roomName,
		TimeLabel: timeLabel,
		Goods:     goods,
		Weather:   weather,
	}
	filled := FillPlaceholders(line, ctx, slots, seed+1)
	return ApplyMicroVariants(filled, seed+31)
}

// PickStyleExamples 從該 NPC 的對話池抽 n 句作為口吻範例（不填佔位符），供 CallAITalk 當 style 範例。
func PickStyleExamples(database *sql.DB, entityID string, n int) []string {
	assignments, _ := GetAssignmentsForEntity(database, entityID)
	if len(assignments) == 0 {
		return nil
	}
	occID := assignments[0].OccupationID
	occs := LoadOccupations(filepath.Join(defaultTemplatesBase, "occupations.json"))
	occ, ok := occs[occID]
	if !ok || occ.DialogueFile == "" {
		return nil
	}
	d, err := LoadDialogue(defaultTemplatesBase, occ.DialogueFile)
	if err != nil || d == nil {
		return nil
	}
	var lines []string
	lines = append(lines, d.Talk.Lines...)
	lines = append(lines, d.Greet.Lines...)
	if len(lines) == 0 {
		return nil
	}
	var p *Personality
	if pers, has := GetPersonalityForEntity(database, entityID); has {
		p = &pers
	}
	seed := int64(0)
	for _, r := range entityID {
		seed = seed*31 + int64(r)
	}
	var out []string
	for i := 0; i < n; i++ {
		line := PickLineWeighted(lines, seed+int64(i)*7, p, "", "")
		if line != "" {
			out = append(out, line)
		}
	}
	return out
}
