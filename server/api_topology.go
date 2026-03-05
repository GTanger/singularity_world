// Package server 星盤拓撲 API：驗證玩家身份後回傳 361 nodes + 760 edges（含 Cost）供星盤檢視器使用。
package server

import (
	"database/sql"
	"encoding/json"
	"net/http"

	"singularity_world/db"
)

// topoNode 星盤節點。
type topoNode struct {
	ID     string `json:"id"`
	Name   string `json:"name"`
	Zone   int    `json:"zone"`
	System string `json:"system"`
}

// topoEdge 星盤邊（含 Cost）。
type topoEdge struct {
	From string  `json:"from"`
	To   string  `json:"to"`
	Type string  `json:"type"`
	Cost float64 `json:"cost"`
}

// topoResponse API 回傳結構。
type topoResponse struct {
	PlayerID string     `json:"player_id"`
	Nodes    []topoNode `json:"nodes"`
	Edges    []topoEdge `json:"edges"`
}

// hubNames 二十主樞名稱（索引 0=N001, 19=N020）。
var hubNames = [20]string{
	"天極", "脈衝", "震淵", "游離", "弦絲",
	"曜核", "凜晶", "淵流", "萬象", "解離",
	"鎮閾", "衡定", "穹壁", "重塑", "逆熵",
	"神淵", "識閾", "坍縮", "無相", "越權",
}

// hubSystems 二十主樞系屬（索引 0=N001），與 361 規格 §2.2 一致。
var hubSystems = [20]string{
	"體", "敏", "體", "敏", "敏",
	"氣", "氣", "氣", "氣", "氣",
	"體", "氣", "體", "體", "氣",
	"氣", "敏", "氣", "敏", "敏",
}

// logicNames 五常邏輯閘代號。
var logicNames = [5]string{"起", "承", "轉", "協", "合"}

// peripheralNames 十二微型狀態代號。
var peripheralNames = [12]string{
	"探", "觸", "納", "蓄", "濾", "析",
	"融", "衍", "律", "束", "釋", "散",
}

// nodeID 由編號產出 N000~N360 格式字串。
func nodeID(n int) string {
	s := "N"
	if n < 10 {
		s += "00"
	} else if n < 100 {
		s += "0"
	}
	return s + itoa(n)
}

// itoa 簡易整數轉字串，避免引入 strconv。
func itoa(n int) string {
	if n == 0 {
		return "0"
	}
	buf := [4]byte{}
	pos := 3
	for n > 0 {
		buf[pos] = byte('0' + n%10)
		n /= 10
		pos--
	}
	return string(buf[pos+1:])
}

// hubSystem 由主樞索引（1-based）取得系屬。
func hubSystem(i int) string {
	if i < 1 || i > 20 {
		return ""
	}
	return hubSystems[i-1]
}

// hubForBlue 由藍節點編號（21~120）推算所屬主樞（1~20）。
func hubForBlue(nid int) int {
	return (nid-21)/5 + 1
}

// hubForGreen 由綠節點編號（121~360）推算所屬主樞（1~20）。
func hubForGreen(nid int) int {
	return (nid-121)/12 + 1
}

// buildTopologyNodes 產出 361 個節點。
func buildTopologyNodes() []topoNode {
	nodes := make([]topoNode, 0, 361)
	nodes = append(nodes, topoNode{ID: "N000", Name: "生之奇點", Zone: 0, System: ""})
	for i := 1; i <= 20; i++ {
		nodes = append(nodes, topoNode{
			ID: nodeID(i), Name: hubNames[i-1], Zone: 1, System: hubSystems[i-1],
		})
	}
	for i := 1; i <= 20; i++ {
		sys := hubSystems[i-1]
		for j := 1; j <= 5; j++ {
			nid := 20 + 5*(i-1) + j
			nodes = append(nodes, topoNode{
				ID: nodeID(nid), Name: logicNames[j-1], Zone: 2, System: sys,
			})
		}
	}
	for i := 1; i <= 20; i++ {
		sys := hubSystems[i-1]
		for s := 1; s <= 12; s++ {
			nid := 120 + 12*(i-1) + s
			nodes = append(nodes, topoNode{
				ID: nodeID(nid), Name: peripheralNames[s-1], Zone: 3, System: sys,
			})
		}
	}
	return nodes
}

// buildTopologyEdges 產出 760 條邊並綁定 costs（順序與 db.ExpandSoulSeedToTopologyCosts 一致）。
func buildTopologyEdges(costs []float64) []topoEdge {
	edges := make([]topoEdge, 0, 760)
	idx := 0

	for i := 1; i <= 20; i++ {
		edges = append(edges, topoEdge{From: "N000", To: nodeID(i), Type: "A", Cost: costs[idx]})
		idx++
	}
	for i := 1; i <= 20; i++ {
		for j := 1; j <= 5; j++ {
			blue := 20 + 5*(i-1) + j
			edges = append(edges, topoEdge{From: nodeID(i), To: nodeID(blue), Type: "B", Cost: costs[idx]})
			idx++
		}
	}
	blueGreenMap := [5][3]int{
		{1, 2, 3}, {3, 4, 5}, {5, 6, 7}, {8, 9, 10}, {10, 11, 12},
	}
	for i := 1; i <= 20; i++ {
		for j := 0; j < 5; j++ {
			blueID := 20 + 5*(i-1) + (j + 1)
			for _, gs := range blueGreenMap[j] {
				greenID := 120 + 12*(i-1) + gs
				edges = append(edges, topoEdge{From: nodeID(blueID), To: nodeID(greenID), Type: "C", Cost: costs[idx]})
				idx++
			}
		}
	}
	for i := 1; i <= 20; i++ {
		base := 120 + 12*(i-1)
		for s := 1; s <= 12; s++ {
			from := base + s
			next := s%12 + 1
			to := base + next
			edges = append(edges, topoEdge{From: nodeID(from), To: nodeID(to), Type: "D", Cost: costs[idx]})
			idx++
		}
	}
	for i := 1; i <= 20; i++ {
		base := 20 + 5*(i-1)
		for j := 1; j <= 5; j++ {
			from := base + j
			next := j%5 + 1
			to := base + next
			edges = append(edges, topoEdge{From: nodeID(from), To: nodeID(to), Type: "E", Cost: costs[idx]})
			idx++
		}
	}
	return edges
}

// HandleTopologyAPI 處理 GET /api/topology?id=xxx&pw=yyy，驗證密碼後回傳該玩家的星盤拓撲。
func HandleTopologyAPI(database *sql.DB, w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	if r.Method != http.MethodGet {
		http.Error(w, `{"error":"GET only"}`, http.StatusMethodNotAllowed)
		return
	}
	playerID := r.URL.Query().Get("id")
	password := r.URL.Query().Get("pw")
	if playerID == "" || password == "" {
		http.Error(w, `{"error":"需提供 id 與 pw 參數"}`, http.StatusBadRequest)
		return
	}
	ok, err := db.VerifyPassword(database, playerID, password)
	if err != nil || !ok {
		http.Error(w, `{"error":"身份驗證失敗"}`, http.StatusForbidden)
		return
	}
	ent, err := db.GetEntity(database, playerID)
	if err != nil || ent == nil {
		http.Error(w, `{"error":"角色不存在"}`, http.StatusNotFound)
		return
	}
	if ent.SoulSeed == nil {
		http.Error(w, `{"error":"角色無 SoulSeed"}`, http.StatusInternalServerError)
		return
	}
	costs := db.ExpandSoulSeedToTopologyCosts(*ent.SoulSeed)
	resp := topoResponse{
		PlayerID: playerID,
		Nodes:    buildTopologyNodes(),
		Edges:    buildTopologyEdges(costs),
	}
	w.Header().Set("Cache-Control", "no-cache")
	json.NewEncoder(w).Encode(resp)
}
