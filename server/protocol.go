// Package server WebSocket 訊息格式：登入、房間視野、依出口移動。傳統 MUD 節點連接節點。
package server

// ClientMsg 客戶端送出的 JSON；type 決定行為。
type ClientMsg struct {
	Type        string `json:"type"`         // "login" | "create_character" | "move" | "ping" | "get_entity_status" | "print_topology_debug"
	PlayerID    string `json:"player_id"`    // login 必填；create_character 時為新角色 id
	Password    string `json:"password"`     // login / create_character 必填
	DisplayChar string `json:"display_char"` // create_character 選填，預設「我」
	Gender      string `json:"gender"`       // create_character 選填：「男」→M、「女」→F，預設 M
	Direction   string `json:"direction"`    // move 時出口方向（例："東"、"西"）
	EntityID    string `json:"entity_id"`    // get_entity_status 時選填，空則為自己
}

// ViewEntity 房間內單一實體，供前端顯示「誰在這裡」。
type ViewEntity struct {
	ID          string `json:"id"`
	Kind        string `json:"kind"`
	DisplayChar string `json:"display_char"`
}

// ExitView 單一出口，供前端顯示可點選的出口。
type ExitView struct {
	Direction   string `json:"direction"`
	ToRoomID    string `json:"to_room_id"`
	ToRoomName  string `json:"to_room_name"`
}

// RoomViewMsg 伺服器推送：當前房間描述、出口列表、同房實體、遊戲時間。
type RoomViewMsg struct {
	Type                  string       `json:"type"`
	RoomID                string       `json:"room_id"`
	RoomName              string       `json:"room_name"`
	Description           string       `json:"description"`
	Exits                 []ExitView   `json:"exits"`
	Entities              []ViewEntity `json:"entities"`
	ServerUnix               int64  `json:"server_unix"`                     // 此 view 送出時的真實 Unix 秒，供前端插值
	GameTimeSecSinceMidnight int    `json:"game_time_sec_since_midnight"`   // 遊戲內當日 0:00 起的秒數（0～86399）
	GameDaysSinceEpoch       int    `json:"game_days_since_epoch"`           // 自 epoch 起算的遊戲日數，奇點曆用
}

// MeMsg 伺服器推送：登入成功後回傳玩家 id、當前房間、體敏氣與四項資源、命途/本源/星盤用欄位（狀態與星盤分頁規格 §五）。
type MeMsg struct {
	Type            string    `json:"type"`
	PlayerID        string    `json:"player_id"`
	RoomID          string    `json:"room_id"`
	RoomName        string    `json:"room_name"`
	Vit             int       `json:"vit"`
	Qi              int       `json:"qi"`
	Dex             int       `json:"dex"`
	HpCur           int       `json:"hp_cur"`
	HpMax           int       `json:"hp_max"`
	InnerCur        int       `json:"inner_cur"`
	InnerMax        int       `json:"inner_max"`
	SpiritCur       int       `json:"spirit_cur"`
	SpiritMax       int       `json:"spirit_max"`
	StaminaCur      int       `json:"stamina_cur"`
	StaminaMax      int       `json:"stamina_max"`
	DisplayTitle    string   `json:"display_title,omitempty"`    // 命途；空則前端顯示「無名之輩」
	OriginSentence  string   `json:"origin_sentence,omitempty"` // 本源一句話，後端拼句，三軸不傳
	ActivatedNodes  []string          `json:"activated_nodes,omitempty"` // 星盤已貫通節點 ID
	TopologyCosts   []float64         `json:"topology_costs,omitempty"` // 760 條 Cost（僅自己，供星盤綁定）
	EquipmentSlots  map[string]string `json:"equipment_slots,omitempty"` // slot→item_id
	EquipmentNames  map[string]string `json:"equipment_names,omitempty"` // slot→item_name（前端顯示用）
}

// MovedMsg 伺服器推送：某角色經出口移動後廣播給所有人。
type MovedMsg struct {
	Type      string `json:"type"`       // "moved"
	PlayerID  string `json:"player_id"`
	RoomID    string `json:"room_id"`
	RoomName  string `json:"room_name"`
}

// ErrorMsg 伺服器推送：錯誤說明。
type ErrorMsg struct {
	Type    string `json:"type"`    // "error"
	Message string `json:"message"`
}

// BlockedMsg 伺服器推送：無此出口或移動被阻擋。
type BlockedMsg struct {
	Type      string `json:"type"`       // "blocked"
	Direction string `json:"direction"`
}

// PongMsg 伺服器回覆心跳，供前景 keep-alive 用。
type PongMsg struct {
	Type string `json:"type"` // "pong"
}

// TopologyDebugAckMsg 暫時除錯用：print_topology_debug 已於伺服器終端印出，回傳此 ack 供前端得知。
type TopologyDebugAckMsg struct {
	Type    string `json:"type"`    // "topology_debug"
	Message string `json:"message"`
}

// EntityStatusMsg 伺服器回傳：單一實體狀態分頁用（體敏氣、四項資源、鎂；自己時含命途/本源/星盤）。
type EntityStatusMsg struct {
	Type            string    `json:"type"`
	EntityID        string    `json:"entity_id"`
	DisplayChar     string    `json:"display_char"`
	Vit             int       `json:"vit"`
	Qi              int       `json:"qi"`
	Dex             int       `json:"dex"`
	HpCur           int       `json:"hp_cur"`
	HpMax           int       `json:"hp_max"`
	InnerCur        int       `json:"inner_cur"`
	InnerMax        int       `json:"inner_max"`
	SpiritCur       int       `json:"spirit_cur"`
	SpiritMax       int       `json:"spirit_max"`
	StaminaCur      int       `json:"stamina_cur"`
	StaminaMax      int       `json:"stamina_max"`
	Magnesium       *int      `json:"magnesium"`
	IsSelf          bool      `json:"is_self"`
	DisplayTitle    string    `json:"display_title,omitempty"`
	OriginSentence  string    `json:"origin_sentence,omitempty"`
	ActivatedNodes  []string          `json:"activated_nodes,omitempty"`
	TopologyCosts   []float64         `json:"topology_costs,omitempty"`
	EquipmentSlots  map[string]string `json:"equipment_slots,omitempty"`
	EquipmentNames  map[string]string `json:"equipment_names,omitempty"`
}
