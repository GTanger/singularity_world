// 一房一檔：讀取 data/rooms.json，拆成 data/rooms/<id>.json。只跑一次即可。
package main

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
)

type roomsFile struct {
	Rooms []roomDef `json:"rooms"`
	Exits []exitDef `json:"exits"`
}
type roomDef struct {
	ID          string   `json:"id"`
	Name        string   `json:"name"`
	Tags        []string `json:"tags"`
	Zone        string   `json:"zone"`
	Description string   `json:"description"`
}
type exitDef struct {
	From      string `json:"from"`
	Direction string `json:"direction"`
	To        string `json:"to"`
}

type roomOut struct {
	ID          string       `json:"id"`
	Name        string       `json:"name"`
	Description string       `json:"description"`
	Tags        []string     `json:"tags"`
	Zone        string       `json:"zone"`
	Exits       []exitOut    `json:"exits"`
}
type exitOut struct {
	Direction string `json:"direction"`
	To        string `json:"to"`
}

func main() {
	data, err := os.ReadFile("data/rooms.json")
	if err != nil {
		log.Fatal(err)
	}
	var f roomsFile
	if err := json.Unmarshal(data, &f); err != nil {
		log.Fatal(err)
	}
	exitsByFrom := make(map[string][]exitOut)
	for _, e := range f.Exits {
		exitsByFrom[e.From] = append(exitsByFrom[e.From], exitOut{Direction: e.Direction, To: e.To})
	}
	if err := os.MkdirAll("data/rooms", 0755); err != nil {
		log.Fatal(err)
	}
	for _, r := range f.Rooms {
		exits := exitsByFrom[r.ID]
		if exits == nil {
			exits = []exitOut{}
		}
		one := roomOut{
			ID: r.ID, Name: r.Name, Description: r.Description,
			Tags: r.Tags, Zone: r.Zone, Exits: exits,
		}
		raw, _ := json.MarshalIndent(one, "", "  ")
		path := filepath.Join("data", "rooms", r.ID+".json")
		if err := os.WriteFile(path, raw, 0644); err != nil {
			log.Fatal(err)
		}
	}
	log.Printf("split %d rooms into data/rooms/*.json", len(f.Rooms))
}
