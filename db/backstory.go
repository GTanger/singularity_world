// Package db 背版組裝（identity）：Talk 時帶入「我是誰」。
package db

import (
	"strings"

	"singularity_world/store"
)

// BuildIdentity 組裝 NPC 的 identity 字串（1～3 句）：真名、職稱、場所、性格、心境、最近事件。
func BuildIdentity(entityID string) string {
	person := GetNPCPersonDisplayName(entityID)
	if person == "" {
		person = entityID
	}
	occ := GetNPCTitleFromAssignments(entityID)
	result := "你是" + person + "。"
	if store.Default != nil {
		assignments := store.Default.GetAssignmentsForEntity(entityID)
		if len(assignments) > 0 {
			venue := store.Default.GetVenue(assignments[0].VenueID)
			if venue != nil {
				if occ != "" {
					result = "你是" + person + "，職稱是" + occ + "，在" + venue.Name + "工作。"
				} else {
					result = "你是" + person + "，在" + venue.Name + "相關場合活動。"
				}
			}
		}
	}
	p, hasPers := GetPersonalityForEntity(entityID)
	if hasPers {
		result += personalityToSentence(p)
	}
	disp := GetDisposition(entityID)
	if disp > 20 {
		result += "你最近心情不錯。"
	}
	if disp < -20 {
		result += "你最近過得不太好。"
	}
	events := GetRecentEvents(entityID, 3)
	for _, e := range events {
		result += e.Payload + "。"
	}
	if sum := GetNpcSummary(entityID); sum != "" {
		result += "與玩家的最近印象：" + sum
	}
	return result
}

func personalityToSentence(p Personality) string {
	var parts []string
	if p.Boldness > 0.6 {
		parts = append(parts, "你性格大膽、敢衝")
	}
	if p.Boldness < 0.3 {
		parts = append(parts, "你性格謹慎、怕事")
	}
	if p.Orderliness > 0.6 {
		parts = append(parts, "做事守規矩")
	}
	if p.Orderliness < 0.3 {
		parts = append(parts, "不太受規矩約束")
	}
	if p.Sensitivity > 0.6 {
		parts = append(parts, "對人較敏感、口吻較熱絡")
	}
	if p.Sensitivity < 0.3 {
		parts = append(parts, "較冷淡、話少")
	}
	if len(parts) == 0 {
		return ""
	}
	return strings.Join(parts, "，") + "。"
}
