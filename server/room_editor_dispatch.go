// 房間心智圖編輯器 API：HTTP 路由分派。
package server

import (
	"net/http"
	"strings"

	"singularity_world/store"
)

func HandleRoomEditorAPI(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.Header().Set("Cache-Control", "no-cache, max-age=0, must-revalidate")
	if store.Default == nil {
		http.Error(w, `{"error":"store not initialized"}`, http.StatusServiceUnavailable)
		return
	}
	path := strings.TrimPrefix(r.URL.Path, "/api/room-editor")
	path = strings.Trim(path, "/")
	parts := strings.Split(path, "/")
	if path == "graph" && r.Method == http.MethodGet {
		handleRoomEditorGraph(w)
		return
	}
	if path == "room" && r.Method == http.MethodPost {
		handleRoomEditorCreate(w, r)
		return
	}
	if len(parts) == 2 && parts[0] == "room" {
		id := parts[1]
		switch r.Method {
		case http.MethodPut:
			handleRoomEditorUpdate(w, r, id)
			return
		case http.MethodDelete:
			handleRoomEditorDelete(w, id)
			return
		}
	}
	if path == "link" {
		switch r.Method {
		case http.MethodPost:
			handleRoomEditorLinkCreate(w, r)
			return
		case http.MethodDelete:
			handleRoomEditorLinkDelete(w, r)
			return
		}
	}
	if path == "layout" && r.Method == http.MethodPut {
		handleRoomEditorLayout(w, r)
		return
	}
	http.Error(w, `{"error":"not found"}`, http.StatusNotFound)
}
