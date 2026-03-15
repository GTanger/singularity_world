// Package server 處理 WebSocket 客戶端訊息：登入、載入房間視野、依出口移動。傳統 MUD 節點連接節點。
package server

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"log"
	"math/rand"
	"strings"
	"time"

	"singularity_world/ai"
	"singularity_world/combat"
	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/event"
	"singularity_world/game"
)

// 創生預設格：以房間名稱「界壁」解析 id，見 db.GetSpawnRoomID。

// HandleMessage 解析客戶端 JSON 並執行 login 或 move；傳入 sessionStore 與 hub 以綁定 session 與廣播。
func HandleMessage(c *Client, raw []byte, database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	var msg ClientMsg
	if err := json.Unmarshal(raw, &msg); err != nil {
		sendError(c, "invalid json")
		return
	}
	switch msg.Type {
	case "login":
		handleLogin(c, &msg, database, cfg, store, hub)
	case "create_character":
		handleCreateCharacter(c, &msg, database, cfg, store, hub)
	case "move":
		handleMove(c, &msg, database, cfg, store, hub)
	case "ping":
		c.Send <- mustJSON(PongMsg{Type: "pong"})
	case "get_entity_status":
		handleGetEntityStatus(c, &msg, database)
	case "get_inventory":
		handleGetInventory(c, database)
	case "equip_item":
		handleEquipItem(c, &msg, database)
	case "unequip_item":
		handleUnequipItem(c, &msg, database)
	case "do_action":
		handleDoAction(c, &msg, database, cfg, store, hub)
	case "print_topology_debug":
		handlePrintTopologyDebug(c, database)
	default:
		sendError(c, "unknown type: "+msg.Type)
	}
}

func handleLogin(c *Client, msg *ClientMsg, database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	if msg.PlayerID == "" {
		sendError(c, "請輸入角色 ID")
		return
	}
	if msg.Password == "" {
		sendError(c, "請輸入密碼")
		return
	}
	ent, err := db.GetEntity(database, msg.PlayerID)
	if err != nil || ent == nil {
		sendError(c, "角色不存在，請先創角")
		return
	}
	if ent.Kind != "player" {
		sendError(c, "此 ID 非玩家角色")
		return
	}
	ok, err := db.VerifyPassword(database, msg.PlayerID, msg.Password)
	if err != nil {
		sendError(c, "驗證失敗")
		return
	}
	if !ok {
		sendError(c, "密碼錯誤")
		return
	}
	loginSuccess(c, msg.PlayerID, database, cfg, store)
}

func handleCreateCharacter(c *Client, msg *ClientMsg, database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	if msg.PlayerID == "" {
		sendError(c, "請輸入角色 ID")
		return
	}
	if msg.Password == "" {
		sendError(c, "請設定密碼")
		return
	}
	if len(msg.Password) < 6 {
		sendError(c, "密碼至少 6 個字元")
		return
	}
	if len(msg.PlayerID) < 2 || len(msg.PlayerID) > 32 {
		sendError(c, "ID 請 2～32 字元")
		return
	}
	existing, err := db.GetEntity(database, msg.PlayerID)
	if err != nil {
		sendError(c, "建立失敗")
		return
	}
	if existing != nil {
		sendError(c, "此 ID 已被使用")
		return
	}
	displayChar := msg.DisplayChar
	if displayChar == "" {
		displayChar = "我"
	}
	if len([]rune(displayChar)) > 1 {
		displayChar = string([]rune(displayChar)[:1])
	}
	gender := "M"
	if msg.Gender == "女" {
		gender = "F"
	} else if msg.Gender == "男" {
		gender = "M"
	}
	if err := db.InsertEntity(database, msg.PlayerID, displayChar, gender); err != nil {
		sendError(c, "建立角色失敗")
		return
	}
	if err := db.SetEntityRoom(database, msg.PlayerID, db.GetSpawnRoomID(database)); err != nil {
		sendError(c, "放入房間失敗")
		return
	}
	if err := db.CreateAuth(database, msg.PlayerID, msg.Password); err != nil {
		sendError(c, "設定密碼失敗")
		return
	}
	loginSuccess(c, msg.PlayerID, database, cfg, store)
}

func loginSuccess(c *Client, playerID string, database *sql.DB, cfg config.Server, store *SessionStore) {
	roomID, err := game.EnsureEntityInRoom(database, playerID, db.GetSpawnRoomID(database))
	if err != nil {
		sendError(c, "房間載入失敗")
		return
	}
	c.PlayerID = playerID
	store.Set(playerID, &Session{Client: c, PlayerID: playerID})
	view, err := game.GetRoomView(database, roomID)
	if err != nil {
		sendError(c, "載入視野失敗")
		return
	}
	// 若玩家上次所在房間已不存在（如地圖重構後 lobby 被移除），改放到創生格並重試
	if view == nil {
		spawnID := db.GetSpawnRoomID(database)
		_ = db.SetEntityRoom(database, playerID, spawnID)
		roomID = spawnID
		view, err = game.GetRoomView(database, roomID)
		if err != nil || view == nil {
			sendError(c, "載入視野失敗")
			return
		}
	}
	ent, _ := db.GetEntity(database, playerID)
	vit, qi, dex := 10, 10, 10
	if ent != nil {
		vit, qi, dex = ent.Vit, ent.Qi, ent.Dex
	}
	rm := db.ComputeResourceMaxes(vit, qi, dex)
	sendRoomView(database, c, view, cfg)
	sendMeWithStatus(c, ent, playerID, roomID, view.Room.Name, vit, qi, dex, rm, database)
}

func handleMove(c *Client, msg *ClientMsg, database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	if c.PlayerID == "" {
		sendError(c, "login first")
		return
	}
	if msg.Direction == "" {
		sendError(c, "direction required")
		return
	}
	newRoomID, ok, err := game.MoveByExit(database, c.PlayerID, msg.Direction)
	if err != nil {
		sendError(c, "move failed")
		return
	}
	if !ok {
		now := game.NowUnix()
		_ = event.Append(database, now, c.PlayerID, event.TypeBlocked, msg.Direction)
		c.Send <- mustJSON(BlockedMsg{Type: "blocked", Direction: msg.Direction})
		return
	}
	view, err := game.GetRoomView(database, newRoomID)
	if err != nil || view == nil {
		sendError(c, "load room failed")
		return
	}
	sendRoomView(database, c, view, cfg)
	hub.Broadcast(mustJSON(MovedMsg{Type: "moved", PlayerID: c.PlayerID, RoomID: newRoomID, RoomName: view.Room.Name}))

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
		npc := npcs[rand.Intn(len(npcs))]
		reaction := db.PickEnterReaction(npc.title, npc.id)
		if reaction == "" {
			return
		}
		time.Sleep(time.Duration(500+rand.Intn(1000)) * time.Millisecond)
		SendNarrateToRoom(store, database, roomID, reaction)
	}(c.PlayerID, newRoomID)
}

func sendRoomView(database *sql.DB, c *Client, view *game.RoomView, cfg config.Server) {
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
				ve.Actions = db.GetSocketsForNPC(database, e.ID, roomID)
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
func sendMeWithStatus(c *Client, ent *entity.Character, playerID, roomID, roomName string, vit, qi, dex int, rm db.ResourceMaxes, database *sql.DB) {
	msg := MeMsg{
		Type:       "me", PlayerID: playerID, RoomID: roomID, RoomName: roomName,
		Vit:        vit, Qi: qi, Dex: dex,
		HpCur:      int(rm.HpCur), HpMax: int(rm.HpMax),
		InnerCur:   int(rm.InnerCur), InnerMax: int(rm.InnerMax),
		SpiritCur:  int(rm.SpiritCur), SpiritMax: int(rm.SpiritMax),
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
		msg.EquipmentSlots, msg.EquipmentNames, _ = parseEquipment(database, ent.EquipmentSlots)
	}
	c.Send <- mustJSON(msg)
}

// parseEquipment 解析 equipment_slots JSON 並查 items 表取物品名稱與描述。
func parseEquipment(database *sql.DB, raw string) (slots, names, descs map[string]string) {
	if raw == "" {
		return nil, nil, nil
	}
	if err := json.Unmarshal([]byte(raw), &slots); err != nil {
		return nil, nil, nil
	}
	names, _ = db.GetItemNames(database, raw)
	descs = db.GetItemDescs(database, raw)
	return slots, names, descs
}

func sendMoved(c *Client, playerID, roomID, roomName string) {
	c.Send <- mustJSON(MovedMsg{Type: "moved", PlayerID: playerID, RoomID: roomID, RoomName: roomName})
}

// ── 插頭插座：do_action ──

func handleDoAction(c *Client, msg *ClientMsg, database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	defer func() {
		if r := recover(); r != nil {
			log.Printf("[do_action] panic: %v", r)
			sendError(c, "執行動作時發生錯誤")
		}
	}()
	if c.PlayerID == "" {
		sendError(c, "請先登入")
		return
	}
	targetID := msg.EntityID
	action := msg.Action
	if targetID == "" || action == "" {
		sendError(c, "缺少目標或動作")
		return
	}
	if targetID == c.PlayerID {
		sendError(c, "無法對自己執行此動作")
		return
	}
	playerRoom, _ := db.GetEntityRoom(database, c.PlayerID)
	if playerRoom == "" {
		sendError(c, "無法取得所在房間")
		return
	}

	// 先嘗試角色目標
	target, err := db.GetEntity(database, targetID)
	if err == nil && target != nil {
		targetRoom, _ := db.GetEntityRoom(database, targetID)
		if targetRoom == "" || playerRoom != targetRoom {
			sendError(c, "目標不在同一房間")
			return
		}
		var targetSockets []string
		if target.Kind == "npc" {
			targetSockets = db.GetSocketsForNPC(database, targetID, playerRoom)
			if !entity.HasSocket(targetSockets, action) {
				sendError(c, "無法對目標執行「"+action+"」")
				return
			}
			if !db.IsDefaultSocket(action) {
				inVenue, _ := db.EntityInVenueAtRoom(database, targetID, playerRoom)
				if !inVenue {
					sendError(c, "對方此刻不在工作場所，無法執行「"+action+"」。")
					return
				}
			}
		} else {
			targetSockets = (&entity.Character{}).Sockets()
			if !entity.HasSocket(targetSockets, action) {
				sendError(c, "無法對目標執行「"+action+"」")
				return
			}
		}
		now := game.NowUnix()
		switch action {
		case "Look":
			narrative := buildLookNarrative(target, database)
			_ = event.Append(database, now, c.PlayerID, event.TypeObserved, targetID)
			lookTargetName := target.DisplayTitle
			if lookTargetName == "" {
				lookTargetName = target.ID
			}
			c.Send <- mustJSON(ActionResultMsg{
				Type: "action_result", Action: "Look",
				TargetID: target.ID, TargetName: lookTargetName,
				Narrative: narrative, Success: true,
			})
		case "Talk":
			playerInput := msg.PlayerInput
			if playerInput == "" {
				playerInput = "（搭話）"
			}
			log.Printf("[Talk] target=%q player_input=%q", targetID, playerInput)
			backstory := db.BuildIdentity(database, targetID)
			snippets := db.SearchArchival(targetID, msg.PlayerInput, 5)
			styleExamples := db.PickStyleExamples(database, targetID, 3)
			reply, err := ai.CallAITalk(cfg.OllamaBaseURL, cfg.OllamaModel, playerInput, backstory, snippets, styleExamples)
			var narrative, npcReply string
			if err != nil || reply == "" {
				if err != nil {
					log.Printf("[Talk] Ollama fallback: %v", err)
				}
				var p *db.Personality
				if target.SoulSeed != nil {
					pp := db.ExpandSoulSeedToPersonality(*target.SoulSeed)
					p = &pp
				}
				narrative = buildTalkNarrative(database, playerRoom, target, p, cfg)
				npcReply = narrative
				if i := strings.Index(narrative, "搭話。"); i >= 0 {
					npcReply = narrative[i+len("搭話。"):]
				}
			} else {
				talkName := target.DisplayTitle
				if talkName == "" {
					talkName = target.ID
				}
				narrative = "你向【" + talkName + "】搭話。" + talkName + "說道：「" + reply + "」"
				npcReply = reply
			}
			_ = event.Append(database, now, c.PlayerID, "talk", targetID)
			targetName := target.DisplayTitle
			if targetName == "" {
				targetName = target.ID
			}
			out := ActionResultMsg{
				Type: "action_result", Action: "Talk",
				TargetID: target.ID, TargetName: targetName,
				Narrative: narrative, Success: true,
			}
			log.Printf("[Talk] sending narrative len=%d", len(narrative))
			c.Send <- mustJSON(out)
			_ = db.InsertArchival(targetID, "玩家說："+playerInput+"；NPC回："+npcReply, "event")
		case "Attack":
			attacker, _ := db.GetEntity(database, c.PlayerID)
			if attacker == nil {
				sendError(c, "找不到自身角色")
				return
			}
			narrative := buildAttackNarrative(attacker, target)
			_ = event.Append(database, now, c.PlayerID, event.TypeCombat, targetID)
			attackTargetName := target.DisplayTitle
			if attackTargetName == "" {
				attackTargetName = target.ID
			}
			c.Send <- mustJSON(ActionResultMsg{
				Type: "action_result", Action: "Attack",
				TargetID: target.ID, TargetName: attackTargetName,
				Narrative: narrative, Success: true,
			})
		default:
			sendError(c, "未知動作："+action)
		}
		return
	}

	// 再嘗試房間物件目標：先依 ID 查，再依名稱查（前端可能送「六角宮燈」等名稱）
	obj, objectRoom := db.GetObjectAndRoom(targetID)
	if obj == nil {
		obj, objectRoom = db.GetObjectByNameInRoom(playerRoom, targetID)
	}
	if obj == nil {
		sendError(c, "找不到目標")
		return
	}
	if objectRoom != playerRoom {
		sendError(c, "目標不在同一房間")
		return
	}
	if !db.ObjectHasSocket(obj, action) {
		sendError(c, "無法對目標執行「"+action+"」")
		return
	}
	narrative := db.ObjectResponse(obj, action)
	if narrative == "" {
		narrative = "你對【" + obj.Name + "】執行了「" + action + "」，但似乎沒有什麼特別的反應。"
	}
	now := game.NowUnix()
	_ = event.Append(database, now, c.PlayerID, event.TypeObserved, obj.ID)

	// 物件具 move_to_room_id 時，Move 動作執行切房
	if action == "Move" && obj.MoveToRoomID != "" {
		view, err := game.GetRoomView(database, obj.MoveToRoomID)
		if err != nil || view == nil {
			sendError(c, "無法前往該處")
			return
		}
		if err := db.SetEntityRoom(database, c.PlayerID, obj.MoveToRoomID); err != nil {
			sendError(c, "移動失敗")
			return
		}
		sendRoomView(database, c, view, cfg)
		hub.Broadcast(mustJSON(MovedMsg{Type: "moved", PlayerID: c.PlayerID, RoomID: obj.MoveToRoomID, RoomName: view.Room.Name}))
	}

	// 回傳該物件其餘可執行動詞（不含剛執行的），讓前端在 Look 後顯示【移動】等可點
	others := make([]string, 0, len(obj.Sockets))
	for _, s := range obj.Sockets {
		if s != action {
			others = append(others, s)
		}
	}
	moveTargetID := ""
	// Look 建築名時若該物件本身無 Move，同房常有「門／簾」物件可進入：找同房具 Move 的物件，讓【移動】改對其送 do_action（偏好緊接在當前物件之後的門）
	if action == "Look" && len(others) == 0 {
		roomObjs := db.GetObjectsInRoom(playerRoom)
		idx := -1
		for i := range roomObjs {
			if roomObjs[i].ID == obj.ID {
				idx = i
				break
			}
		}
		if idx >= 0 {
			for _, delta := range []struct{ start, end int }{{idx + 1, len(roomObjs)}, {0, idx}} {
				for i := delta.start; i < delta.end; i++ {
					o := &roomObjs[i]
					if o.MoveToRoomID != "" && db.ObjectHasSocket(o, "Move") {
						moveTargetID = o.ID
						break
					}
				}
				if moveTargetID != "" {
					break
				}
			}
		} else {
			for i := range roomObjs {
				o := &roomObjs[i]
				if o.MoveToRoomID != "" && db.ObjectHasSocket(o, "Move") {
					moveTargetID = o.ID
					break
				}
			}
		}
		if moveTargetID != "" {
			others = []string{"Move"}
		}
	}
	c.Send <- mustJSON(ActionResultMsg{
		Type: "action_result", Action: action,
		TargetID: obj.ID, TargetName: obj.Name,
		Narrative: narrative, Success: true,
		Actions: others, MoveTargetID: moveTargetID,
	})
}

func buildLookNarrative(target *entity.Character, database *sql.DB) string {
	name := target.DisplayTitle
	if name == "" {
		name = target.ID
	}
	pronoun := "他"
	if target.Gender == "F" {
		pronoun = "她"
	}
	var physique string
	switch {
	case target.Vit >= 20:
		physique = "體格異常魁梧"
	case target.Vit >= 15:
		physique = "體格健壯"
	case target.Vit >= 10:
		physique = "身材勻稱"
	default:
		physique = "身形消瘦"
	}
	var agility string
	switch {
	case target.Dex >= 20:
		agility = "舉止間透著驚人的敏捷"
	case target.Dex >= 15:
		agility = "動作輕靈"
	case target.Dex >= 10:
		agility = "步履平穩"
	default:
		agility = "行動略顯遲緩"
	}
	var qiPresence string
	switch {
	case target.Qi >= 20:
		qiPresence = "，周身隱隱有氣勁流轉"
	case target.Qi >= 15:
		qiPresence = "，氣息沉穩"
	case target.Qi >= 10:
		qiPresence = ""
	default:
		qiPresence = "，氣息微弱"
	}
	desc := fmt.Sprintf("你仔細打量了【%s】。%s%s，%s%s。", name, pronoun, physique, agility, qiPresence)
	if target.EquipmentSlots != "" {
		names, _ := db.GetItemNames(database, target.EquipmentSlots)
		if len(names) > 0 {
			pieces := make([]string, 0, 3)
			for _, n := range names {
				if len(pieces) >= 3 {
					break
				}
				pieces = append(pieces, n)
			}
			desc += " 身上穿戴著" + strings.Join(pieces, "、") + "。"
		}
	}
	return desc
}

func buildTalkNarrative(database *sql.DB, playerRoom string, target *entity.Character, personality *db.Personality, cfg config.Server) string {
	name := target.DisplayTitle
	if name == "" {
		name = target.ID
	}
	// 優先從職業對話檔抽句並填佔位符（{name}、{room}、{time}、{mood}、{verb}、{thing}、{goods}）
	assignments, _ := db.GetAssignmentsForEntity(database, target.ID)
	if len(assignments) > 0 {
		roomName, _ := db.GetRoomName(database, playerRoom)
		timeLabel := ""
		if now := game.NowUnix(); cfg.GameTimeEpochUnix != 0 {
			_, hour, _, _ := game.GameTimeNow(now, cfg.GameTimeEpochUnix, cfg.GameTimeScale)
			if hour >= 5 && hour < 10 {
				timeLabel = "清晨"
			} else if hour >= 10 && hour < 14 {
				timeLabel = "正午"
			} else if hour >= 14 && hour < 18 {
				timeLabel = "傍晚"
			} else {
				timeLabel = "夜裡"
			}
		}
		line := db.PickFromDialogue(database, target.ID, playerRoom, assignments[0].OccupationID, "talk", personality, "", roomName, timeLabel, "")
		if line != "" {
			return "你向【" + name + "】搭話。" + line
		}
	}
	// 無指派或無對話檔時 fallback 提示詞池（也算對話提示詞，擴充以降低重複率）
	responses := []string{
		"「你好，有什麼事嗎？」",
		"「這裡最近不太平靜，你小心點。」",
		"「我只是個路人，別找我麻煩。」",
		"「你看起來像是個新手。」",
		"「嗯？」",
		"「別擋路。」",
		"「你也是來這裡討生活的？」",
		"「聽說城外最近出了些怪事。」",
		"「有事快說，我還有活要幹。」",
		"「這條街什麼人都有，自己多留神。」",
		"「想打聽事？找別人吧。」",
		"「靈脈這幾日不穩，少往地縫邊湊。」",
		"「買東西往那頭，我這兒不賣。」",
		"「路過就路過，別瞎瞧。」",
		"「……你誰啊？」",
		"「有緣再聊，先走了。」",
		"「城裡規矩多，別亂闖。」",
		"「丹藥鋪在東邊，兵器在西邊。」",
		"「沒見過你，新來的？」",
		"「天快黑了，早點找地方落腳。」",
		"「最近生意不好做啊。」",
		"「修行之人，少管閒事。」",
		"「要問路的話，前頭有告示。」",
		"「沒什麼好說的。」",
		"「嗯，怎麼了？」",
		"「這兒不興白打聽，要問拿誠意來。」",
		"「你身上氣息挺雜的，哪條道上的？」",
		"「閒話少說，我忙。」",
		"「初來乍到？先摸清地頭再說。」",
		"「別擋著光。」",
		"「有事說事。」",
		"「……唔。」",
		"「今日不宜多話。」",
		"「你找錯人了。」",
		"「街上人多口雜，別亂搭話。」",
		"「哦。」",
		"「沒見過像你這樣問的。」",
		"「要歇腳往客棧去。」",
		"「靈氣稀薄處少待，傷身。」",
		"「說完了？那我走了。」",
		"「你問的我不清楚。」",
		"「路還長，省點力氣吧。」",
		"「……隨便你。」",
		"「這年頭誰都不容易。」",
		"「別扯上我。」",
		"「有那閒心不如多練兩手。」",
		"「嗯，聽著呢。」",
		"「話多招禍。」",
		"「你問別人吧。」",
		"「沒什麼，隨便聊聊也行。」",
		"「初來？先找個地方住下。」",
		"「這兒就這樣，習慣就好。」",
		"「別耽誤我做事。」",
		"「……有事？」",
		"「風大，聽不清。」",
		"「你自便。」",
		"「少打聽，多做事。」",
		"「今日不順，別惹我。」",
		"「哦，然後呢？」",
		"「路過的？路過就快走。」",
		"「沒什麼好聊的。」",
		"「你誰？」",
		"「嗯。」",
		"「說吧，我聽著。」",
		"「這條街就這樣，熱鬧歸熱鬧，小心點。」",
		"「修行要緊，別瞎晃。」",
		"「……怎麼？」",
		"「有事明日再說。」",
		"「你找別人問去。」",
		"「沒空。」",
		"「隨便。」",
		"「唔。」",
		"「罷了，你說。」",
		"「聽過就忘，別外傳。」",
		"「這兒人多，不方便說。」",
		"「你倒是會挑人問。」",
		"「算了，當我沒說。」",
		"「……行吧。」",
		"「有什麼事？」",
		"「別礙事。」",
		"「嗯，你說。」",
		"「今日不宜久談。」",
		"「路過的人多了，你算一個。」",
		"「要幫手？找掌櫃。」",
		"「我沒什麼好說的。」",
		"「你問的我不懂。」",
		"「少來套近乎。」",
		"「……何事？」",
		"「聽著呢。」",
		"「有事快說。」",
		"「這地兒就這樣。」",
		"「別亂打聽。」",
		"「嗯哼。」",
		"「你自求多福。」",
		"「沒什麼。」",
		"「說。」",
		"「……哦。」",
		"「隨便聊聊可以。」",
		"「別耽誤工夫。」",
		"「今日沒興致。」",
		"「你問別人。」",
		"「聽到了。」",
		"「嗯，然後？」",
		"「有事？」",
		"「少廢話。」",
		"「……說完了？」",
		"「你忙你的。」",
		"「這條街規矩多。」",
		"「別惹事。」",
		"「唔，好吧。」",
		"「聽著。」",
		"「沒什麼好問的。」",
		"「你問吧。」",
		"「今日不宜多言。」",
		"「路過。」",
		"「……嗯。」",
		"「有事說。」",
		"「別擋道。」",
		"「隨便你。」",
		"「聽你的。」",
		"「沒空聊。」",
		"「你誰啊？」",
		"「嗯。」",
		"「說來聽聽。」",
		"「這兒不興多問。」",
		"「別多事。」",
		"「……好。」",
		"「有事？」",
		"「聽著呢。」",
		"「沒什麼。」",
		"「你說。」",
		"「今日事多。」",
		"「路過的都這樣問。」",
		"「……唔。」",
		"「有事快說。」",
		"「別礙著。」",
		"「隨便。」",
		"「聽到了。」",
		"「沒興趣。」",
		"「你問什麼？」",
		"「嗯。」",
		"「說吧。」",
		"「這地兒雜。」",
		"「別亂來。」",
		"「……哦。」",
		"「有事？」",
		"「聽著。」",
		"「沒啥。」",
		"「你說說看。」",
		"「今日忙。」",
		"「路過的人不少。」",
		"「……嗯。」",
		"「有事說事。」",
		"「別擋。」",
		"「隨你。」",
		"「聽。」",
		"「沒。」",
		"「你講。」",
		"「嗯。」",
		"「說。」",
		"「這兒就這樣。」",
		"「別。」",
		"「……。」",
	}
	h := 0
	for _, r := range target.ID {
		h += int(r)
	}
	now := game.NowUnix()
	// 用遊戲時間 + NPC ID  hash 做 seed，同一刻同一 NPC 固定，不同時刻或不同 NPC 則分散，降低重複感
	seed := int64(now)*1000 + int64(h)
	rng := rand.New(rand.NewSource(seed))
	idx := rng.Intn(len(responses))
	if personality != nil {
		shift := int(personality.Boldness * float64(len(responses) / 2))
		idx = (idx + shift + len(responses)) % len(responses)
	}
	return "你向【" + name + "】搭話。" + name + "說道：" + responses[idx]
}

func buildAttackNarrative(attacker, defender *entity.Character) string {
	winner, rawLog := combat.Resolve(attacker.Vit, attacker.Dex, defender.Vit, defender.Dex)
	aName := attacker.DisplayTitle
	if aName == "" {
		aName = attacker.ID
	}
	dName := defender.DisplayTitle
	if dName == "" {
		dName = defender.ID
	}
	log := strings.ReplaceAll(rawLog, "攻方", "【"+aName+"】")
	log = strings.ReplaceAll(log, "守方", "【"+dName+"】")
	prefix := "你向【" + dName + "】發起攻擊！"
	var suffix string
	if winner == "attacker" {
		suffix = "\n你取得了勝利。（戰鬥系統尚未完整實裝，不扣除氣血）"
	} else {
		suffix = "\n你敗下陣來。（戰鬥系統尚未完整實裝，不扣除氣血）"
	}
	return prefix + log + suffix
}

func handleGetEntityStatus(c *Client, msg *ClientMsg, database *sql.DB) {
	if c.PlayerID == "" {
		sendError(c, "請先登入")
		return
	}
	entityID := msg.EntityID
	if entityID == "" {
		entityID = c.PlayerID
	}
	ent, err := db.GetEntity(database, entityID)
	if err != nil || ent == nil {
		sendError(c, "找不到該角色")
		return
	}
	isSelf := entityID == c.PlayerID
	mag := ent.Magnesium
	var magPtr *int
	if isSelf {
		magPtr = &mag
	}
	rm := db.ComputeResourceMaxes(ent.Vit, ent.Qi, ent.Dex)
	status := EntityStatusMsg{
		Type:        "entity_status",
		EntityID:    ent.ID,
		DisplayChar: ent.DisplayChar,
		Vit:         ent.Vit,
		Qi:          ent.Qi,
		Dex:         ent.Dex,
		HpCur:       int(rm.HpCur), HpMax: int(rm.HpMax),
		InnerCur:    int(rm.InnerCur), InnerMax: int(rm.InnerMax),
		SpiritCur:   int(rm.SpiritCur), SpiritMax: int(rm.SpiritMax),
		StaminaCur:  int(rm.StaminaCur), StaminaMax: int(rm.StaminaMax),
		Magnesium:   magPtr,
		IsSelf:      isSelf,
	}
	if ent.DisplayTitle != "" {
		status.DisplayTitle = ent.DisplayTitle
	}
	if isSelf {
		status.ActivatedNodes = parseActivatedNodes(ent.ActivatedNodes)
		if ent.SoulSeed != nil {
			status.OriginSentence = db.ExpandSoulSeedToOriginSentence(*ent.SoulSeed)
			status.TopologyCosts = db.ExpandSoulSeedToTopologyCosts(*ent.SoulSeed)
		}
	}
	status.EquipmentSlots, status.EquipmentNames, status.EquipmentDescs = parseEquipment(database, ent.EquipmentSlots)
	c.Send <- mustJSON(status)
}

// handlePrintTopologyDebug 暫時除錯：依當前登入角色之 soul_seed 展開 760 邊權，於伺服器終端印出 SoulSeed、N000→N001/N002/N003 的 Cost、以及全邊 Cost 總和（應為 10000）。
func handlePrintTopologyDebug(c *Client, database *sql.DB) {
	if c.PlayerID == "" {
		sendError(c, "請先登入")
		return
	}
	ent, err := db.GetEntity(database, c.PlayerID)
	if err != nil || ent == nil {
		sendError(c, "找不到角色")
		return
	}
	if ent.SoulSeed == nil || *ent.SoulSeed == 0 {
		fmt.Println("[topology_debug] 角色無 soul_seed（可能為舊資料）")
		sendError(c, "此角色無 soul_seed")
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

func handleGetInventory(c *Client, database *sql.DB) {
	if c.PlayerID == "" {
		sendError(c, "請先登入")
		return
	}
	ent, err := db.GetEntity(database, c.PlayerID)
	if err != nil || ent == nil {
		sendError(c, "找不到角色")
		return
	}
	result := db.GetInventory(database, ent.Inventory, ent.Vit)
	items := make([]InventoryItemView, 0, len(result.Items))
	for _, it := range result.Items {
		items = append(items, InventoryItemView{
			ItemID:      it.ItemID,
			Name:        it.Name,
			ItemType:    it.ItemType,
			Qty:         it.Qty,
			Weight:      it.Weight,
			SubTotal:    it.SubTotal,
			Description: it.Description,
			Slot:        it.Slot,
		})
	}
	c.Send <- mustJSON(InventoryMsg{
		Type:          "inventory",
		Items:         items,
		CurrentWeight: result.CurrentWeight,
		MaxWeight:     result.MaxWeight,
	})
}

func handleEquipItem(c *Client, msg *ClientMsg, database *sql.DB) {
	if c.PlayerID == "" {
		sendError(c, "請先登入")
		return
	}
	itemID := msg.ItemID
	if itemID == "" {
		sendError(c, "未指定物品")
		return
	}
	_, itemType, slot, _, _, err := db.GetItemInfo(database, itemID)
	if err != nil {
		sendError(c, "物品不存在")
		return
	}
	if itemType != "equipment" || slot == "" {
		sendError(c, "此物品無法穿戴")
		return
	}
	targetSlot := slot
	if slot == "hold" {
		targetSlot = msg.TargetSlot
		if targetSlot != "hold_l" && targetSlot != "hold_r" {
			sendError(c, "請指定左手或右手")
			return
		}
	}

	ent, err := db.GetEntity(database, c.PlayerID)
	if err != nil || ent == nil {
		sendError(c, "找不到角色")
		return
	}

	var invEntries []db.InventoryEntry
	if ent.Inventory != "" && ent.Inventory != "[]" {
		_ = json.Unmarshal([]byte(ent.Inventory), &invEntries)
	}
	hasItem := false
	for _, e := range invEntries {
		if e.ItemID == itemID && e.Qty > 0 {
			hasItem = true
			break
		}
	}
	if !hasItem {
		sendError(c, "背包中無此物品")
		return
	}

	var currentSlots map[string]string
	if ent.EquipmentSlots != "" {
		_ = json.Unmarshal([]byte(ent.EquipmentSlots), &currentSlots)
	}
	if currentSlots == nil {
		currentSlots = make(map[string]string)
	}
	oldItemID := currentSlots[targetSlot]

	if err := db.RemoveFromInventory(database, c.PlayerID, itemID, 1); err != nil {
		sendError(c, "背包操作失敗")
		return
	}
	if err := db.UpdateEquipmentSlot(database, c.PlayerID, targetSlot, itemID); err != nil {
		sendError(c, "裝備操作失敗")
		return
	}
	if oldItemID != "" {
		if err := db.AddToInventory(database, c.PlayerID, oldItemID, 1); err != nil {
			sendError(c, "舊裝備回收失敗")
			return
		}
	}

	pushRefresh(c, database)
}

func handleUnequipItem(c *Client, msg *ClientMsg, database *sql.DB) {
	if c.PlayerID == "" {
		sendError(c, "請先登入")
		return
	}
	slotCode := msg.Slot
	if slotCode == "" {
		sendError(c, "未指定槽位")
		return
	}

	ent, err := db.GetEntity(database, c.PlayerID)
	if err != nil || ent == nil {
		sendError(c, "找不到角色")
		return
	}

	var currentSlots map[string]string
	if ent.EquipmentSlots != "" {
		_ = json.Unmarshal([]byte(ent.EquipmentSlots), &currentSlots)
	}
	if currentSlots == nil {
		sendError(c, "無裝備可脫下")
		return
	}
	itemID := currentSlots[slotCode]
	if itemID == "" {
		sendError(c, "該槽位無裝備")
		return
	}

	_, _, _, _, weight, wErr := db.GetItemInfo(database, itemID)
	if wErr != nil {
		weight = 0
	}
	currentWeight := db.InventoryWeight(database, ent.Inventory)
	maxWeight := float64(ent.Vit) * 10.0
	if currentWeight+weight > maxWeight {
		sendError(c, "背包已滿，無法脫下")
		return
	}

	if err := db.ClearEquipmentSlot(database, c.PlayerID, slotCode); err != nil {
		sendError(c, "槽位操作失敗")
		return
	}
	if err := db.AddToInventory(database, c.PlayerID, itemID, 1); err != nil {
		sendError(c, "背包操作失敗")
		return
	}

	pushRefresh(c, database)
}

func pushRefresh(c *Client, database *sql.DB) {
	ent, err := db.GetEntity(database, c.PlayerID)
	if err != nil || ent == nil {
		return
	}
	rm := db.ComputeResourceMaxes(ent.Vit, ent.Qi, ent.Dex)
	invResult := db.GetInventory(database, ent.Inventory, ent.Vit)
	items := make([]InventoryItemView, 0, len(invResult.Items))
	for _, it := range invResult.Items {
		items = append(items, InventoryItemView{
			ItemID:      it.ItemID,
			Name:        it.Name,
			ItemType:    it.ItemType,
			Qty:         it.Qty,
			Weight:      it.Weight,
			SubTotal:    it.SubTotal,
			Description: it.Description,
			Slot:        it.Slot,
		})
	}
	c.Send <- mustJSON(InventoryMsg{
		Type:          "inventory",
		Items:         items,
		CurrentWeight: invResult.CurrentWeight,
		MaxWeight:     invResult.MaxWeight,
	})

	isSelf := true
	mag := ent.Magnesium
	status := EntityStatusMsg{
		Type:        "entity_status",
		EntityID:    ent.ID,
		DisplayChar: ent.DisplayChar,
		Vit:         ent.Vit, Qi: ent.Qi, Dex: ent.Dex,
		HpCur: int(rm.HpCur), HpMax: int(rm.HpMax),
		InnerCur: int(rm.InnerCur), InnerMax: int(rm.InnerMax),
		SpiritCur: int(rm.SpiritCur), SpiritMax: int(rm.SpiritMax),
		StaminaCur: int(rm.StaminaCur), StaminaMax: int(rm.StaminaMax),
		Magnesium: &mag,
		IsSelf:    isSelf,
	}
	if ent.DisplayTitle != "" {
		status.DisplayTitle = ent.DisplayTitle
	}
	status.ActivatedNodes = parseActivatedNodes(ent.ActivatedNodes)
	if ent.SoulSeed != nil {
		status.OriginSentence = db.ExpandSoulSeedToOriginSentence(*ent.SoulSeed)
		status.TopologyCosts = db.ExpandSoulSeedToTopologyCosts(*ent.SoulSeed)
	}
	status.EquipmentSlots, status.EquipmentNames, status.EquipmentDescs = parseEquipment(database, ent.EquipmentSlots)
	c.Send <- mustJSON(status)
}

func sendError(c *Client, message string) {
	c.Send <- mustJSON(ErrorMsg{Type: "error", Message: message})
}

func mustJSON(v interface{}) []byte {
	b, _ := json.Marshal(v)
	return b
}

// GetObserverPositions 回傳目前所有已登入玩家的世界座標（格點制時供 RunViewSimulation 用；房間制可留空）。
func GetObserverPositions(store *SessionStore, database *sql.DB) []game.Pos {
	return nil
}

// BroadcastRoomViews 對所有在線玩家推送其當前房間的最新視野。
// 用於 NPC 排班移動後同步前端人物欄。
func BroadcastRoomViews(store *SessionStore, database *sql.DB, cfg config.Server) {
	for _, s := range store.AllSessions() {
		roomID, _ := db.GetEntityRoom(database, s.PlayerID)
		if roomID == "" {
			continue
		}
		view, err := game.GetRoomView(database, roomID)
		if err != nil || view == nil {
			continue
		}
		sendRoomView(database, s.Client, view, cfg)
	}
}
