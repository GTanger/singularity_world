// Package server 房間管理 API：列表、新增、修改、刪除房間與出口。
package server

import (
	"database/sql"
	"encoding/json"
	"net/http"
	"strings"

	"singularity_world/db"
)

// HandleRoomsAPI 處理 /api/rooms 與 /api/rooms/:id、/api/rooms/:id/exits。
func HandleRoomsAPI(database *sql.DB, w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	path := strings.TrimPrefix(r.URL.Path, "/api/rooms")
	path = strings.Trim(path, "/")
	parts := strings.SplitN(path, "/", 3)

	switch r.Method {
	case http.MethodGet:
		if path == "" {
			listRooms(database, w)
			return
		}
		http.Error(w, `{"error":"not found"}`, http.StatusNotFound)
	case http.MethodPost:
		if path == "" {
			createRoom(database, w, r)
			return
		}
		if len(parts) == 2 && parts[0] != "" && parts[1] == "exits" {
			addExit(database, w, r, parts[0])
			return
		}
		http.Error(w, `{"error":"bad request"}`, http.StatusBadRequest)
	case http.MethodPut:
		if len(parts) == 1 && parts[0] != "" {
			updateRoom(database, w, r, parts[0])
			return
		}
		http.Error(w, `{"error":"bad request"}`, http.StatusBadRequest)
	case http.MethodDelete:
		if len(parts) == 1 && parts[0] != "" {
			deleteRoom(database, w, parts[0])
			return
		}
		if len(parts) == 3 && parts[0] != "" && parts[1] == "exits" && parts[2] != "" {
			removeExit(database, w, parts[0], parts[2])
			return
		}
		http.Error(w, `{"error":"bad request"}`, http.StatusBadRequest)
	default:
		http.Error(w, `{"error":"method not allowed"}`, http.StatusMethodNotAllowed)
	}
}

func listRooms(database *sql.DB, w http.ResponseWriter) {
	list, err := db.ListAllRooms(database)
	if err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	_ = json.NewEncoder(w).Encode(map[string]interface{}{"rooms": list})
}

func createRoom(database *sql.DB, w http.ResponseWriter, r *http.Request) {
	var body struct {
		ID          string `json:"id"`
		Name        string `json:"name"`
		Description string `json:"description"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil || body.ID == "" {
		http.Error(w, `{"error":"need id, name, description"}`, http.StatusBadRequest)
		return
	}
	if body.Name == "" {
		body.Name = body.ID
	}
	if err := db.CreateRoom(database, body.ID, body.Name, body.Description); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}
	w.WriteHeader(http.StatusCreated)
	_ = json.NewEncoder(w).Encode(map[string]string{"id": body.ID})
}

func updateRoom(database *sql.DB, w http.ResponseWriter, r *http.Request, id string) {
	var body struct {
		Name        string `json:"name"`
		Description string `json:"description"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		http.Error(w, `{"error":"invalid json"}`, http.StatusBadRequest)
		return
	}
	if err := db.UpdateRoom(database, id, body.Name, body.Description); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}
	_ = json.NewEncoder(w).Encode(map[string]string{"id": id})
}

func deleteRoom(database *sql.DB, w http.ResponseWriter, id string) {
	if id == "lobby" {
		http.Error(w, `{"error":"cannot delete lobby"}`, http.StatusBadRequest)
		return
	}
	if err := db.DeleteRoom(database, id); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	_ = json.NewEncoder(w).Encode(map[string]string{"deleted": id})
}

func addExit(database *sql.DB, w http.ResponseWriter, r *http.Request, fromID string) {
	var body struct {
		Direction string `json:"direction"`
		ToRoomID  string `json:"to_room_id"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil || body.Direction == "" || body.ToRoomID == "" {
		http.Error(w, `{"error":"need direction, to_room_id"}`, http.StatusBadRequest)
		return
	}
	if err := db.AddExit(database, fromID, body.Direction, body.ToRoomID); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}
	w.WriteHeader(http.StatusCreated)
	_ = json.NewEncoder(w).Encode(map[string]string{"from": fromID, "direction": body.Direction, "to": body.ToRoomID})
}

func removeExit(database *sql.DB, w http.ResponseWriter, fromID, direction string) {
	if err := db.RemoveExit(database, fromID, direction); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}
	_ = json.NewEncoder(w).Encode(map[string]string{"removed": fromID + " " + direction})
}
