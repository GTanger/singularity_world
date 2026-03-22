// 模擬創角時生成 soul_seed，並依規格展開三軸與 760 條邊權。
// 規格：人物屬性彙整 §2.0、361拓撲系統規格 §6.1。
package main

import (
	"encoding/binary"
	"flag"
	"fmt"
	"math"
	"math/rand"

	cryptorand "crypto/rand"
)

var seedFlag = flag.Int64("seed", 0, "可選：固定 soul_seed 以便重現同一角色展開")

const (
	// 三軸區間（人物屬性彙整）
	ampMin, ampMax = 0.1, 3.0
	freqMin, freqMax = 0.5, 2.0
	phaseMin, phaseMax = -1.0, 1.0
	// 邊權（361 規格 §6.1.0）
	rawWeightMin, rawWeightMax = 0.1, 1.0
	totalCost                 = 10000
	numEdges                  = 760
)

func main() {
	flag.Parse()
	seed := *seedFlag
	if seed == 0 {
		seed = generateSoulSeed()
	}
	fmt.Println("=== 模擬創角：SoulSeed 與展開值 ===")
	fmt.Printf("soul_seed (int64): %d\n\n", seed)

	rng := rand.New(rand.NewSource(seed))

	// 前 3 次 RNG → 三軸
	u1, u2, u3 := rng.Float64(), rng.Float64(), rng.Float64()
	amp := ampMin + u1*(ampMax-ampMin)
	freq := freqMin + u2*(freqMax-freqMin)
	phase := phaseMin + u3*(phaseMax-phaseMin)

	fmt.Println("三軸信號光譜（前 3 次 RNG）")
	fmt.Printf("  Amplitude (能階): %.4f  [%.1f, %.1f]\n", amp, ampMin, ampMax)
	fmt.Printf("  Frequency (時脈): %.4f  [%.1f, %.1f]\n", freq, freqMin, freqMax)
	fmt.Printf("  Phase (相位):     %.4f  [%.1f, %.1f]\n", phase, phaseMin, phaseMax)

	// 基礎體敏氣由三軸映射（人物屬性彙整 §二建議實作）
	const base = 10.0
	const kAmp, kFreq, kPhase = 0.2, 0.2, 0.2
	baseVit := base * (1 + kAmp*(amp-1))
	baseQi := base * (1 + kFreq*(freq-1))
	baseDex := base * (1 + kPhase*phase)
	fmt.Println("基礎體敏氣（由三軸線性映射，未加竅穴／詞元／裝備）")
	fmt.Printf("  基礎體質: %.1f → 取整 %d\n", baseVit, int(baseVit+0.5))
	fmt.Printf("  基礎氣脈: %.1f → 取整 %d\n", baseQi, int(baseQi+0.5))
	fmt.Printf("  基礎靈敏: %.1f → 取整 %d\n\n", baseDex, int(baseDex+0.5))

	// 第 4～763 次 RNG → 760 條邊原始權重
	raw := make([]float64, numEdges)
	var sumRaw float64
	for k := 0; k < numEdges; k++ {
		u := rng.Float64()
		raw[k] = rawWeightMin + u*(rawWeightMax-rawWeightMin)
		sumRaw += raw[k]
	}

	// 歸一化為總和 10000
	costs := make([]float64, numEdges)
	var sumCost float64
	for k := 0; k < numEdges; k++ {
		costs[k] = (raw[k] / sumRaw) * totalCost
		sumCost += costs[k]
	}

	fmt.Println("361 拓撲邊權（760 條，歸一化後總和 = 10000）")
	fmt.Printf("  原始權重總和 S: %.4f\n", sumRaw)
	fmt.Printf("  Cost 總和:      %.4f\n", sumCost)
	fmt.Println("  前 20 條（型 A 紅→橘）Cost 取樣:")
	for k := 0; k < 20; k++ {
		fmt.Printf("    e_%d: %.2f\n", k+1, costs[k])
	}
	fmt.Println("  型 B 取樣 e_21～e_25:")
	for k := 20; k < 25; k++ {
		fmt.Printf("    e_%d: %.2f\n", k+1, costs[k])
	}
	fmt.Println("  型 C 取樣 e_121～e_125:")
	for k := 120; k < 125; k++ {
		fmt.Printf("    e_%d: %.2f\n", k+1, costs[k])
	}
	minC, maxC := costs[0], costs[0]
	for _, c := range costs {
		if c < minC {
			minC = c
		}
		if c > maxC {
			maxC = c
		}
	}
	fmt.Printf("  760 條 Cost 範圍: min=%.2f, max=%.2f\n", minC, maxC)
}

// generateSoulSeed 模擬創角時後端產生僅屬於該角色的 int64 種子。
// 實作可改為：時間戳＋實體 ID 雜湊、或加密安全亂數。
func generateSoulSeed() int64 {
	var b [8]byte
	if _, err := cryptorand.Read(b[:]); err != nil {
		// 若無 crypto/rand 則用簡單 fallback（僅示範）
		return int64(math.Abs(float64(b[0])*1e10 + float64(b[1])*1e8))
	}
	return int64(binary.BigEndian.Uint64(b[:]))
}

