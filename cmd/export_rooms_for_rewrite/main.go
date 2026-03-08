// 掃描 data/rooms 下所有 .json，輸出【名稱｜id｜描述｜出口】到單一文檔，供 Gemini 重寫描述用。
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

type roomFile struct {
	ID          string     `json:"id"`
	Name        string     `json:"name"`
	Description string     `json:"description"`
	Exits       []exitOut  `json:"exits"`
}
type exitOut struct {
	Direction string `json:"direction"`
	To       string `json:"to"`
}

func main() {
	roomsDir := "data/rooms"
	if len(os.Args) > 1 {
		roomsDir = os.Args[1]
	}
	outPath := "docs/rooms_export_for_rewrite.md"
	if len(os.Args) > 2 {
		outPath = os.Args[2]
	}

	var rooms []roomFile
	err := filepath.Walk(roomsDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.IsDir() || !strings.HasSuffix(path, ".json") {
			return nil
		}
		if strings.HasSuffix(path, "_template.json") {
			return nil
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		var r roomFile
		if err := json.Unmarshal(data, &r); err != nil {
			return nil // 略過格式不符的檔
		}
		if r.ID == "" || r.ID == "example_room_id" {
			return nil
		}
		rooms = append(rooms, r)
		return nil
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "walk: %v\n", err)
		os.Exit(1)
	}

	sort.Slice(rooms, func(i, j int) bool { return rooms[i].ID < rooms[j].ID })

	var b strings.Builder
	b.WriteString("# 房間清單：名稱｜id｜描述｜出口（供 Gemini 重寫描述用）\n\n")
	b.WriteString("以下為所有格子的【名稱｜id｜描述｜出口】。請依同一格式重寫「描述」欄位，其餘勿改。\n\n")
	b.WriteString("---\n\n")

	for _, r := range rooms {
		b.WriteString("## ")
		b.WriteString(r.Name)
		b.WriteString("\n\n")
		b.WriteString("| 欄位 | 內容 |\n")
		b.WriteString("|------|------|\n")
		b.WriteString("| **名稱** | ")
		b.WriteString(escapeTable(r.Name))
		b.WriteString(" |\n")
		b.WriteString("| **id** | ")
		b.WriteString(escapeTable(r.ID))
		b.WriteString(" |\n")
		b.WriteString("| **描述** | ")
		b.WriteString(escapeTable(r.Description))
		b.WriteString(" |\n")
		exitsStr := formatExits(r.Exits)
		b.WriteString("| **出口** | ")
		b.WriteString(escapeTable(exitsStr))
		b.WriteString(" |\n\n")
	}

	if err := os.WriteFile(outPath, []byte(b.String()), 0644); err != nil {
		fmt.Fprintf(os.Stderr, "write: %v\n", err)
		os.Exit(1)
	}
	fmt.Printf("wrote %d rooms to %s\n", len(rooms), outPath)
}

func formatExits(exits []exitOut) string {
	if len(exits) == 0 {
		return "（無）"
	}
	parts := make([]string, len(exits))
	for i, e := range exits {
		parts[i] = e.Direction + " → " + e.To
	}
	return strings.Join(parts, "；")
}

func escapeTable(s string) string {
	s = strings.ReplaceAll(s, "|", "\\|")
	s = strings.ReplaceAll(s, "\n", " ")
	return s
}
