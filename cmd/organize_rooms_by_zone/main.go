// 依 zone 將 data/rooms/*.json 搬入 data/rooms/<zone>/<id>.json。只處理直接位於 data/rooms/ 的 .json，不搬子目錄內檔案。
package main

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
	"strings"
)

type room struct {
	ID   string `json:"id"`
	Zone string `json:"zone"`
}

func sanitizeDir(s string) string {
	for _, c := range []string{"/", "\\", ":", "*", "?", "\"", "<", ">", "|"} {
		s = strings.ReplaceAll(s, c, "_")
	}
	s = strings.TrimSpace(s)
	if s == "" {
		s = "_"
	}
	return s
}

func main() {
	dir := "data/rooms"
	entries, err := os.ReadDir(dir)
	if err != nil {
		log.Fatal(err)
	}
	for _, e := range entries {
		if e.IsDir() || filepath.Ext(e.Name()) != ".json" {
			continue
		}
		path := filepath.Join(dir, e.Name())
		data, err := os.ReadFile(path)
		if err != nil {
			log.Printf("read %s: %v", path, err)
			continue
		}
		var r room
		if err := json.Unmarshal(data, &r); err != nil {
			log.Printf("parse %s: %v", path, err)
			continue
		}
		zoneDir := sanitizeDir(r.Zone)
		targetDir := filepath.Join(dir, zoneDir)
		if err := os.MkdirAll(targetDir, 0755); err != nil {
			log.Fatal(err)
		}
		targetPath := filepath.Join(targetDir, r.ID+".json")
		if targetPath == path {
			continue
		}
		if err := os.Rename(path, targetPath); err != nil {
			log.Printf("rename %s -> %s: %v", path, targetPath, err)
		}
	}
	log.Print("done: rooms moved under data/rooms/<zone>/<id>.json")
}
