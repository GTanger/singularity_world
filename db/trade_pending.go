package db

import (
	"sync"
	"time"
)

// TradePending 玩家與 NPC 議價中的暫存（記憶體）；逾時自動失效。對齊第一版 10.17 Trade 流程。
type TradePending struct {
	NPCID   string
	ItemID  string
	ItemQty int // 本輪交易量（目前固定 1）
	AskMg   int
	FloorMg int
	Expires int64 // Unix 秒
}

const tradePendingTTL = 5 * time.Minute

var (
	tradeMu      sync.Mutex
	tradePending = make(map[string]*TradePending) // key: playerID + "|" + npcID
)

func tradeSessionKey(playerID, npcID string) string {
	return playerID + "|" + npcID
}

// TradeOfferSet 建立或覆寫一筆待成交報價。
func TradeOfferSet(playerID string, p *TradePending) {
	if p == nil {
		return
	}
	tradeMu.Lock()
	defer tradeMu.Unlock()
	p.Expires = time.Now().Add(tradePendingTTL).Unix()
	tradePending[tradeSessionKey(playerID, p.NPCID)] = p
}

// TradeOfferGet 取得未逾時的待報價；逾時則刪除並回 nil。
func TradeOfferGet(playerID, npcID string) *TradePending {
	tradeMu.Lock()
	defer tradeMu.Unlock()
	k := tradeSessionKey(playerID, npcID)
	p := tradePending[k]
	if p == nil {
		return nil
	}
	if time.Now().Unix() > p.Expires {
		delete(tradePending, k)
		return nil
	}
	return p
}

// TradeOfferClear 結束議價狀態（成交、拒絕或取消）。
func TradeOfferClear(playerID, npcID string) {
	tradeMu.Lock()
	defer tradeMu.Unlock()
	delete(tradePending, tradeSessionKey(playerID, npcID))
}
