// Package combat 戰鬥模擬測試：現有 Resolve 與文件建議第一階段公式並行模擬。
package combat

import (
	"fmt"
	"math"
	"strings"
	"testing"
)

// 模擬用角色簡稱
type simFighter struct {
	name string
	Vit  int
	Dex  int
}

// resolveWithFormula 依文件「建議第一階段」公式模擬一場：先手 Dex 高者，每輪傷害
// D = max(1, round((1+Atk_Vit/10)*(1-Def_Vit/(Def_Vit+20))))，HP 初值 = Vit，任一方 HP≤0 即結束。
// 回傳 winner 名稱、回合數、log 片段。
func resolveWithFormula(a, d simFighter) (winner string, rounds int, log []string) {
	rounds = 0
	aHP, dHP := a.Vit, d.Vit
	damage := func(atkVit, defVit int) int {
		if defVit <= 0 {
			return atkVit
		}
		mit := float64(defVit) / (float64(defVit) + 20)
		raw := (1.0 + float64(atkVit)/10.0) * (1.0 - mit)
		return int(math.Max(1, math.Round(raw)))
	}

	aFirst := a.Dex >= d.Dex
	log = append(log, fmt.Sprintf("【%s】Vit=%d Dex=%d vs 【%s】Vit=%d Dex=%d", a.name, a.Vit, a.Dex, d.name, d.Vit, d.Dex))
	if aFirst {
		log = append(log, "先手：攻方")
	} else {
		log = append(log, "先手：守方")
	}

	for aHP > 0 && dHP > 0 {
		rounds++
		if aFirst {
			dmg := damage(a.Vit, d.Vit)
			dHP -= dmg
			log = append(log, fmt.Sprintf("  第%d輪 攻方打守方 %d 點 → 守方HP=%d", rounds, dmg, dHP))
			if dHP <= 0 {
				return a.name, rounds, log
			}
			dmg = damage(d.Vit, a.Vit)
			aHP -= dmg
			log = append(log, fmt.Sprintf("  第%d輪 守方反擊 %d 點 → 攻方HP=%d", rounds, dmg, aHP))
			if aHP <= 0 {
				return d.name, rounds, log
			}
		} else {
			dmg := damage(d.Vit, a.Vit)
			aHP -= dmg
			log = append(log, fmt.Sprintf("  第%d輪 守方先打 %d 點 → 攻方HP=%d", rounds, dmg, aHP))
			if aHP <= 0 {
				return d.name, rounds, log
			}
			dmg = damage(a.Vit, d.Vit)
			dHP -= dmg
			log = append(log, fmt.Sprintf("  第%d輪 攻方反擊 %d 點 → 守方HP=%d", rounds, dmg, dHP))
			if dHP <= 0 {
				return a.name, rounds, log
			}
		}
		if rounds >= 100 {
			log = append(log, "（超過 100 回合強制結束）")
			if aHP >= dHP {
				return a.name, rounds, log
			}
			return d.name, rounds, log
		}
	}
	if aHP <= 0 && dHP <= 0 {
		return "平手", rounds, log
	}
	if aHP <= 0 {
		return d.name, rounds, log
	}
	return a.name, rounds, log
}

func TestCombat_CurrentResolve_Simulation(t *testing.T) {
	// 現有 Resolve：靈敏高者先手，體質當血量，輪流扣 1 直到一方歸零
	scenarios := []struct {
		name string
		aVit, aDex int
		dVit, dDex int
	}{
		{"均衡對均衡", 5, 5, 5, 5},
		{"攻方先手且體厚", 8, 7, 4, 3},
		{"守方先手且體厚", 4, 3, 8, 7},
		{"攻方高靈敏薄血", 2, 10, 6, 2},
		{"守方高靈敏薄血", 6, 2, 2, 10},
		{"雙高體", 10, 3, 10, 4},
	}
	for _, s := range scenarios {
		t.Run(s.name, func(t *testing.T) {
			winner, log, _, _ := Resolve(s.aVit, s.aDex, s.dVit, s.dDex)
			t.Logf("攻方 Vit=%d Dex=%d vs 守方 Vit=%d Dex=%d → 勝方: %s", s.aVit, s.aDex, s.dVit, s.dDex, winner)
			t.Logf("log: %s", strings.TrimSpace(log))
		})
	}
}

func TestCombat_FormulaSimulation(t *testing.T) {
	// 依文件第一階段公式：D = max(1, (1+Vit/10)*(1-DefVit/(DefVit+20)))，先手 Dex 高者
	pairs := []struct {
		a, d simFighter
	}{
		{simFighter{"甲", 5, 5}, simFighter{"乙", 5, 5}},
		{simFighter{"攻", 8, 7}, simFighter{"守", 4, 3}},
		{simFighter{"守", 4, 3}, simFighter{"攻", 8, 7}},
		{simFighter{"快刀", 2, 10}, simFighter{"鐵壁", 6, 2}},
		{simFighter{"鐵壁", 10, 2}, simFighter{"快刀", 2, 10}},
		{simFighter{"A", 10, 4}, simFighter{"B", 10, 3}},
	}
	for i, p := range pairs {
		winner, rounds, log := resolveWithFormula(p.a, p.d)
		t.Run(fmt.Sprintf("場景%d_%s_vs_%s", i+1, p.a.name, p.d.name), func(t *testing.T) {
			for _, line := range log {
				t.Log(line)
			}
			t.Logf("→ 勝方: %s，共 %d 回合", winner, rounds)
		})
	}
}

func TestCombat_FormulaVsCurrent_Compare(t *testing.T) {
	// 同一組參數：比較現有 Resolve 與公式模擬的勝方是否一致（可能因公式不同而不同，僅記錄）
	a, d := simFighter{"攻", 6, 5}, simFighter{"守", 5, 6}
	winnerF, rounds, _ := resolveWithFormula(a, d)
	winnerR, logR, _, _ := Resolve(a.Vit, a.Dex, d.Vit, d.Dex)
	t.Logf("公式模擬: %s 勝，%d 回合", winnerF, rounds)
	t.Logf("現有 Resolve: %s 勝，log: %s", winnerR, strings.TrimSpace(logR))
	// 不強制一致，僅供比對
}

func TestResolveV2_NilOpt_SameAsResolve(t *testing.T) {
	// ResolveV2(..., nil) 應與 Resolve 同勝方、同最終 HP，且 log 含 §七 標籤
	aVit, aDex := 8, 7
	dVit, dDex := 4, 3
	w1, _, a1, d1 := Resolve(aVit, aDex, dVit, dDex)
	w2, log2, a2, d2 := ResolveV2(aVit, aDex, dVit, dDex, nil)
	if w1 != w2 || a1 != a2 || d1 != d2 {
		t.Errorf("ResolveV2(nil) 應與 Resolve 一致: Resolve winner=%s aHP=%d dHP=%d, V2 winner=%s aHP=%d dHP=%d", w1, a1, d1, w2, a2, d2)
	}
	if !strings.Contains(log2, "[Initiative]") || !strings.Contains(log2, "[Action:Skill]") || !strings.Contains(log2, "[Result]") {
		t.Errorf("ResolveV2 log 應含 §七 標籤: %s", log2)
	}
	t.Logf("ResolveV2(nil) log: %s", strings.TrimSpace(log2))
}

func TestResolveV2_AlphaTerrain(t *testing.T) {
	// Alpha>1、Terrain lush 時傷害乘數大，戰報含 §七 標籤與傷害/剩餘數字
	opt := &CombatOpt{Alpha: 1.3, Terrain: "lush"}
	winner, log2, a2, d2 := ResolveV2(5, 5, 5, 5, opt)
	if winner != "attacker" && winner != "defender" {
		t.Errorf("ResolveV2(opt) winner=%s", winner)
	}
	if !strings.Contains(log2, "傷害 ") || !strings.Contains(log2, "剩餘 ") {
		t.Errorf("戰報應含傷害與剩餘: %s", log2)
	}
	t.Logf("ResolveV2(Alpha=1.3,lush) winner=%s aHP=%d dHP=%d", winner, a2, d2)
}
