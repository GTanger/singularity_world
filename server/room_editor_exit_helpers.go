// 房間編輯器：出口列表與 Move 物件同步。
package server

import (
	"fmt"
	"slices"
	"strings"

	"singularity_world/model"
)

func addOrReplaceExit(list []roomEditorExit, ex roomEditorExit) []roomEditorExit {
	if strings.TrimSpace(ex.Direction) == "" || strings.TrimSpace(ex.To) == "" {
		return list
	}
	for i := range list {
		if list[i].Direction == ex.Direction {
			list[i] = ex
			return list
		}
	}
	return append(list, ex)
}

// ensureMoveObjectForExit 為「玩家點物件 Move 能切房」補上 move_to_room_id：與 exits 一併維護。
// 優先沿用已有 id 或 move_to_room_id 指向 toRoomID 的物件，否則新增（id 預設為目標房 id）。
func ensureMoveObjectForExit(room *roomEditorRoomFile, toRoomID, direction, targetDisplayName string) {
	if room == nil || strings.TrimSpace(toRoomID) == "" {
		return
	}
	dir := strings.TrimSpace(direction)
	if dir == "" {
		dir = toRoomID
	}
	targetName := strings.TrimSpace(targetDisplayName)
	if targetName == "" {
		targetName = toRoomID
	}
	defaultMoveText := fmt.Sprintf("你前往「%s」。", targetName)

	for i := range room.Objects {
		o := &room.Objects[i]
		if o.MoveToRoomID != toRoomID && o.ID != toRoomID {
			continue
		}
		o.MoveToRoomID = toRoomID
		if o.ID == "" {
			o.ID = toRoomID
		}
		if o.Name == "" {
			o.Name = dir
		}
		if !slices.Contains(o.Sockets, "Move") {
			o.Sockets = append(o.Sockets, "Move")
		}
		if o.Responses == nil {
			o.Responses = map[string]string{}
		}
		if strings.TrimSpace(o.Responses["Move"]) == "" {
			o.Responses["Move"] = defaultMoveText
		}
		return
	}

	room.Objects = append(room.Objects, model.RoomObject{
		ID:           toRoomID,
		Name:         dir,
		Owner:        "",
		Sockets:      []string{"Move"},
		Responses:    map[string]string{"Move": defaultMoveText},
		MoveToRoomID: toRoomID,
	})
}
