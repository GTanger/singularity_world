// 房間編輯器：房間 POST／PUT／DELETE。
package server

import (
	"encoding/json"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"singularity_world/store"
)

func handleRoomEditorCreate(w http.ResponseWriter, r *http.Request) {
	var req roomEditorCreateReq
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil || strings.TrimSpace(req.ID) == "" {
		http.Error(w, `{"error":"invalid request"}`, http.StatusBadRequest)
		return
	}
	idx, err := walkRoomFiles()
	if err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	if idx[req.ID] != "" {
		http.Error(w, `{"error":"room id exists"}`, http.StatusConflict)
		return
	}
	f := &roomEditorRoomFile{ID: req.ID, Name: req.Name, Description: req.Description, Zone: req.Zone, Tags: req.Tags, Objects: req.Objects, Exits: []roomEditorExit{}}
	if f.Name == "" {
		f.Name = req.ID
	}
	if req.CloneFrom != "" {
		src, _, err := readRoomFileByID(req.CloneFrom)
		if err == nil && src != nil {
			f.Description = src.Description
			f.Tags = src.Tags
			f.Zone = src.Zone
			f.Objects = src.Objects
		}
	}
	outPath := filepath.Join(getRoomsBasePath(), "editor", normalizeIDForFile(req.ID)+".json")
	if err := writeRoomFile(outPath, f); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	ensureStoreRoom(f)
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true, "id": f.ID, "path": outPath})
}

func handleRoomEditorUpdate(w http.ResponseWriter, r *http.Request, id string) {
	var req roomEditorUpdateReq
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error":"invalid json"}`, http.StatusBadRequest)
		return
	}
	f, path, err := readRoomFileByID(id)
	if err != nil {
		http.Error(w, `{"error":"room not found"}`, http.StatusNotFound)
		return
	}
	f.Name = req.Name
	if f.Name == "" {
		f.Name = f.ID
	}
	f.Description = req.Description
	f.Zone = req.Zone
	f.Tags = req.Tags
	f.Objects = req.Objects
	if err := writeRoomFile(path, f); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	ensureStoreRoom(f)
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true, "id": id})
}

func handleRoomEditorDelete(w http.ResponseWriter, id string) {
	f, path, err := readRoomFileByID(id)
	if err != nil || f == nil {
		http.Error(w, `{"error":"room not found"}`, http.StatusNotFound)
		return
	}
	if err := os.Remove(path); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	idx, _ := walkRoomFiles()
	for rid := range idx {
		rf, p, err := readRoomFileByID(rid)
		if err != nil || rf == nil {
			continue
		}
		filtered := make([]roomEditorExit, 0, len(rf.Exits))
		changed := false
		for _, ex := range rf.Exits {
			if ex.To == id {
				changed = true
				continue
			}
			filtered = append(filtered, ex)
		}
		if changed {
			rf.Exits = filtered
			_ = writeRoomFile(p, rf)
			ensureStoreRoom(rf)
		}
	}
	store.Default.DeleteRoomData(id)
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true, "deleted": id})
}
