// Package model 提供與 db、store 共用的資料型別，避免 db ↔ store 形成 import cycle。
package model

// Room 單一房間節點。
type Room struct {
	ID          string   `json:"id"`
	Name        string   `json:"name"`
	Tags        []string `json:"tags,omitempty"`
	Zone        string   `json:"zone,omitempty"`
	Description string   `json:"description"`
}

// Exit 單一出口：方向 → 目標房間。
type Exit struct {
	Direction  string `json:"direction"`
	ToRoomID   string `json:"to_room_id"`
	ToRoomName string `json:"to_room_name"`
}
