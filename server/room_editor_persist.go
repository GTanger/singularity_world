// 房間編輯器：layout 與房間 JSON 檔讀寫、store 同步。
package server

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"singularity_world/model"
	"singularity_world/store"
)

func roomEditorLayoutPath() string {
	return filepath.Join("data", "runtime", "room_editor_layout.json")
}

func roomEditorGroupsPath() string {
	return filepath.Join("data", "runtime", "editor_groups.json")
}

func loadEditorGroups() [][]string {
	b, err := os.ReadFile(roomEditorGroupsPath())
	if err != nil {
		return [][]string{}
	}
	var g [][]string
	if json.Unmarshal(b, &g) != nil {
		return [][]string{}
	}
	return g
}

func saveEditorGroups(groups [][]string) error {
	if groups == nil {
		groups = [][]string{}
	}
	path := roomEditorGroupsPath()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	b, err := json.MarshalIndent(groups, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, b, 0o644)
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
