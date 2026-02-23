// Package entity 負責插頭插座匹配邏輯。本檔為動詞與名詞插座匹配，對齊決策 002。
package entity

// HasSocket 檢查名詞的插座清單是否包含動詞 verb。
// 參數：sockets 為名詞對外開放的動詞清單；verb 為玩家意圖（動詞）。
// 回傳：true 表示可執行。無副作用。
func HasSocket(sockets []string, verb string) bool {
	for _, s := range sockets {
		if s == verb {
			return true
		}
	}
	return false
}
