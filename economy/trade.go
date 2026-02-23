// Package economy 負責交易與鎂流轉，對齊詞盤細規 §一鎂產消。
package economy

// TransferMagnesium 將 fromID 的 amount 鎂轉給 toID；呼叫方需負責 DB 交易與餘額檢查。
// 第一版可由 db 層實作：讀兩邊 magnesium，扣加後寫回。此處定義介面供 combat/任務 等呼叫。
func TransferMagnesium(fromID, toID string, amount int) {
	_ = fromID
	_ = toID
	_ = amount
}
