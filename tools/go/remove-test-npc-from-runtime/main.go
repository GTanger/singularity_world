// 一次性清洗：從 runtime JSON 移除「試話」實體的殘留引用（不影響職稱「服務生」）。
// 請在關閉伺服器後執行（專案根目錄）：go run ./scripts/remove-test-npc-from-runtime
package main

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
	"strings"
)

const testNPCID = "試話"

func main() {
	rt := filepath.Join("data", "runtime")

	// 1. npc_memory.json：移除 entity_id == 試話 的條目
	memPath := filepath.Join(rt, "npc_memory.json")
	if raw, err := os.ReadFile(memPath); err == nil {
		var m struct {
			Entries []struct {
				EntityID     string `json:"entity_id"`
				SubjectID    string `json:"subject_id"`
				MeetCount    int    `json:"meet_count"`
				Favorability int    `json:"favorability"`
			} `json:"entries"`
		}
		if json.Unmarshal(raw, &m) == nil {
			var out []interface{}
			for _, e := range m.Entries {
				if e.EntityID != testNPCID {
					out = append(out, map[string]interface{}{
						"entity_id": e.EntityID, "subject_id": e.SubjectID,
						"meet_count": e.MeetCount, "favorability": e.Favorability,
					})
				}
			}
			enc, _ := json.MarshalIndent(map[string]interface{}{"entries": out}, "", "  ")
			if err := os.WriteFile(memPath, enc, 0644); err != nil {
				log.Printf("%s: %v", memPath, err)
			} else {
				log.Printf("cleaned entity_id 試話 from %s", memPath)
			}
		}
	}

	// 2. entity_rooms.json：移除 entity_id == 試話
	erPath := filepath.Join(rt, "entity_rooms.json")
	if raw, err := os.ReadFile(erPath); err == nil {
		var m struct {
			Entries []struct {
				EntityID string `json:"entity_id"`
				RoomID   string `json:"room_id"`
			} `json:"entries"`
		}
		if json.Unmarshal(raw, &m) == nil {
			var out []interface{}
			for _, e := range m.Entries {
				if e.EntityID != testNPCID {
					out = append(out, map[string]interface{}{"entity_id": e.EntityID, "room_id": e.RoomID})
				}
			}
			enc, _ := json.MarshalIndent(map[string]interface{}{"entries": out}, "", "  ")
			if err := os.WriteFile(erPath, enc, 0644); err != nil {
				log.Printf("%s: %v", erPath, err)
			} else {
				log.Printf("cleaned entity_id 試話 from %s", erPath)
			}
		}
	}

	// 3. npc_npc_summaries.json：移除 key 含 試話
	sumPath := filepath.Join(rt, "npc_npc_summaries.json")
	if raw, err := os.ReadFile(sumPath); err == nil {
		var m struct {
			Summaries map[string]string `json:"summaries"`
		}
		if json.Unmarshal(raw, &m) == nil && m.Summaries != nil {
			n := 0
			for k := range m.Summaries {
				if strings.Contains(k, testNPCID) {
					delete(m.Summaries, k)
					n++
				}
			}
			if n > 0 {
				enc, _ := json.MarshalIndent(m, "", "  ")
				if err := os.WriteFile(sumPath, enc, 0644); err != nil {
					log.Printf("%s: %v", sumPath, err)
				} else {
					log.Printf("removed %d 試話 keys from %s", n, sumPath)
				}
			}
		}
	}

	// 4. npc_rumors.json：移除 key 含 試話 的 rumor
	rumPath := filepath.Join(rt, "npc_rumors.json")
	if raw, err := os.ReadFile(rumPath); err == nil {
		var m struct {
			Rumors map[string]interface{} `json:"rumors"`
		}
		if json.Unmarshal(raw, &m) == nil && m.Rumors != nil {
			n := 0
			for k := range m.Rumors {
				if strings.Contains(k, testNPCID) {
					delete(m.Rumors, k)
					n++
				}
			}
			if n > 0 {
				enc, _ := json.MarshalIndent(m, "", "  ")
				if err := os.WriteFile(rumPath, enc, 0644); err != nil {
					log.Printf("%s: %v", rumPath, err)
				} else {
					log.Printf("removed %d 試話 keys from %s", n, rumPath)
				}
			}
		}
	}

	// 5. npc_archival.json：移除 entity_id == 試話 的條目
	archPath := filepath.Join(rt, "npc_archival.json")
	if raw, err := os.ReadFile(archPath); err == nil {
		var m struct {
			Entries []struct {
				EntityID  string `json:"entity_id"`
				Content   string `json:"content"`
				Tag       string `json:"tag"`
				CreatedAt int64  `json:"created_at"`
			} `json:"entries"`
		}
		if json.Unmarshal(raw, &m) == nil {
			var out []interface{}
			for _, e := range m.Entries {
				if e.EntityID != testNPCID {
					out = append(out, map[string]interface{}{
						"entity_id": e.EntityID, "content": e.Content,
						"tag": e.Tag, "created_at": e.CreatedAt,
					})
				}
			}
			enc, _ := json.MarshalIndent(map[string]interface{}{"entries": out}, "", "  ")
			if err := os.WriteFile(archPath, enc, 0644); err != nil {
				log.Printf("%s: %v", archPath, err)
			} else {
				removed := len(m.Entries) - len(out)
				if removed > 0 {
					log.Printf("cleaned %d entity_id 試話 entries from %s", removed, archPath)
				}
			}
		}
	}

	// 6. npc_rumor_digest.json：把 digest 裡的「試話：…」去掉
	digPath := filepath.Join(rt, "npc_rumor_digest.json")
	if raw, err := os.ReadFile(digPath); err == nil {
		var m struct {
			Digest struct {
				Text        string `json:"text"`
				SourceCount int    `json:"source_count"`
				UpdatedAt   int64  `json:"updated_at"`
			} `json:"digest"`
		}
		if json.Unmarshal(raw, &m) == nil && strings.Contains(m.Digest.Text, testNPCID) {
			m.Digest.Text = strings.ReplaceAll(m.Digest.Text, testNPCID+"：離開前往start_bou…；", "")
			m.Digest.Text = strings.ReplaceAll(m.Digest.Text, testNPCID+"：排班抵達；", "")
			m.Digest.Text = strings.TrimSpace(strings.TrimPrefix(strings.TrimSpace(m.Digest.Text), "；"))
			enc, _ := json.MarshalIndent(m, "", "  ")
			if err := os.WriteFile(digPath, enc, 0644); err != nil {
				log.Printf("%s: %v", digPath, err)
			} else {
				log.Printf("cleaned 試話 from digest text in %s", digPath)
			}
		}
	}

	log.Print("done. restart server to use clean runtime.")
}
