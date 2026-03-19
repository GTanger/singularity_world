// Package db：10.19 輕量 NPC 行為反應（依性格一句話），對齊借物／留人／送行。
package db

import (
	"database/sql"
	"math/rand"
)

// NpcBehaviorReactionLine 依情境回傳一句 NPC 視角的敘事尾綴；無則空字串。
// kind：subdue_victim（被留人放倒）、slay_victim（將死）、lost_subdue（打贏了留人的玩家）、borrow_ok、borrow_caught。
func NpcBehaviorReactionLine(database *sql.DB, npcID, playerID string, kind string) string {
	_ = playerID // 預留：日後依「對該玩家記憶」變體台詞
	var p *Personality
	if target, err := GetEntity(database, npcID); err == nil && target != nil && target.SoulSeed != nil {
		pp := ExpandSoulSeedToPersonality(*target.SoulSeed)
		p = &pp
	}
	// 無性格時用中性台詞
	bold, sens, ord := 0.5, 0.5, 0.5
	if p != nil {
		bold, sens, ord = p.Boldness, p.Sensitivity, p.Orderliness
	}
	r := rand.Float64()

	switch kind {
	case "subdue_victim":
		if ord > 0.65 {
			return "【" + shortName(database, npcID) + "】咬牙道：「今日算你留手……」"
		}
		if bold > 0.65 {
			return "【" + shortName(database, npcID) + "】雖倒地，仍怒目瞪你。"
		}
		if sens > 0.65 && r < 0.5 {
			return "【" + shortName(database, npcID) + "】喘息著，一時說不出話。"
		}
		return "【" + shortName(database, npcID) + "】倒地不起，暫難起身。"

	case "slay_victim":
		return "" // 死亡由區域 narrate 與戰報處理，此處不重複

	case "lost_subdue":
		if bold > 0.6 {
			return "【" + shortName(database, npcID) + "】哼了一聲：「還站得住？」"
		}
		return "【" + shortName(database, npcID) + "】收勢而立。"

	case "borrow_ok":
		if ord > 0.55 && r < 0.4 {
			return "【" + shortName(database, npcID) + "】似乎尚未察覺身上少了一物。"
		}
		return ""

	case "borrow_caught":
		if bold > 0.6 {
			return "【" + shortName(database, npcID) + "】厲聲喝道：「住手！你這算哪門子的借！」"
		}
		if ord > 0.6 {
			return "【" + shortName(database, npcID) + "】皺眉盯著你：「不告而取，豈是君子？」"
		}
		return "【" + shortName(database, npcID) + "】一把按住你的手：「想借？先問過我。」"
	default:
		return ""
	}
}

func shortName(database *sql.DB, entityID string) string {
	e, err := GetEntity(database, entityID)
	if err != nil || e == nil {
		return entityID
	}
	if e.Kind == "npc" {
		return GetNPCTitle(database, entityID)
	}
	if e.DisplayTitle != "" {
		return e.DisplayTitle
	}
	return e.ID
}
