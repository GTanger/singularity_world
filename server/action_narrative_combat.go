// do_action 敘事：戰鬥戰報（combat.ResolveV2）。
package server

import (
	"strings"

	"singularity_world/combat"
	"singularity_world/db"
	"singularity_world/entity"
)

// buildAttackNarrative 依 combat.ResolveV2 產出戰報（§七 標籤）；subdue=true 為留人（氣血不低於 1）。回傳 winner 供 NPC 反應。
func buildAttackNarrative(roomID string, attacker, defender *entity.Character, subdue bool) (narrative string, winner string, attackerFinalHP, defenderFinalHP int) {
	opt := &combat.CombatOpt{Subdue: subdue}
	if roomID != "" {
		if t := db.TerrainFromRoom(roomID); t != "" {
			opt.Terrain = t
		}
	}
	winner, rawLog, aHP, dHP := combat.ResolveV2(attacker.Vit, attacker.Dex, defender.Vit, defender.Dex, opt)
	aName := attacker.DisplayTitle
	if aName == "" {
		aName = attacker.ID
	}
	dName := defender.DisplayTitle
	if dName == "" {
		dName = defender.ID
	}
	log := strings.ReplaceAll(rawLog, "攻方", "【"+aName+"】")
	log = strings.ReplaceAll(log, "守方", "【"+dName+"】")
	var prefix string
	if subdue {
		prefix = "你對【" + dName + "】出手，意在留人！"
	} else {
		prefix = "你對【" + dName + "】出手，意在送行！"
	}
	var suffix string
	if winner == "attacker" {
		if subdue {
			suffix = "\n你留住了對方。"
		} else {
			suffix = "\n你取得了勝利。"
		}
	} else {
		suffix = "\n你敗下陣來。"
	}
	return prefix + log + suffix, winner, aHP, dHP
}
