// 一次性除錯：用與 server 相同的 db.ExpandSoulSeedToTopologyCosts 印出 SoulSeed、N000→N001/N002/N003 Cost、全邊總和。
package main

import (
	"flag"
	"fmt"

	"singularity_world/db"
)

func main() {
	seed := flag.Int64("seed", 42, "soul_seed 值")
	flag.Parse()
	costs := db.ExpandSoulSeedToTopologyCosts(*seed)
	var sum float64
	for _, c := range costs {
		sum += c
	}
	fmt.Println("========== 361 拓撲除錯（當前角色） ==========")
	fmt.Printf("  SoulSeed (int64): %d\n", *seed)
	fmt.Println("  N000（生之奇點）→ 前三條電漿流 Cost：")
	fmt.Printf("    N000 → N001: %.4f\n", costs[0])
	fmt.Printf("    N000 → N002: %.4f\n", costs[1])
	fmt.Printf("    N000 → N003: %.4f\n", costs[2])
	fmt.Printf("  全 760 條連線 Cost 總和: %.4f （規格常數應為 10000）\n", sum)
	fmt.Println("=============================================")
}
