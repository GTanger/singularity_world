// Package db NPC 長期記憶（archival）薄封裝；型別 store.ArchivalEntry 定義在 store 包。
package db

import (
	"strings"
	"time"

	"singularity_world/store"
)

// InsertArchival 寫入一條長期記憶。
func InsertArchival(entityID, content, tag string) error {
	if store.Default == nil {
		return nil
	}
	entry := store.ArchivalEntry{
		EntityID:  entityID,
		Content:   content,
		Tag:       tag,
		CreatedAt: time.Now().Unix(),
	}
	return store.Default.AppendArchival(entry)
}

// SearchArchival 依 entity_id 取最近 topK 條，可選關鍵字過濾（簡易版 strings.Contains）。回傳 Content 字串 slice。
func SearchArchival(entityID, query string, topK int) []string {
	if store.Default == nil {
		return nil
	}
	entries := store.Default.GetArchivalByEntity(entityID)
	var out []string
	for _, e := range entries {
		if query != "" && !strings.Contains(e.Content, query) {
			continue
		}
		out = append(out, e.Content)
		if len(out) >= topK {
			break
		}
	}
	return out
}
