// 房間編輯器：PUT layout（座標持久化）。
package server

import (
	"encoding/json"
	"net/http"
)

func handleRoomEditorLayout(w http.ResponseWriter, r *http.Request) {
	var req roomEditorLayoutReq
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error":"invalid json"}`, http.StatusBadRequest)
		return
	}
	if err := saveRoomEditorLayout(req.Positions); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
}
