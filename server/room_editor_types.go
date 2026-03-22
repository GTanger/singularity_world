// 房間心智圖編輯器 API：請求／回應 JSON 型別。
package server

import "singularity_world/model"

type roomEditorExit struct {
	Direction string `json:"direction"`
	To        string `json:"to"`
}

type roomEditorRoomFile struct {
	ID          string             `json:"id"`
	Name        string             `json:"name"`
	Description string             `json:"description"`
	Tags        []string           `json:"tags,omitempty"`
	Zone        string             `json:"zone,omitempty"`
	Exits       []roomEditorExit   `json:"exits,omitempty"`
	Objects     []model.RoomObject `json:"objects,omitempty"`
}

type roomEditorNode struct {
	ID          string             `json:"id"`
	Name        string             `json:"name"`
	Description string             `json:"description"`
	Tags        []string           `json:"tags,omitempty"`
	Zone        string             `json:"zone,omitempty"`
	Objects     []model.RoomObject `json:"objects,omitempty"`
}

type roomEditorEdge struct {
	From      string `json:"from"`
	To        string `json:"to"`
	Direction string `json:"direction"`
}

type roomEditorGraphResp struct {
	Nodes    []roomEditorNode         `json:"nodes"`
	Edges    []roomEditorEdge         `json:"edges"`
	Layout   map[string]roomEditorPos `json:"layout"`
	BasePath string                   `json:"base_path"`
}

type roomEditorPos struct {
	X float64 `json:"x"`
	Y float64 `json:"y"`
}

type roomEditorCreateReq struct {
	ID          string             `json:"id"`
	Name        string             `json:"name"`
	Description string             `json:"description"`
	Zone        string             `json:"zone"`
	Tags        []string           `json:"tags"`
	Objects     []model.RoomObject `json:"objects"`
	CloneFrom   string             `json:"clone_from"`
}

type roomEditorUpdateReq struct {
	Name        string             `json:"name"`
	Description string             `json:"description"`
	Zone        string             `json:"zone"`
	Tags        []string           `json:"tags"`
	Objects     []model.RoomObject `json:"objects"`
}

type roomEditorLinkReq struct {
	From       string `json:"from"`
	To         string `json:"to"`
	Direction  string `json:"direction"`
	Reverse    bool   `json:"reverse"`
	ReverseDir string `json:"reverse_direction"`
}

type roomEditorLayoutReq struct {
	Positions map[string]roomEditorPos `json:"positions"`
}
