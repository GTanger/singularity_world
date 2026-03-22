// 房間編輯器：GET graph。
package server

import (
	"encoding/json"
	"net/http"
	"sort"

	"singularity_world/store"
)

func handleRoomEditorGraph(w http.ResponseWriter) {
	ids := store.Default.RoomIDs()
	sort.Strings(ids)
	nodes := make([]roomEditorNode, 0, len(ids))
	edges := make([]roomEditorEdge, 0, 256)
	for _, id := range ids {
		r, _ := store.Default.GetRoom(id)
		if r == nil {
			continue
		}
		nodes = append(nodes, roomEditorNode{ID: r.ID, Name: r.Name, Description: r.Description, Zone: r.Zone, Tags: r.Tags, Objects: r.Objects})
		ex, _ := store.Default.GetExitsForRoom(id)
		for _, e := range ex {
			edges = append(edges, roomEditorEdge{From: id, To: e.ToRoomID, Direction: e.Direction})
		}
	}
	_ = json.NewEncoder(w).Encode(roomEditorGraphResp{Nodes: nodes, Edges: edges, Layout: loadRoomEditorLayout(), BasePath: getRoomsBasePath()})
}
