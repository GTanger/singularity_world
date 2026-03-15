// Package db 背版組裝（identity）：Talk 時帶入「我是誰」。
package db

import (
	"database/sql"
	"strings"

	"singularity_world/store"
)

// BuildIdentity 組裝 NPC 的 identity 字串（1～3 句）：職稱、場所、性格、心境、最近事件。
func BuildIdentity(database *sql.DB, entityID string) string {
	title := GetNPCTitle(database, entityID)
	if title == "" {
		title = entityID
	}
	result := "你是" + title + "。"
	if store.Default != nil {
		assignments := store.Default.GetAssignmentsForEntity(entityID)
		if len(assignments) > 0 {
			venue := store.Default.GetVenue(assignments[0].VenueID)
			if venue != nil {
				result = "你是" + title + "，在" + venue.Name + "工作。"
			}
		}
	}
	p, hasPers := GetPersonalityForEntity(database, entityID)
	if hasPers {
		result += personalityToSentence(p)
	}
	disp := GetDisposition(database, entityID)
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
	if len(parts) == 0 {
		return ""
	}
	return strings.Join(parts, "，") + "。"
}
