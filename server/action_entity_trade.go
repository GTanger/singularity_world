// do_action：對同房實體 Trade；若無法取得買家則 sendError 並回傳 true。
package server

import (
	"fmt"
	"log"
	"strconv"
	"strings"

	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/event"
	"singularity_world/gametext"
)

func doActionEntityTrade(c *Client, msg *ClientMsg, now int64, targetID string, target *entity.Character) bool {
	tradeTargetName := target.DisplayTitle
	if tradeTargetName == "" {
		tradeTargetName = target.ID
	}
	playerTradeInput := strings.TrimSpace(msg.PlayerInput)
	if isTradeRejectInput(playerTradeInput) {
		db.TradeOfferClear(c.PlayerID, targetID)
		c.Send <- mustJSON(ActionResultMsg{
			Type: "action_result", Action: "Trade",
			TargetID: target.ID, TargetName: tradeTargetName,
			Narrative: "你中止了與【" + tradeTargetName + "】的交易。", Success: true,
		})
		return false
	}
	pending := db.TradeOfferGet(c.PlayerID, targetID)
	if pending == nil {
		if playerTradeInput != "" {
			c.Send <- mustJSON(ActionResultMsg{
				Type: "action_result", Action: "Trade",
				TargetID: target.ID, TargetName: tradeTargetName,
				Narrative: "對方尚未開價。請先點【交易】取得報價，再在輸入欄填寫出價（鎂）。", Success: true,
			})
			return false
		}
		itemID, okOffer := db.PickNPCTradeOffer(target.Inventory)
		if !okOffer {
			narrative := "你向【" + tradeTargetName + "】提出交易，對方表示目前暫無可交易之物。"
			c.Send <- mustJSON(ActionResultMsg{
				Type: "action_result", Action: "Trade",
				TargetID: target.ID, TargetName: tradeTargetName,
				Narrative: narrative, Success: true,
			})
			return false
		}
		ask := db.DefaultTradeAskMg(itemID)
		floor := db.TradeFloorFromAsk(ask)
		itemName := itemID
		if n, _, _, _, _, err := db.GetItemInfo(itemID); err == nil && n != "" {
			itemName = n
		}
		db.TradeOfferSet(c.PlayerID, &db.TradePending{
			NPCID: targetID, ItemID: itemID, ItemQty: 1, AskMg: ask, FloorMg: floor,
		})
		narrative := fmt.Sprintf(
			"【%s】願意向你賣出「%s」一份，開價 %d 鎂（議價底線約 %d 鎂）。請再點【交易】，在欄位輸入你願付的鎂數；達開價則一口價成交，介於底線與開價之間則以你的出價成交。輸入「拒絕」可取消。",
			tradeTargetName, itemName, ask, floor,
		)
		c.Send <- mustJSON(ActionResultMsg{
			Type: "action_result", Action: "Trade",
			TargetID: target.ID, TargetName: tradeTargetName,
			Narrative: narrative, Success: true,
		})
		return false
	}
	if playerTradeInput == "" {
		c.Send <- mustJSON(ActionResultMsg{
			Type: "action_result", Action: "Trade",
			TargetID: target.ID, TargetName: tradeTargetName,
			Narrative: "請輸入你願付的鎂數（整數），或輸入「拒絕」取消交易。", Success: true,
		})
		return false
	}
	offer, errParseOffer := strconv.Atoi(playerTradeInput)
	if errParseOffer != nil || offer < 0 {
		c.Send <- mustJSON(ActionResultMsg{
			Type: "action_result", Action: "Trade",
			TargetID: target.ID, TargetName: tradeTargetName,
			Narrative: "請輸入有效的鎂數（非負整數），或「拒絕」。", Success: true,
		})
		return false
	}
	buyer, _ := db.GetEntity(c.PlayerID)
	if buyer == nil {
		sendError(c, gametext.Client("get_self_failed"))
		return true
	}
	var paid int
	if offer >= pending.AskMg {
		paid = pending.AskMg
	} else if offer >= pending.FloorMg {
		paid = offer
	} else {
		c.Send <- mustJSON(ActionResultMsg{
			Type: "action_result", Action: "Trade",
			TargetID: target.ID, TargetName: tradeTargetName,
			Narrative: fmt.Sprintf("【%s】搖頭：至少要 %d 鎂才肯點頭。", tradeTargetName, pending.FloorMg),
			Success:   true,
		})
		return false
	}
	if buyer.Magnesium < paid {
		c.Send <- mustJSON(ActionResultMsg{
			Type: "action_result", Action: "Trade",
			TargetID: target.ID, TargetName: tradeTargetName,
			Narrative: "你的鎂不足，無法成交。", Success: true,
		})
		return false
	}
	if err := db.TransferMagnesium(c.PlayerID, targetID, paid); err != nil {
		log.Printf("[Trade] transfer: %v", err)
		c.Send <- mustJSON(ActionResultMsg{
			Type: "action_result", Action: "Trade",
			TargetID: target.ID, TargetName: tradeTargetName,
			Narrative: "成交時鎂轉帳失敗，請稍後再試。", Success: false,
		})
		return false
	}
	_ = db.AddToInventory(c.PlayerID, pending.ItemID, pending.ItemQty)
	_ = db.RemoveFromInventory(targetID, pending.ItemID, pending.ItemQty)
	db.TradeOfferClear(c.PlayerID, targetID)
	itemName := pending.ItemID
	if n, _, _, _, _, err := db.GetItemInfo(pending.ItemID); err == nil && n != "" {
		itemName = n
	}
	var narrative string
	if offer >= pending.AskMg {
		narrative = fmt.Sprintf("你以開價 %d 鎂買下「%s」。", paid, itemName)
	} else {
		narrative = fmt.Sprintf("一番議價後，你以 %d 鎂買下「%s」。", paid, itemName)
	}
	_ = event.Append(now, c.PlayerID, "trade", targetID)
	c.Send <- mustJSON(ActionResultMsg{
		Type: "action_result", Action: "Trade",
		TargetID: target.ID, TargetName: tradeTargetName,
		Narrative: narrative, Success: true,
	})
	pushRefresh(c)
	return false
}
