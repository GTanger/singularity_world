package server

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"singularity_world/model"
	"singularity_world/store"
)

type roomEditorExit struct {
	Direction string `json:"direction"`
	To        string `json:"to"`
}

type roomEditorRoomFile struct {
	ID          string             `json:"id"`
	Name        string             `json:"name"`
	Description string             `json:"description"`
	Tags        []string           `json:"tags,omitempty"`
	Zone        string             `json:"zone,omitempty"`
	Exits       []roomEditorExit   `json:"exits,omitempty"`
	Objects     []model.RoomObject `json:"objects,omitempty"`
}

type roomEditorNode struct {
	ID          string             `json:"id"`
	Name        string             `json:"name"`
	Description string             `json:"description"`
	Tags        []string           `json:"tags,omitempty"`
	Zone        string             `json:"zone,omitempty"`
	Objects     []model.RoomObject `json:"objects,omitempty"`
}

type roomEditorEdge struct {
	From      string `json:"from"`
	To        string `json:"to"`
	Direction string `json:"direction"`
}

type roomEditorGraphResp struct {
	Nodes    []roomEditorNode         `json:"nodes"`
	Edges    []roomEditorEdge         `json:"edges"`
	Layout   map[string]roomEditorPos `json:"layout"`
	BasePath string                   `json:"base_path"`
}

type roomEditorPos struct {
	X float64 `json:"x"`
	Y float64 `json:"y"`
}

type roomEditorCreateReq struct {
	ID          string             `json:"id"`
	Name        string             `json:"name"`
	Description string             `json:"description"`
	Zone        string             `json:"zone"`
	Tags        []string           `json:"tags"`
	Objects     []model.RoomObject `json:"objects"`
	CloneFrom   string             `json:"clone_from"`
}

type roomEditorUpdateReq struct {
	Name        string             `json:"name"`
	Description string             `json:"description"`
	Zone        string             `json:"zone"`
	Tags        []string           `json:"tags"`
	Objects     []model.RoomObject `json:"objects"`
}

type roomEditorLinkReq struct {
	From       string `json:"from"`
	To         string `json:"to"`
	Direction  string `json:"direction"`
	Reverse    bool   `json:"reverse"`
	ReverseDir string `json:"reverse_direction"`
}

type roomEditorLayoutReq struct {
	Positions map[string]roomEditorPos `json:"positions"`
}

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

func roomEditorLayoutPath() string {
	return filepath.Join("data", "runtime", "room_editor_layout.json")
}

func loadRoomEditorLayout() map[string]roomEditorPos {
	path := roomEditorLayoutPath()
	b, err := os.ReadFile(path)
	if err != nil {
		return map[string]roomEditorPos{}
	}
	m := map[string]roomEditorPos{}
	if json.Unmarshal(b, &m) != nil {
		return map[string]roomEditorPos{}
	}
	return m
}

func saveRoomEditorLayout(m map[string]roomEditorPos) error {
	if m == nil {
		m = map[string]roomEditorPos{}
	}
	path := roomEditorLayoutPath()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	b, err := json.MarshalIndent(m, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, b, 0o644)
}

func getRoomsBasePath() string {
	return filepath.Join("data", "rooms")
}

func walkRoomFiles() (map[string]string, error) {
	base := getRoomsBasePath()
	idx := map[string]string{}
	err := filepath.WalkDir(base, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || filepath.Ext(d.Name()) != ".json" {
			return nil
		}
		if strings.HasPrefix(strings.TrimSuffix(d.Name(), filepath.Ext(d.Name())), "_") {
			return nil
		}
		b, err := os.ReadFile(path)
		if err != nil {
			return nil
		}
		var f roomEditorRoomFile
		if json.Unmarshal(b, &f) != nil || f.ID == "" {
			return nil
		}
		idx[f.ID] = path
		return nil
	})
	return idx, err
}

func readRoomFileByID(id string) (*roomEditorRoomFile, string, error) {
	idx, err := walkRoomFiles()
	if err != nil {
		return nil, "", err
	}
	path := idx[id]
	if path == "" {
		return nil, "", fmt.Errorf("room not found")
	}
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, "", err
	}
	var f roomEditorRoomFile
	if err := json.Unmarshal(b, &f); err != nil {
		return nil, "", err
	}
	return &f, path, nil
}

func writeRoomFile(path string, f *roomEditorRoomFile) error {
	if f == nil || f.ID == "" {
		return fmt.Errorf("invalid room")
	}
	b, err := json.MarshalIndent(f, "", "  ")
	if err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	return os.WriteFile(path, b, 0o644)
}

func normalizeIDForFile(id string) string {
	id = strings.TrimSpace(strings.ToLower(id))
	id = strings.ReplaceAll(id, " ", "_")
	id = strings.ReplaceAll(id, "/", "_")
	id = strings.ReplaceAll(id, "\\", "_")
	return id
}

func ensureStoreRoom(room *roomEditorRoomFile) {
	if store.Default == nil || room == nil {
		return
	}
	r := &model.Room{ID: room.ID, Name: room.Name, Description: room.Description, Tags: room.Tags, Zone: room.Zone, Objects: room.Objects}
	exits := make([]model.Exit, 0, len(room.Exits))
	for _, ex := range room.Exits {
		toName := ex.To
		if tr, _ := store.Default.GetRoom(ex.To); tr != nil && tr.Name != "" {
			toName = tr.Name
		}
		exits = append(exits, model.Exit{Direction: ex.Direction, ToRoomID: ex.To, ToRoomName: toName})
	}
	store.Default.UpsertRoomData(r, exits)
}

func handleRoomEditorGraph(w http.ResponseWriter) {
	ids := store.Default.RoomIDs()
	sort.Strings(ids)
	nodes := make([]roomEditorNode, 0, len(ids))
	edges := make([]roomEditorEdge, 0, 256)
	for _, id := range ids {
		r, _ := store.Default.GetRoom(id)
		if r == nil {
			continue
		}
		nodes = append(nodes, roomEditorNode{ID: r.ID, Name: r.Name, Description: r.Description, Zone: r.Zone, Tags: r.Tags, Objects: r.Objects})
		ex, _ := store.Default.GetExitsForRoom(id)
		for _, e := range ex {
			edges = append(edges, roomEditorEdge{From: id, To: e.ToRoomID, Direction: e.Direction})
		}
	}
	_ = json.NewEncoder(w).Encode(roomEditorGraphResp{Nodes: nodes, Edges: edges, Layout: loadRoomEditorLayout(), BasePath: getRoomsBasePath()})
}

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
	to, _, err := readRoomFileByID(req.To)
	if err != nil {
		http.Error(w, `{"error":"to room not found"}`, http.StatusNotFound)
		return
	}
	dir := strings.TrimSpace(req.Direction)
	if dir == "" {
		dir = to.Name
	}
	from.Exits = addOrReplaceExit(from.Exits, roomEditorExit{Direction: dir, To: req.To})
	if err := writeRoomFile(fromPath, from); err != nil {
		http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
		return
	}
	ensureStoreRoom(from)
	if req.Reverse {
		rev, revPath, err := readRoomFileByID(req.To)
		if err == nil {
			rd := strings.TrimSpace(req.ReverseDir)
			if rd == "" {
				rd = from.Name
			}
			rev.Exits = addOrReplaceExit(rev.Exits, roomEditorExit{Direction: rd, To: req.From})
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
