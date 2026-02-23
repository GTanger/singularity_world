// Package entity 負責玩家／NPC／建築等實體結構與插頭插座邏輯。本檔為建築／靜態物。
package entity

// Building 為地圖上的建築或靜態物，佔格、可阻擋或可接觸；對齊決策 002、003。
type Building struct {
	ID          string
	X           int
	Y           int
	Blocking    bool     // true 表示不可進入
	SocketList  []string // 可執行動作，空則僅擋路
	DisplayChar string
}

// Sockets 回傳此建築的插座清單；若無則為僅擋路實體。
func (b *Building) Sockets() []string {
	return b.SocketList
}
