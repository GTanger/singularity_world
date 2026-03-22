// do_action：房間物件動作的敘事字串。
package server

import "singularity_world/db"

func buildObjectActionNarrative(obj *db.RoomObject, action string) string {
	narrative := db.ObjectResponse(obj, action)
	if narrative == "" {
		narrative = "你對【" + obj.Name + "】執行了「" + action + "」，但似乎沒有什麼特別的反應。"
	}
	return narrative
}
