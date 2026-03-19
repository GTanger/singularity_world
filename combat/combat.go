// Package combat 負責戰鬥判定與文字 log，對齊決策 001 同一套規則、戰鬥系統概念—三版融合 §八。
// Resolve 為第一階段簡化；ResolveV2 支援 α/β/γ、地形與 §七 戰報標籤。
package combat

import (
	"fmt"
	"math"
	"math/rand"
	"strings"
)

// CombatOpt 擴充選項：α/β/γ（§2.2）、地形（§2.5）。nil 或零值時以文件預設代入。
type CombatOpt struct {
	Alpha   float64 // 能階係數，未實裝預設 1
	Beta    float64 // 時脈係數，未實裝預設 1
	Gamma   float64 // 相位係數（暴擊/偏轉），未實裝預設 0
	Terrain string  // 地形標籤：""、"silent"、"lush"、"grip"、"chaos"，未實裝預設 1
	// Subdue 留人：任一方 HP 將 ≤0 時改為 1 並立刻結束該場（不觸發死亡路徑）。對齊借物與戰鬥意圖—用語與系統草案。
	Subdue bool
}

func defaultOpt(opt *CombatOpt) *CombatOpt {
	if opt == nil {
		return &CombatOpt{Alpha: 1, Beta: 1, Gamma: 0}
	}
	o := *opt
	if o.Alpha == 0 {
		o.Alpha = 1
	}
	if o.Beta == 0 {
		o.Beta = 1
	}
	return &o
}

// terrainMult 地形對傷害的乘數（§2.5 M_Lush 等）。空或未知為 1。
func terrainMult(terrain string) float64 {
	switch strings.ToLower(strings.TrimSpace(terrain)) {
	case "lush":
		return 1.15 // α 放大
	case "chaos":
		return 1.08 // γ 波動放大，略增
	case "silent", "grip":
		return 1.0
	default:
		return 1.0
	}
}

// damageFirstStage 第一階段公式：D = max(1, round((1+Atk_Vit/10)*(1-Def_Vit/(Def_Vit+20))))。
// 未實裝項以 1 代入；僅用體質與減傷，常數 20 可調。
func damageFirstStage(atkVit, defVit int) int {
	if defVit <= 0 {
		return atkVit
	}
	mit := float64(defVit) / (float64(defVit) + 20)
	raw := (1.0 + float64(atkVit)/10.0) * (1.0 - mit)
	return int(math.Max(1, math.Round(raw)))
}

// damageV2 依 CombatOpt 套用 α 與地形乘數後的單次傷害；opt 為 nil 時等同 damageFirstStage。
func damageV2(atkVit, defVit int, opt *CombatOpt) int {
	base := damageFirstStage(atkVit, defVit)
	if opt == nil {
		return base
	}
	o := defaultOpt(opt)
	d := float64(base) * o.Alpha * terrainMult(o.Terrain)
	return int(math.Max(1, math.Round(d)))
}

// phaseRoll 依 Gamma 判定暴擊（true, false）或偏轉（false, true）；否則（false, false）。未實裝預設 γ=0 不觸發。
func phaseRoll(gamma float64) (crit, deflect bool) {
	if gamma <= 0 {
		return false, false
	}
	r := rand.Float64()
	if r < gamma*0.08 {
		return false, true
	}
	if r < gamma*0.08+gamma*0.12 {
		return true, false
	}
	return false, false
}

// applyPhase 依 γ 判定偏轉（傷害 0）或暴擊（×1.5），回傳有效傷害與戰報標籤（[Conflict:Deflect]／[Conflict:Crit]）。
func applyPhase(baseDmg int, gamma float64) (effectiveDmg int, tag string) {
	crit, deflect := phaseRoll(gamma)
	if deflect {
		return 0, " [Conflict:Deflect]"
	}
	if crit {
		return int(math.Max(1, math.Round(float64(baseDmg) * 1.5))), " [Conflict:Crit]"
	}
	return baseDmg, ""
}

// Resolve 依雙方屬性判定勝負並產出文字 log，並回傳雙方戰鬥結束後的 HP（供呼叫方寫回 DB）。
// 第一階段實作：先手 Dex 高者；每輪傷害依體質與減傷公式；HP 初值 = Vit；任一方 HP≤0 即結束。
// 參數：attackerVit, attackerDex, defenderVit, defenderDex 為體質與靈敏。
// 回傳：winner 為 "attacker" 或 "defender"；log 為戰鬥過程文字；attackerFinalHP, defenderFinalHP 為結束時剩餘 HP（可為 0）。
func Resolve(attackerVit, attackerDex, defenderVit, defenderDex int) (winner, log string, attackerFinalHP, defenderFinalHP int) {
	aHP, dHP := attackerVit, defenderVit
	const maxRounds = 200
	rounds := 0
	log = "戰鬥開始。"
	attackerFirst := attackerDex >= defenderDex
	if attackerFirst {
		log += " 攻方先手。"
	} else {
		log += " 守方先手。"
	}
	for aHP > 0 && dHP > 0 && rounds < maxRounds {
		rounds++
		if attackerFirst {
			dmg := damageFirstStage(attackerVit, defenderVit)
			dHP -= dmg
			log += " 攻方擊中守方，"
			if dHP <= 0 {
				return "attacker", log + " 守方敗。", aHP, dHP
			}
			dmg = damageFirstStage(defenderVit, attackerVit)
			aHP -= dmg
			log += " 守方反擊。"
			if aHP <= 0 {
				return "defender", log + " 攻方敗。", aHP, dHP
			}
		} else {
			dmg := damageFirstStage(defenderVit, attackerVit)
			aHP -= dmg
			log += " 守方擊中攻方，"
			if aHP <= 0 {
				return "defender", log + " 攻方敗。", aHP, dHP
			}
			dmg = damageFirstStage(attackerVit, defenderVit)
			dHP -= dmg
			log += " 攻方反擊。"
			if dHP <= 0 {
				return "attacker", log + " 守方敗。", aHP, dHP
			}
		}
	}
	if aHP <= 0 && dHP <= 0 {
		return "defender", log + " 平手。", aHP, dHP
	}
	if aHP <= 0 {
		return "defender", log + " 攻方敗。", aHP, dHP
	}
	return "attacker", log + " 守方敗。", aHP, dHP
}

// ResolveV2 擴充版：支援 CombatOpt（α/β/γ、地形）與 §七 戰報標籤（[Initiative]、[Action:Skill]、[Result]）。
// opt 為 nil 時行為等同 Resolve，僅 log 帶標籤。回傳同 Resolve。
func ResolveV2(attackerVit, attackerDex, defenderVit, defenderDex int, opt *CombatOpt) (winner, log string, attackerFinalHP, defenderFinalHP int) {
	aHP, dHP := attackerVit, defenderVit
	const maxRounds = 200
	rounds := 0
	log = "戰鬥開始。"
	attackerFirst := attackerDex >= defenderDex
	if attackerFirst {
		log += " [Initiative] 攻方先手。"
	} else {
		log += " [Initiative] 守方先手。"
	}
	o := defaultOpt(opt)
	subdue := opt != nil && opt.Subdue
	for aHP > 0 && dHP > 0 && rounds < maxRounds {
		rounds++
		if attackerFirst {
			dmg, tag := applyPhase(damageV2(attackerVit, defenderVit, opt), o.Gamma)
			dHP -= dmg
			log += fmt.Sprintf(" [Action:Skill] 攻方擊中守方，傷害 %d。%s [Result] 守方剩餘 %d。", dmg, tag, dHP)
			if dHP <= 0 {
				if subdue {
					dHP = 1
					log += " 守方氣竭倒地。 [Result] 攻方留人（制伏）。"
					return "attacker", log, aHP, dHP
				}
				log += " 守方敗。 [Result] 攻方取得了勝利。"
				return "attacker", log, aHP, dHP
			}
			dmg, tag = applyPhase(damageV2(defenderVit, attackerVit, opt), o.Gamma)
			aHP -= dmg
			log += fmt.Sprintf(" 守方反擊。 [Action:Skill] 守方擊中攻方，傷害 %d。%s [Result] 攻方剩餘 %d。", dmg, tag, aHP)
			if aHP <= 0 {
				if subdue {
					aHP = 1
					log += " 攻方不支倒地。 [Result] 守方留人（制伏）。"
					return "defender", log, aHP, dHP
				}
				log += " 攻方敗。 [Result] 守方取得了勝利。"
				return "defender", log, aHP, dHP
			}
		} else {
			dmg, tag := applyPhase(damageV2(defenderVit, attackerVit, opt), o.Gamma)
			aHP -= dmg
			log += fmt.Sprintf(" [Action:Skill] 守方擊中攻方，傷害 %d。%s [Result] 攻方剩餘 %d。", dmg, tag, aHP)
			if aHP <= 0 {
				if subdue {
					aHP = 1
					log += " 攻方不支倒地。 [Result] 守方留人（制伏）。"
					return "defender", log, aHP, dHP
				}
				log += " 攻方敗。 [Result] 守方取得了勝利。"
				return "defender", log, aHP, dHP
			}
			dmg, tag = applyPhase(damageV2(attackerVit, defenderVit, opt), o.Gamma)
			dHP -= dmg
			log += fmt.Sprintf(" 攻方反擊。 [Action:Skill] 攻方擊中守方，傷害 %d。%s [Result] 守方剩餘 %d。", dmg, tag, dHP)
			if dHP <= 0 {
				if subdue {
					dHP = 1
					log += " 守方氣竭倒地。 [Result] 攻方留人（制伏）。"
					return "attacker", log, aHP, dHP
				}
				log += " 守方敗。 [Result] 攻方取得了勝利。"
				return "attacker", log, aHP, dHP
			}
		}
	}
	if aHP <= 0 && dHP <= 0 {
		log += " 平手。 [Result] 雙方敗下陣來。"
		return "defender", log, aHP, dHP
	}
	if aHP <= 0 {
		log += " 攻方敗。 [Result] 守方取得了勝利。"
		return "defender", log, aHP, dHP
	}
	log += " 守方敗。 [Result] 攻方取得了勝利。"
	return "attacker", log, aHP, dHP
}
