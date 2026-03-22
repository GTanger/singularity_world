package ai

import (
	"strings"

	"singularity_world/gametext"
)

// DialogueScoreDetail scores one round of NPC dialogue (0–100) for debug and telemetry.
type DialogueScoreDetail struct {
	Length       int    `json:"length"`
	Anchor       int    `json:"anchor"`
	Relation     int    `json:"relation"`
	Repeat       int    `json:"repeat"`
	Diversity    int    `json:"diversity"`
	DialogueFeel int    `json:"dialogue_feel"`
	Identity     int    `json:"identity"`
	Narration    int    `json:"narration"`
	ToneDrift    int    `json:"tone_drift,omitempty"`
	Total        int    `json:"total"`
	KilledBy     string `json:"killed_by,omitempty"`
}

// ScoreNpcDialogue scores one exchange between two NPC lines using gametext rules.
func ScoreNpcDialogue(lineA, lineB string, roomName string, recentEvents []string,
	topicHint, relationHint, npcNpcMemory string,
	speakerName, listenerName string,
	recentArchivalLines []string,
) DialogueScoreDetail {
	d := DialogueScoreDetail{}
	combined := lineA + lineB
	sc := gametext.DialogueScorer()

	for _, p := range sc.Poison {
		if p == "" {
			continue
		}
		if strings.Contains(strings.ToLower(combined), strings.ToLower(p)) {
			d.KilledBy = "poison:" + p
			d.Total = 0
			return d
		}
	}

	lenA, lenB := len([]rune(lineA)), len([]rune(lineB))
	switch {
	case lenA >= 4 && lenA <= 28 && lenB >= 4 && lenB <= 28:
		d.Length = 15
	case lenA <= 2 || lenB <= 2:
		d.Length = 0
	case lenA > 40 || lenB > 40:
		d.Length = 3
	default:
		d.Length = 10
	}

	anchorPool := []string{roomName, topicHint, npcNpcMemory}
	anchorPool = append(anchorPool, recentEvents...)
	hits := 0
	for _, anchor := range anchorPool {
		anchor = strings.TrimSpace(anchor)
		if anchor == "" {
			continue
		}
		runes := []rune(anchor)
		for i := 0; i+1 < len(runes); i++ {
			bigram := string(runes[i : i+2])
			if strings.Contains(combined, bigram) {
				hits++
				break
			}
		}
	}
	d.Anchor = hits * 5
	if d.Anchor > 20 {
		d.Anchor = 20
	}

	d.Relation = 7
	cold := false
	for _, sub := range sc.RelationColdMarkers {
		if sub != "" && strings.Contains(relationHint, sub) {
			cold = true
			break
		}
	}
	if cold {
		for _, w := range sc.IntimateWhenCold {
			if w != "" && strings.Contains(combined, w) {
				d.Relation = 0
				break
			}
		}
		if d.Relation != 0 {
			d.Relation = 10
		}
	} else {
		warm := false
		for _, sub := range sc.RelationWarmMarkers {
			if sub != "" && strings.Contains(relationHint, sub) {
				warm = true
				break
			}
		}
		if warm {
			for _, w := range sc.FormalWhenWarm {
				if w != "" && strings.Contains(combined, w) {
					d.Relation = 3
					break
				}
			}
			if d.Relation != 3 {
				d.Relation = 10
			}
		}
	}

	d.Repeat = 0
	for _, prev := range recentArchivalLines {
		prev = strings.TrimSpace(prev)
		if prev == "" {
			continue
		}
		if prev == lineA || prev == lineB {
			d.Repeat = -25
			break
		}
		if runeOverlapRatio(prev, lineA) > 0.85 || runeOverlapRatio(prev, lineB) > 0.85 {
			d.Repeat = -15
			break
		}
	}

	var bigrams []string
	for _, line := range []string{lineA, lineB} {
		r := []rune(line)
		for i := 0; i+1 < len(r); i++ {
			bigrams = append(bigrams, string(r[i:i+2]))
		}
	}
	if len(bigrams) > 0 {
		unique := make(map[string]bool, len(bigrams))
		for _, bg := range bigrams {
			unique[bg] = true
		}
		ratio := float64(len(unique)) / float64(len(bigrams))
		d.Diversity = int(ratio * 10)
	}

	for _, sig := range sc.PickupSignals {
		if sig != "" && strings.Contains(lineB, sig) {
			d.DialogueFeel += 8
			break
		}
	}
	for _, q := range sc.QuestionMarkers {
		if q != "" && strings.Contains(lineA, q) {
			d.DialogueFeel += 7
			break
		}
	}
	if d.DialogueFeel > 15 {
		d.DialogueFeel = 15
	}

	d.Identity = 10
	if speakerName != "" && strings.Contains(lineA, speakerName) {
		d.Identity -= 5
	}
	if listenerName != "" && strings.Contains(lineB, listenerName) {
		d.Identity -= 5
	}

	dm := gametext.DialogueMarkers()
	hasMarker := false
	for _, m := range []rune(dm) {
		if strings.ContainsRune(combined, m) {
			hasMarker = true
			break
		}
	}
	if !hasMarker {
		d.Narration = -20
	}

	d.ToneDrift = wastelandTonePenalty(combined, sc.WastelandMarkers)

	d.Total = d.Length + d.Anchor + d.Relation + d.Repeat +
		d.Diversity + d.DialogueFeel + d.Identity + d.Narration + d.ToneDrift
	if d.Total < 0 {
		d.Total = 0
	}
	if d.Total > 100 {
		d.Total = 100
	}
	return d
}

func wastelandTonePenalty(combined string, markers []string) int {
	if combined == "" {
		return 0
	}
	for _, m := range markers {
		if m != "" && strings.Contains(combined, m) {
			return -10
		}
	}
	return 0
}

func runeOverlapRatio(a, b string) float64 {
	ra, rb := []rune(a), []rune(b)
	if len(ra) == 0 || len(rb) == 0 {
		return 0
	}
	set := make(map[rune]bool, len(ra))
	for _, r := range ra {
		set[r] = true
	}
	match := 0
	for _, r := range rb {
		if set[r] {
			match++
		}
	}
	bigger := len(ra)
	if len(rb) > bigger {
		bigger = len(rb)
	}
	return float64(match) / float64(bigger)
}

// RawEventsForDialogueScore strips display prefixes from room event strings for scoring.
func RawEventsForDialogueScore(recentEvents []string) []string {
	raw := make([]string, 0, len(recentEvents))
	prefixes := gametext.RawEventTrimPrefixes()
	for _, ev := range recentEvents {
		ev = strings.TrimSpace(ev)
		for _, p := range prefixes {
			if p != "" {
				ev = strings.TrimPrefix(ev, p)
			}
		}
		ev = strings.TrimSpace(ev)
		if ev != "" {
			raw = append(raw, ev)
		}
	}
	return raw
}
