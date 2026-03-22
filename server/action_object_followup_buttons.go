// do_action：物件動作後，其餘可點動詞與 Look 時的 Move 目標 fallback。
package server

import "singularity_world/db"

func objectFollowupActionButtons(obj *db.RoomObject, playerRoom, action string) (others []string, moveTargetID string) {
	others = make([]string, 0, len(obj.Sockets))
	for _, s := range obj.Sockets {
		if s != action {
			others = append(others, s)
		}
	}
	moveTargetID = ""
	if action == "Look" && len(others) == 0 && obj.Responses["Look"] == "" {
		roomObjs := db.GetObjectsInRoom(playerRoom)
		idx := -1
		for i := range roomObjs {
			if roomObjs[i].ID == obj.ID {
				idx = i
				break
			}
		}
		if idx >= 0 {
			for _, delta := range []struct{ start, end int }{{idx + 1, len(roomObjs)}, {0, idx}} {
				for i := delta.start; i < delta.end; i++ {
					o := &roomObjs[i]
					if o.MoveToRoomID != "" && db.ObjectHasSocket(o, "Move") {
						moveTargetID = o.ID
						break
					}
				}
				if moveTargetID != "" {
					break
				}
			}
		} else {
			for i := range roomObjs {
				o := &roomObjs[i]
				if o.MoveToRoomID != "" && db.ObjectHasSocket(o, "Move") {
					moveTargetID = o.ID
					break
				}
			}
		}
		if moveTargetID != "" {
			others = []string{"Move"}
		}
	}
	return others, moveTargetID
}
