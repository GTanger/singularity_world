// do_action 敘事：Look 打量文字。
package server

import (
	"fmt"
	"strings"

	"singularity_world/db"
	"singularity_world/entity"
)

func buildLookNarrative(target *entity.Character) string {
	name := target.DisplayTitle
	if name == "" {
		name = target.ID
	}
	pronoun := "他"
	if target.Gender == "F" {
		pronoun = "她"
	}
	var physique string
	switch {
	case target.Vit >= 20:
		physique = "體格異常魁梧"
	case target.Vit >= 15:
		physique = "體格健壯"
	case target.Vit >= 10:
		physique = "身材勻稱"
	default:
		physique = "身形消瘦"
	}
	var agility string
	switch {
	case target.Dex >= 20:
		agility = "舉止間透著驚人的敏捷"
	case target.Dex >= 15:
		agility = "動作輕靈"
	case target.Dex >= 10:
		agility = "步履平穩"
	default:
		agility = "行動略顯遲緩"
	}
	var qiPresence string
	switch {
	case target.Qi >= 20:
		qiPresence = "，周身隱隱有氣勁流轉"
	case target.Qi >= 15:
		qiPresence = "，氣息沉穩"
	case target.Qi >= 10:
		qiPresence = ""
	default:
		qiPresence = "，氣息微弱"
	}
	desc := fmt.Sprintf("你仔細打量了【%s】。%s%s，%s%s。", name, pronoun, physique, agility, qiPresence)
	if target.EquipmentSlots != "" {
		names, _ := db.GetItemNames(target.EquipmentSlots)
		if len(names) > 0 {
			pieces := make([]string, 0, 3)
			for _, n := range names {
				if len(pieces) >= 3 {
					break
				}
				pieces = append(pieces, n)
			}
			desc += " 身上穿戴著" + strings.Join(pieces, "、") + "。"
		}
	}
	return desc
}
