// 一次性清洗：清空 npc_npc_summaries、從 npc_archival 移除 tag=npc_npc。
// 執行（專案根目錄）：go run ./scripts/clean-npc-npc-pollution
package main

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
)

type summariesFile struct {
	Summaries map[string]string `json:"summaries"`
}

type archivalFile struct {
	Entries []struct {
		EntityID  string `json:"entity_id"`
		Content   string `json:"content"`
		Tag       string `json:"tag"`
		CreatedAt int64  `json:"created_at"`
	} `json:"entries"`
}

func main() {
	rt := filepath.Join("data", "runtime")
	sumPath := filepath.Join(rt, "npc_npc_summaries.json")
	archPath := filepath.Join(rt, "npc_archival.json")

	emptySum, _ := json.MarshalIndent(summariesFile{Summaries: map[string]string{}}, "", "  ")
	if err := os.WriteFile(sumPath, emptySum, 0644); err != nil {
		log.Fatal(sumPath, err)
	}
	log.Printf("wrote empty summaries: %s", sumPath)

	raw, err := os.ReadFile(archPath)
	if err != nil {
		log.Fatal(archPath, err)
	}
	var af archivalFile
	if err := json.Unmarshal(raw, &af); err != nil {
		log.Fatal(err)
	}
	var out []struct {
		EntityID  string `json:"entity_id"`
		Content   string `json:"content"`
		Tag       string `json:"tag"`
		CreatedAt int64  `json:"created_at"`
	}
	removed := 0
	for _, e := range af.Entries {
		if e.Tag == "npc_npc" {
			removed++
			continue
		}
		out = append(out, e)
	}
	type outFile struct {
		Entries []struct {
			EntityID  string `json:"entity_id"`
			Content   string `json:"content"`
			Tag       string `json:"tag"`
			CreatedAt int64  `json:"created_at"`
		} `json:"entries"`
	}
	enc, err := json.MarshalIndent(outFile{Entries: out}, "", "  ")
	if err != nil {
		log.Fatal(err)
	}
	if err := os.WriteFile(archPath, enc, 0644); err != nil {
		log.Fatal(archPath, err)
	}
	log.Printf("npc_archival: removed %d npc_npc entries, kept %d", removed, len(out))
}
