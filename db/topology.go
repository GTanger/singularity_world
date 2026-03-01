// Package db 本檔提供 361 拓撲邊權由 SoulSeed 展開，供除錯與日後內視／繞路使用。規格：361拓撲系統規格 §6.1。
package db

import (
	"math/rand"
)

// 361 拓撲邊權常數（與規格 §6.1.0、cmd/soulseed_demo 一致）
const (
	RawWeightMin   = 0.1
	RawWeightMax   = 1.0
	TotalCostNorm  = 10000 // CONST_TOTAL_TOPOLOGY_COST
	NumTopologyEdges = 760
)

// ExpandSoulSeedToTopologyCosts 由 soul_seed 決定性產生 760 條邊的阻力（Cost）。
// 與 ExpandSoulSeedToBaseStats 共用同一 RNG 序：前 3 次為三軸，第 4～763 次為 760 條邊原始權重，再歸一化為總和 10000。
// 回傳長度 760，索引 0～19 為型 A（N000→N001..N020），即前 3 條為 N000→N001、N000→N002、N000→N003。
func ExpandSoulSeedToTopologyCosts(seed int64) []float64 {
	rng := rand.New(rand.NewSource(seed))
	// 消耗前 3 次 RNG（與三軸一致，確保與 base stats 同序）
	_, _, _ = rng.Float64(), rng.Float64(), rng.Float64()

	raw := make([]float64, NumTopologyEdges)
	var sumRaw float64
	for k := 0; k < NumTopologyEdges; k++ {
		u := rng.Float64()
		raw[k] = RawWeightMin + u*(RawWeightMax-RawWeightMin)
		sumRaw += raw[k]
	}
	costs := make([]float64, NumTopologyEdges)
	for k := 0; k < NumTopologyEdges; k++ {
		costs[k] = (raw[k] / sumRaw) * TotalCostNorm
	}
	return costs
}
