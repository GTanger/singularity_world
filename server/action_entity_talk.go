// do_action：對同房實體 Talk（Ollama 或敘事 fallback）。
package server

import (
	"log"
	"strings"

	"singularity_world/ai"
	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/event"
)

func doActionEntityTalk(c *Client, msg *ClientMsg, cfg config.Server, store *SessionStore, now int64, playerRoom, targetID string, target *entity.Character) {
	playerInput := msg.PlayerInput
	if playerInput == "" {
		playerInput = "（搭話）"
	}
	log.Printf("[Talk] target=%q player_input=%q", targetID, playerInput)
	backstory := db.BuildIdentity(targetID) + db.FormatNpcMemoryForBackstory(targetID, c.PlayerID)
	snippets := db.SearchArchivalForPlayerTalk(targetID, msg.PlayerInput, 5)
	styleExamples := db.PickStyleExamples(targetID, 3)
	var p *db.Personality
	if target.SoulSeed != nil {
		pp := db.ExpandSoulSeedToPersonality(*target.SoulSeed)
		p = &pp
	}
	sensitivityHint := ""
	if p != nil {
		if p.Sensitivity > 0.6 {
			sensitivityHint = "此角色較熱絡，可多說一兩句。"
		} else if p.Sensitivity < 0.3 {
			sensitivityHint = "此角色較冷淡，回覆簡短。"
		}
	}
	disp := db.GetDisposition(targetID)
	if disp < -30 {
		sensitivityHint += "此角色心境低落，語氣較冷漠沉鬱。"
	} else if disp > 30 {
		sensitivityHint += "此角色心情愉快，語氣較活潑熱情。"
	}
	roomDisplayName, _ := db.GetRoomName(playerRoom)
	if strings.TrimSpace(roomDisplayName) == "" {
		roomDisplayName = playerRoom
	}
	systemPrompt, userMsg := ai.PlayerNPCTalkBuildPrompts(playerInput, backstory, snippets, styleExamples, sensitivityHint, roomDisplayName)
	var narrative, npcReply string
	switch {
	case cfg.PlayerTalkUsesWebAPI():
		reply, err := ai.CallOpenAICompatibleChat(cfg.PlayerTalkAPIBaseURL, cfg.PlayerTalkAPIKey, cfg.PlayerTalkAPIModel, systemPrompt, userMsg)
		if err != nil || reply == "" {
			if err != nil {
				log.Printf("[Talk] web API fallback: %v", err)
			}
			narrative = buildTalkNarrative(playerRoom, target, p, cfg, playerInput)
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
	case cfg.OllamaBaseURL != "" && cfg.OllamaModel != "":
		reply, err := ai.CallAITalk(cfg.OllamaBaseURL, cfg.OllamaModel, playerInput, backstory, snippets, styleExamples, sensitivityHint, roomDisplayName)
		if err != nil || reply == "" {
			if err != nil {
				log.Printf("[Talk] Ollama fallback: %v", err)
			}
			narrative = buildTalkNarrative(playerRoom, target, p, cfg, playerInput)
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
	default:
		// 未設定雲端亦未設定 Ollama：模板敘事
		narrative = buildTalkNarrative(playerRoom, target, p, cfg, playerInput)
		npcReply = narrative
		if i := strings.Index(narrative, "搭話。"); i >= 0 {
			npcReply = narrative[i+len("搭話。"):]
		}
	}
	_ = event.Append(now, c.PlayerID, "talk", targetID)
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
	FlushConversationAndAppend(c.PlayerID, targetID, playerInput, npcReply, now)
	store.RecordPlayerTalkForRoomEcho(c.PlayerID, playerRoom, playerInput)
	_ = db.AdjustFavorability(targetID, c.PlayerID, db.FavTalk)
}
