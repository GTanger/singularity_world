// Package combat 負責戰鬥判定與文字 log，對齊決策 001 同一套規則、第一版可做清單 §1.7。
package combat

// Resolve 依雙方屬性判定勝負並產出文字 log；第一版可簡化為屬性對撞或固定公式。
// 參數：attackerVit, attackerDex, defenderVit, defenderDex 為體質與靈敏（或擴充參數）。
// 回傳：winner 為 "attacker" 或 "defender"；log 為戰鬥過程文字。無副作用（實際扣血/死亡由呼叫方寫 DB）。
func Resolve(attackerVit, attackerDex, defenderVit, defenderDex int) (winner string, log string) {
	// 第一版：簡化為靈敏高者先手、體質當血量，先歸零者敗
	aHP := attackerVit
	dHP := defenderVit
	log = "戰鬥開始。"
	if attackerDex >= defenderDex {
		dHP--
		log += " 攻方先手。"
		for aHP > 0 && dHP > 0 {
			aHP--
			log += " 守方反擊。"
			if aHP <= 0 {
				return "defender", log + " 攻方敗。"
			}
			dHP--
		}
		if dHP <= 0 {
			return "attacker", log + " 守方敗。"
		}
	} else {
		aHP--
		log += " 守方先手。"
		for aHP > 0 && dHP > 0 {
			dHP--
			log += " 攻方反擊。"
			if dHP <= 0 {
				return "attacker", log + " 守方敗。"
			}
			aHP--
		}
		if aHP <= 0 {
			return "defender", log + " 攻方敗。"
		}
	}
	return "defender", log + " 平手。"
}
