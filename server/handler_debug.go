// 008 P5：拓撲除錯等開發用 WS 指令。
package server

import (
	"fmt"

	"singularity_world/db"
	"singularity_world/gametext"
)

// handlePrintTopologyDebug 暫時除錯：依當前登入角色之 soul_seed 展開 760 邊權，於伺服器終端印出 SoulSeed、N000→N001/N002/N003 的 Cost、以及全邊 Cost 總和（應為 10000）。
func handlePrintTopologyDebug(c *Client) {
	if c.PlayerID == "" {
		sendError(c, gametext.Client("need_login"))
		return
	}
	ent, err := db.GetEntity(c.PlayerID)
	if err != nil || ent == nil {
		sendError(c, gametext.Client("inv_entity_not_found"))
		return
	}
	if ent.SoulSeed == nil || *ent.SoulSeed == 0 {
		fmt.Println("[topology_debug] 角色無 soul_seed（可能為舊資料）")
		sendError(c, gametext.Client("seed_no_soul"))
		return
	}
	seed := *ent.SoulSeed
	costs := db.ExpandSoulSeedToTopologyCosts(seed)
	var sum float64
	for _, c := range costs {
		sum += c
	}
	fmt.Println("========== 361 拓撲除錯（當前角色） ==========")
	fmt.Printf("  SoulSeed (int64): %d\n", seed)
	fmt.Println("  N000（生之奇點）→ 前三條電漿流 Cost：")
	fmt.Printf("    N000 → N001: %.4f\n", costs[0])
	fmt.Printf("    N000 → N002: %.4f\n", costs[1])
	fmt.Printf("    N000 → N003: %.4f\n", costs[2])
	fmt.Printf("  全 760 條連線 Cost 總和: %.4f （規格常數應為 10000）\n", sum)
	fmt.Println("=============================================")
	c.Send <- mustJSON(TopologyDebugAckMsg{Type: "topology_debug", Message: "已於伺服器終端印出"})
}
