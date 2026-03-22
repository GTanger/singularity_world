package ai

import (
	"strings"

	"singularity_world/gametext"
)

// StripNpcLineSpeakerPrefix removes leading "Name:" or "Name：" style prefixes.
func StripNpcLineSpeakerPrefix(line string, names ...string) string {
	line = strings.TrimSpace(line)
	for _, name := range names {
		if name == "" {
			continue
		}
		if strings.HasPrefix(line, name+":") {
			line = strings.TrimSpace(line[len(name)+1:])
			break
		}
		if strings.HasPrefix(line, name+"：") {
			line = strings.TrimSpace(strings.TrimPrefix(line, name+"："))
			break
		}
	}
	return line
}

// SanitizeNPCDialogueLine returns cleaned dialogue or empty if the line should be dropped.
func SanitizeNPCDialogueLine(line string) string {
	s := strings.TrimSpace(line)
	if s == "" {
		return ""
	}
	low := strings.ToLower(s)
	san := gametext.Sanitize()
	for _, t := range san.ThinkTokens {
		if t != "" && strings.Contains(low, strings.ToLower(t)) {
			return ""
		}
	}
	if san.CodeFence != "" && strings.Contains(s, san.CodeFence) {
		return ""
	}
	for _, m := range san.SelfCorrectMarkers {
		if m != "" && strings.Contains(s, m) {
			return ""
		}
	}
	for _, m := range san.AssistantMarkers {
		if m != "" && strings.Contains(s, m) {
			return ""
		}
	}
	gopen, gclose := string(rune(0x300c)), string(rune(0x300d))
	for len(s) >= len(gopen)+len(gclose) && strings.HasPrefix(s, gopen) && strings.HasSuffix(s, gclose) {
		s = strings.TrimSpace(s[len(gopen) : len(s)-len(gclose)])
	}
	for strings.HasSuffix(s, gclose+gclose) {
		s = strings.TrimSuffix(s, gclose)
	}
	return strings.TrimSpace(s)
}
