package npcnpc

import (
	"strings"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/gametext"
	"singularity_world/npc"
	"singularity_world/store"
)

// TopicMaskForRoom 依房間與時段建構 NPC 主題遮罩。
func TopicMaskForRoom(roomID string, hour int) npc.NpcTopicMask {
	venueIDs, _ := db.GetVenueIDsForRoom(roomID)
	var roomTags []string
	if r, err := db.GetRoom(roomID); err == nil && r != nil {
		roomTags = r.Tags
	}
	return npc.NpcTopicMask{
		IsWorkVenue:  len(venueIDs) > 0,
		IsNightTime:  hour < 5 || hour >= 21,
		HasRoomEvent: len(RecentRoomEvents(roomID, 1)) > 0,
		RoomTags:     roomTags,
	}
}

// NpcPairSameVenue 若兩 NPC 有共用工作場所則為 true。
func NpcPairSameVenue(idA, idB string) bool {
	asA, _ := db.GetAssignmentsForEntity(idA)
	asB, _ := db.GetAssignmentsForEntity(idB)
	if len(asA) == 0 || len(asB) == 0 {
		return false
	}
	vid := make(map[string]bool, len(asA))
	for _, a := range asA {
		if a.VenueID != "" {
			vid[a.VenueID] = true
		}
	}
	for _, b := range asB {
		if b.VenueID != "" && vid[b.VenueID] {
			return true
		}
	}
	return false
}

// HasTag 檢查 tags 是否含 t。
func HasTag(tags []string, t string) bool {
	for _, v := range tags {
		if v == t {
			return true
		}
	}
	return false
}

func addTag(tags []string, t string) []string {
	if t == "" || HasTag(tags, t) {
		return tags
	}
	return append(tags, t)
}

func removeTag(tags []string, t string) []string {
	if t == "" || len(tags) == 0 {
		return tags
	}
	out := make([]string, 0, len(tags))
	for _, v := range tags {
		if v != t {
			out = append(out, v)
		}
	}
	return out
}

func dyadRelationHint(d *store.NpcDyad) string {
	if d == nil {
		return gametext.DyadRelationHint(true, 0, 0)
	}
	return gametext.DyadRelationHint(false, d.Sentiment, d.Familiarity)
}

// SentimentDeltaFromDialogue 依台詞與主題推估關係情緒增量。
func SentimentDeltaFromDialogue(topicHint, lineA, lineB string, familiarity int, topicID string) int {
	delta := 0
	sen := gametext.Sentiment()
	if gametext.TopicContainsAny(topicHint, sen.ShiftOrChatSubstrings) {
		delta++
	}
	if sen.PrySubstring != "" && strings.Contains(topicHint, sen.PrySubstring) && familiarity < 20 {
		delta--
	}
	if topicID == gametext.TopicID("outside_rumor") || topicID == gametext.TopicID("weather") {
		delta++
	} else if topicID == "" {
		if gametext.TopicContainsAny(topicHint, sen.OutsideWildSubstrings) {
			delta++
		} else if sen.WeatherSubstring != "" && strings.Contains(topicHint, sen.WeatherSubstring) {
			delta++
		}
	}
	joined := lineA + " " + lineB
	for _, w := range sen.NegWords {
		if w != "" && strings.Contains(joined, w) {
			delta -= 2
			break
		}
	}
	for _, w := range sen.PosWords {
		if w != "" && strings.Contains(joined, w) {
			delta++
			break
		}
	}
	return delta
}

func qualityGateNpcLine(line string, maxRunes int) (bool, string) {
	if maxRunes <= 0 {
		maxRunes = config.Sim().QualityGateDefaultMaxRunes
		if maxRunes <= 0 {
			maxRunes = 52
		}
	}
	s := strings.TrimSpace(line)
	if s == "" {
		return false, "empty"
	}
	if strings.ContainsRune(s, '\ufffd') {
		return false, "unicode_garbage"
	}
	t := strings.TrimSpace(s)
	if t == "{" || t == "}" || (strings.HasPrefix(t, "{") && len([]rune(t)) <= 4) {
		return false, "json_residue"
	}
	low := strings.ToLower(s)
	for _, leak := range []string{"npc_", "entity_", "room_"} {
		if strings.Contains(low, leak) {
			return false, "leaked_id"
		}
	}
	if strings.Contains(low, "street") || strings.Contains(low, "zone") {
		return false, "leaked_id"
	}
	if strings.Contains(s, "---") || strings.Contains(s, "```") || strings.Contains(s, "\n\n") {
		return false, "markdown_residue"
	}
	if len([]rune(s)) <= 2 {
		return false, "too_short"
	}
	if len([]rune(s)) > maxRunes {
		return false, "too_long"
	}
	for _, b := range gametext.QualityGateBadMeta() {
		if b != "" && strings.Contains(s, b) {
			return false, "meta_text"
		}
	}
	gopen, gclose := string(rune(0x300c)), string(rune(0x300d))
	if strings.Count(s, gopen) > 2 || strings.Count(s, gclose) > 2 {
		return false, "nested_quotes"
	}
	if hasLongRepeat(s) {
		return false, "repetition"
	}
	if looksLikeNpcNpcNarrationLine(s) {
		return false, "suspicious_narration"
	}
	return true, ""
}

func looksLikeNpcNpcNarrationLine(s string) bool {
	r := []rune(s)
	if len(r) < 8 || !strings.HasSuffix(s, string(rune(0x3002))) {
		return false
	}
	hints := gametext.DialogueMarkers()
	for _, ch := range r {
		if strings.ContainsRune(hints, ch) {
			return false
		}
	}
	return true
}

func shouldSkipNpcNpcSummaryUpdate(oldSummary, newSummary, lineA, lineB string) bool {
	oldSummary = strings.TrimSpace(oldSummary)
	newSummary = strings.TrimSpace(newSummary)
	if oldSummary == "" {
		return false
	}
	if newSummary == oldSummary {
		return true
	}
	if db.RuneLCSSimilarity(oldSummary, newSummary) >= 0.7 {
		return true
	}
	la, lb := strings.TrimSpace(lineA), strings.TrimSpace(lineB)
	if la != "" && db.RuneLCSSimilarity(oldSummary, la) >= 0.7 {
		return true
	}
	if lb != "" && db.RuneLCSSimilarity(oldSummary, lb) >= 0.7 {
		return true
	}
	return false
}

func dedupeSocialContextLines(lines []string) []string {
	seen := make(map[string]bool, len(lines))
	out := make([]string, 0, len(lines))
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		key := strings.ToLower(line)
		if seen[key] {
			continue
		}
		seen[key] = true
		out = append(out, line)
	}
	return out
}

func hasLongRepeat(s string) bool {
	r := []rune(s)
	if len(r) < 8 {
		return false
	}
	for i := 0; i+3 < len(r); i++ {
		ch := r[i]
		if ch == ' ' {
			continue
		}
		j := i + 1
		for j < len(r) && r[j] == ch {
			j++
		}
		if j-i >= 4 {
			return true
		}
	}
	return false
}

func mergeThreadAnchors(old []string, fresh []string) []string {
	seen := map[string]bool{}
	out := make([]string, 0, 4)
	for _, v := range old {
		v = strings.TrimSpace(v)
		if v == "" || seen[v] {
			continue
		}
		seen[v] = true
		out = append(out, v)
	}
	for _, v := range fresh {
		v = strings.TrimSpace(v)
		if v == "" || seen[v] {
			continue
		}
		seen[v] = true
		out = append(out, v)
	}
	if len(out) > 4 {
		out = out[len(out)-4:]
	}
	return out
}

func anchorConsistencyCheck(existing []string, lineA, lineB string) (bool, string) {
	if len(existing) == 0 {
		return true, ""
	}
	joined := strings.TrimSpace(lineA + " " + lineB)
	if joined == "" {
		return true, ""
	}
	negators := gametext.AnchorNegators()
	for _, a := range existing {
		a = strings.TrimSpace(a)
		if a == "" {
			continue
		}
		runes := []rune(a)
		for i := 0; i+1 < len(runes); i++ {
			k := string(runes[i : i+2])
			if !strings.Contains(joined, k) {
				continue
			}
			for _, n := range negators {
				if strings.Contains(joined, n+k) || strings.Contains(joined, k+n) {
					return false, "anchor negated: " + k
				}
			}
		}
	}
	return true, ""
}
