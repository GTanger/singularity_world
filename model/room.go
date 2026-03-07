// Package model 提供與 db、store 共用的資料型別，避免 db ↔ store 形成 import cycle。
package model

// RoomObject 房間內可互動物件（Look/Read/Smell 等），name 須與房間描述中 〔〕 一致。
type RoomObject struct {
	ID        string            `json:"id"`
	Name      string            `json:"name"`
	Owner     string            `json:"owner"`
	Sockets   []string          `json:"sockets"`
	Responses map[string]string `json:"responses"`
}

// Room 單一房間節點。
type Room struct {
	ID          string        `json:"id"`
	Name        string        `json:"name"`
	Tags        []string      `json:"tags,omitempty"`
	Zone        string        `json:"zone,omitempty"`
	Description string        `json:"description"`
	Objects     []RoomObject  `json:"objects,omitempty"`
}

// Exit 單一出口：方向 → 目標房間。
type Exit struct {
	Direction  string `json:"direction"`
	ToRoomID   string `json:"to_room_id"`
	ToRoomName string `json:"to_room_name"`
}
