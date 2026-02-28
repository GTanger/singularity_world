// Package entity 負責玩家／NPC／建築等實體結構與插頭插座邏輯。本檔為角色（玩家/NPC）共用結構。
package entity

// Character 為玩家與 NPC 共用實體，對齊人物角色模板第一版最小集（id、位置、移動、體質/氣脈/靈敏、鎂、觀測時間）。
type Character struct {
	ID              string
	Kind            string // "player" | "npc"
	DisplayChar     string
	X               int
	Y               int
	MoveState       string // "idle" | "moving"
	TargetX         *int
	TargetY         *int
	WalkOrRun       string
	MoveStartedAt   *int64
	Vit             int
	Qi              int
	Dex             int
	Magnesium       int
	LastObservedAt  *int64
	CreatedAt       int64
	Gender          string // "M" | "F"；空字串表示未設
	SoulSeed        *int64 // 創角時寫入，唯一決定三軸與 760 邊權；nil 表示舊資料
}

// Sockets 回傳此角色對外開放的動詞清單（插頭插座最小集：Talk, Attack, Look）。
// 參數：無。回傳：[]string。無副作用。
func (c *Character) Sockets() []string {
	return []string{"Talk", "Attack", "Look"}
}
