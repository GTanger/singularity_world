// 房間編輯器：連結 POST／DELETE。
package server

import (
	"encoding/json"
	"net/http"
	"strings"
)

func handleRoomEditorLinkCreate(w http.ResponseWriter, r *http.Request) {
	var req roomEditorLinkReq
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil || req.From == "" || req.To == "" {
		http.Error(w, `{"error":"invalid request"}`, http.StatusBadRequest)
		return
	}
	from, fromPath, err := readRoomFileByID(req.From)
	if err != nil {
		http.Error(w, `{"error":"from room not found"}`, http.StatusNotFound)
		return
	}
	to, toPath, err := readRoomFileByID(req.To)
	if err != nil {
		http.Error(w, `{"error":"to room not found"}`, http.StatusNotFound)
		return
	}
	dir := strings.TrimSpace(req.Direction)
	if dir == "" {
		dir = to.Name
	}
	from.Exits = addOrReplaceExit(from.Exits, roomEditorExit{Direction: dir, To: req.To})
	ensureMoveObjectForExit(from, req.To, dir, to.Name)
	if err := writeRoomFile(fromPath, from); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	ensureStoreRoom(from)
	if req.Reverse {
		rev := to
		revPath := toPath
		if rev != nil && revPath != "" {
			rd := strings.TrimSpace(req.ReverseDir)
			if rd == "" {
				rd = from.Name
			}
			rev.Exits = addOrReplaceExit(rev.Exits, roomEditorExit{Direction: rd, To: req.From})
			ensureMoveObjectForExit(rev, req.From, rd, from.Name)
			_ = writeRoomFile(revPath, rev)
			ensureStoreRoom(rev)
		}
	}
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
}

func handleRoomEditorLinkDelete(w http.ResponseWriter, r *http.Request) {
	var req roomEditorLinkReq
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil || req.From == "" || req.To == "" {
		http.Error(w, `{"error":"invalid request"}`, http.StatusBadRequest)
		return
	}
	from, p, err := readRoomFileByID(req.From)
	if err != nil {
		http.Error(w, `{"error":"from room not found"}`, http.StatusNotFound)
		return
	}
	filtered := make([]roomEditorExit, 0, len(from.Exits))
	for _, ex := range from.Exits {
		if ex.To != req.To {
			filtered = append(filtered, ex)
		}
	}
	from.Exits = filtered
	_ = writeRoomFile(p, from)
	ensureStoreRoom(from)
	if req.Reverse {
		rev, rp, err := readRoomFileByID(req.To)
		if err == nil {
			rf := make([]roomEditorExit, 0, len(rev.Exits))
			for _, ex := range rev.Exits {
				if ex.To != req.From {
					rf = append(rf, ex)
				}
			}
			rev.Exits = rf
			_ = writeRoomFile(rp, rev)
			ensureStoreRoom(rev)
		}
	}
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
}
