// 房間編輯器：POST reload — 從磁碟 JSON 重新載入房間與出口。
package server

import (
	"net/http"

	"singularity_world/store"
)

func handleRoomEditorReload(w http.ResponseWriter) {
	if err := store.Default.ReloadRooms(); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}
