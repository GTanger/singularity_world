// 008 P5：移動、房間視野、me 訊息與離房觀測清理。
package server

import (
	"encoding/json"
	"math/rand"
	"time"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/event"
	"singularity_world/game"
	"singularity_world/npc"
)

func handleMove(c *Client, msg *ClientMsg, cfg config.Server, store *SessionStore, hub *Hub) {
	if c.PlayerID == "" {
		sendError(c, "login first")
		return
	}
	if msg.Direction == "" {
		sendError(c, "direction required")
		return
	}
	oldRoomID, _ := db.GetEntityRoom(c.PlayerID)
	newRoomID, ok, err := game.MoveByExit(c.PlayerID, msg.Direction)
	if err != nil {
		sendError(c, "move failed")
		return
	}
	if !ok {
		now := game.NowUnix()
		_ = event.Append(now, c.PlayerID, event.TypeBlocked, msg.Direction)
		c.Send <- mustJSON(BlockedMsg{Type: "blocked", Direction: msg.Direction})
		return
	}
	// §七 7.3：離開房間 → 若該房已無其他玩家，該房 NPC 恢復未觀測狀態
	onLeaveRoom(store, oldRoomID, c.PlayerID)
	view, err := game.GetRoomView(newRoomID, currentGameHour(cfg))
	if err != nil || view == nil {
		sendError(c, "load room failed")
		return
	}
	sendRoomView(c, view, cfg)
	hub.Broadcast(mustJSON(MovedMsg{Type: "moved", PlayerID: c.PlayerID, RoomID: newRoomID, RoomName: view.Room.Name}))
	// 10.18 短期記憶：玩家進房時記錄與該房每位 NPC 的「見面」一次
	for _, e := range view.Entities {
		if e.Kind == "npc" && e.ID != c.PlayerID {
			_ = db.RecordMeet(e.ID, c.PlayerID)
		}
	}

	// NPC 進房反應：隨機挑一個同房 NPC 延遲回應
	go func(playerID, roomID string) {
		type npcInfo struct{ id, title string }
		var npcs []npcInfo
		for _, e := range view.Entities {
			if e.Kind == "npc" && e.ID != playerID {
				npcs = append(npcs, npcInfo{e.ID, e.DisplayTitle})
			}
		}
		if len(npcs) == 0 {
			return
		}
		picked := npcs[rand.Intn(len(npcs))]
		occ, person := db.SplitNPCListDisplayLabel(picked.title)
		if person == "" {
			person = picked.id
		}
		if occ == "" {
			return
		}
		reaction := npc.PickEnterReaction(occ, person)
		if reaction == "" {
			return
		}
		time.Sleep(time.Duration(500+rand.Intn(1000)) * time.Millisecond)
		SendNarrateToRoom(store, roomID, reaction)
	}(c.PlayerID, newRoomID)
}

// onLeaveRoom 當玩家離開某房時，若該房已無其他玩家，則將該房內所有 NPC 的 last_observed_at 清空（恢復未觀測）。
func onLeaveRoom(store *SessionStore, roomID, leftPlayerID string) {
	if roomID == "" {
		return
	}
	for _, s := range store.AllSessions() {
		if s.PlayerID == "" || s.PlayerID == leftPlayerID {
			continue
		}
		r, _ := db.GetEntityRoom(s.PlayerID)
		if r == roomID {
			return // 仍有其他玩家在該房，不清
		}
	}
	entities, _ := db.GetEntitiesInRoom(roomID, -1)
	for _, e := range entities {
		if e.Kind == "npc" {
			_ = db.ClearLastObserved(e.ID)
		}
	}
}

func sendRoomView(c *Client, view *game.RoomView, cfg config.Server) {
	if view == nil {
		return
	}
	roomID := view.Room.ID
	entities := make([]ViewEntity, 0, len(view.Entities))
	for _, e := range view.Entities {
		ve := ViewEntity{ID: e.ID, Kind: e.Kind, DisplayChar: e.DisplayChar}
		if e.Kind == "npc" && e.DisplayTitle != "" {
			ve.DisplayName = e.DisplayTitle
		} else {
			ve.DisplayName = e.ID
		}
		if e.ID != c.PlayerID {
			if e.Kind == "npc" {
				ve.Actions = db.GetSocketsForNPC(e.ID, roomID)
			} else {
				ve.Actions = e.Sockets()
			}
		}
		entities = append(entities, ve)
	}
	exits := make([]ExitView, 0, len(view.Exits))
	for _, ex := range view.Exits {
		exits = append(exits, ExitView{Direction: ex.Direction, ToRoomID: ex.ToRoomID, ToRoomName: ex.ToRoomName})
	}
	objects := make([]ViewObject, 0, len(view.Objects))
	for _, o := range view.Objects {
		objects = append(objects, ViewObject{ID: o.ID, Name: o.Name, Actions: o.Sockets})
	}
	now := game.NowUnix()
	secSinceMidnight, _, _, daysSinceEpoch := game.GameTimeNow(now, cfg.GameTimeEpochUnix, cfg.GameTimeScale)
	msg := RoomViewMsg{
		Type:                     "view",
		RoomID:                   view.Room.ID,
		RoomName:                 view.Room.Name,
		Description:              view.Room.Description,
		Exits:                    exits,
		Entities:                 entities,
		Objects:                  objects,
		ServerUnix:               now,
		GameTimeSecSinceMidnight: secSinceMidnight,
		GameDaysSinceEpoch:       daysSinceEpoch,
	}
	// §七 7.1／7.2：進入房間觸發觀測 — 經 Observer 介面或 fallback ObserveRoom
	if defaultObserver != nil {
		for _, e := range view.Entities {
			if e.Kind == "npc" {
				defaultObserver.OnObserve(e.ID, c.PlayerID, now)
			}
		}
	} else {
		game.ObserveRoom(roomID, c.PlayerID, now)
	}
	c.Send <- mustJSON(msg)
}

func sendMe(c *Client, playerID, roomID, roomName string, vit, qi, dex int, rm db.ResourceMaxes) {
	c.Send <- mustJSON(MeMsg{
		Type: "me", PlayerID: playerID, RoomID: roomID, RoomName: roomName,
		Vit: vit, Qi: qi, Dex: dex,
		HpCur: int(rm.HpCur), HpMax: int(rm.HpMax),
		InnerCur: int(rm.InnerCur), InnerMax: int(rm.InnerMax),
		SpiritCur: int(rm.SpiritCur), SpiritMax: int(rm.SpiritMax),
		StaminaCur: int(rm.StaminaCur), StaminaMax: int(rm.StaminaMax),
	})
}

// parseActivatedNodes 將 entities.activated_nodes（JSON 陣列字串）解析為 []string；失敗或空則回傳 ["N000"]。
func parseActivatedNodes(raw string) []string {
	if raw == "" {
		return []string{"N000"}
	}
	var list []string
	if err := json.Unmarshal([]byte(raw), &list); err != nil {
		return []string{"N000"}
	}
	if len(list) == 0 {
		return []string{"N000"}
	}
	return list
}

// sendMeWithStatus 同 sendMe，並帶入命途／本源／星盤／裝備欄位；ent 可為 nil。
func sendMeWithStatus(c *Client, ent *entity.Character, playerID, roomID, roomName string, vit, qi, dex int, rm db.ResourceMaxes) {
	msg := MeMsg{
		Type: "me", PlayerID: playerID, RoomID: roomID, RoomName: roomName,
		Vit: vit, Qi: qi, Dex: dex,
		HpCur: int(rm.HpCur), HpMax: int(rm.HpMax),
		InnerCur: int(rm.InnerCur), InnerMax: int(rm.InnerMax),
		SpiritCur: int(rm.SpiritCur), SpiritMax: int(rm.SpiritMax),
		StaminaCur: int(rm.StaminaCur), StaminaMax: int(rm.StaminaMax),
	}
	if ent != nil {
		if ent.DisplayTitle != "" {
			msg.DisplayTitle = ent.DisplayTitle
		}
		if ent.SoulSeed != nil {
			msg.OriginSentence = db.ExpandSoulSeedToOriginSentence(*ent.SoulSeed)
			msg.TopologyCosts = db.ExpandSoulSeedToTopologyCosts(*ent.SoulSeed)
		}
		msg.ActivatedNodes = parseActivatedNodes(ent.ActivatedNodes)
		msg.EquipmentSlots, msg.EquipmentNames, _ = parseEquipment(ent.EquipmentSlots)
	}
	c.Send <- mustJSON(msg)
}

// parseEquipment 解析 equipment_slots JSON 並查 items 表取物品名稱與描述。
func parseEquipment(raw string) (slots, names, descs map[string]string) {
	if raw == "" {
		return nil, nil, nil
	}
	if err := json.Unmarshal([]byte(raw), &slots); err != nil {
		return nil, nil, nil
	}
	names, _ = db.GetItemNames(raw)
	descs = db.GetItemDescs(raw)
	return slots, names, descs
}

func sendMoved(c *Client, playerID, roomID, roomName string) {
	c.Send <- mustJSON(MovedMsg{Type: "moved", PlayerID: playerID, RoomID: roomID, RoomName: roomName})
}
