// 房間編輯器：GET/POST groups（群組持久化）。
package server

import (
	"encoding/json"
	"net/http"
)

func handleRoomEditorGroupsGet(w http.ResponseWriter) {
	groups := loadEditorGroups()
	_ = json.NewEncoder(w).Encode(groups)
}

func handleRoomEditorGroupsPost(w http.ResponseWriter, r *http.Request) {
	var groups [][]string
	if err := json.NewDecoder(r.Body).Decode(&groups); err != nil {
		http.Error(w, `{"error":"invalid json"}`, http.StatusBadRequest)
		return
	}
	if err := saveEditorGroups(groups); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
}
