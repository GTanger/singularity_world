// Package server 處理 WebSocket 客戶端訊息：登入、載入房間視野、依出口移動。傳統 MUD 節點連接節點。
package server

import (
	"encoding/json"

	"singularity_world/config"
)

func HandleMessage(c *Client, raw []byte, cfg config.Server, store *SessionStore, hub *Hub) {
	var msg ClientMsg
	if err := json.Unmarshal(raw, &msg); err != nil {
		sendError(c, "invalid json")
		return
	}
	switch msg.Type {
	case "login":
		handleLogin(c, &msg, cfg, store, hub)
	case "create_character":
		handleCreateCharacter(c, &msg, cfg, store, hub)
	case "move":
		handleMove(c, &msg, cfg, store, hub)
	case "ping":
		c.Send <- mustJSON(PongMsg{Type: "pong"})
	case "get_entity_status":
		handleGetEntityStatus(c, &msg)
	case "get_inventory":
		handleGetInventory(c)
	case "equip_item":
		handleEquipItem(c, &msg)
	case "unequip_item":
		handleUnequipItem(c, &msg)
	case "do_action":
		handleDoAction(c, &msg, cfg, store, hub)
	case "print_topology_debug":
		handlePrintTopologyDebug(c)
	default:
		sendError(c, "unknown type: "+msg.Type)
	}
}
