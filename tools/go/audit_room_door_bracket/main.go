// 全地圖修訂：比照房內門戶／Move 鑲嵌邏輯（現行浮生城 editor 房亦同）
// 1) 門／簾／入口類（有 move_to_room_id 且名稱像入口）→ 僅 Move，移除 Look
// 2) 建築名 Look 敘述中，若出現同房「門／簾」物件名且未包〔〕→ 改為 〔名稱〕
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

type roomFile struct {
	ID          string       `json:"id"`
	Name        string       `json:"name"`
	Description string       `json:"description"`
	Tags        []string     `json:"tags,omitempty"`
	Zone        string       `json:"zone,omitempty"`
	Exits       []exitDef    `json:"exits,omitempty"`
	Objects     []roomObject `json:"objects,omitempty"`
}

type exitDef struct {
	Direction string `json:"direction"`
	To        string `json:"to"`
	UIHidden  bool   `json:"ui_hidden,omitempty"`
}

type roomObject struct {
	ID           string            `json:"id"`
	Name         string            `json:"name"`
	Owner        string            `json:"owner,omitempty"`
	Sockets      []string          `json:"sockets"`
	Responses    map[string]string  `json:"responses"`
	MoveToRoomID string            `json:"move_to_room_id,omitempty"`
}

// isDoorLike 名稱像入口（門、簾、牌等），點擊應直接 Move
func isDoorLike(name string) bool {
	n := strings.TrimSpace(name)
	if n == "" {
		return false
	}
	return strings.Contains(n, "門") || strings.Contains(n, "簾") || strings.Contains(n, "牌") ||
		strings.Contains(n, "扉") || strings.Contains(n, "簷") || strings.Contains(n, "入口")
}

func hasSocket(sockets []string, s string) bool {
	for _, x := range sockets {
		if x == s {
			return true
		}
	}
	return false
}

func processRoom(r *roomFile) (fixed int) {
	// 收集本房所有「入口」物件名（有 move_to_room_id 的），依名稱長度降序以便替換時長名優先
	doorNames := make([]string, 0)
	for i := range r.Objects {
		o := &r.Objects[i]
		if o.MoveToRoomID != "" && isDoorLike(o.Name) {
			doorNames = append(doorNames, o.Name)
		}
	}
	sort.Slice(doorNames, func(i, j int) bool { return len(doorNames[i]) > len(doorNames[j]) })

	// 1) 門／簾類改為僅 Move
	for i := range r.Objects {
		o := &r.Objects[i]
		if o.MoveToRoomID == "" || !isDoorLike(o.Name) {
			continue
		}
		if hasSocket(o.Sockets, "Look") || len(o.Sockets) > 1 {
			o.Sockets = []string{"Move"}
			if o.Responses == nil {
				o.Responses = make(map[string]string)
			}
			moveResp := o.Responses["Move"]
			if moveResp == "" {
				moveResp = "你往" + o.Name + "走去。"
			}
			o.Responses = map[string]string{"Move": moveResp}
			fixed++
		}
	}

	// 2) 建築名 Look 敘述中，同房門名未包〔〕則包起來（依名稱長度由長到短替換，避免短名吃長名）
	for i := range r.Objects {
		o := &r.Objects[i]
		if o.MoveToRoomID != "" {
			continue
		}
		lookText := o.Responses["Look"]
		if lookText == "" {
			continue
		}
		// 依名稱長度降序，先替換長的
		for _, dname := range doorNames {
			if dname == "" {
				continue
			}
			wrapped := "〔" + dname + "〕"
			if strings.Contains(lookText, wrapped) {
				continue
			}
			if strings.Contains(lookText, dname) {
				o.Responses["Look"] = strings.Replace(lookText, dname, wrapped, 1)
				lookText = o.Responses["Look"]
				fixed++
			}
		}
	}
	return fixed
}

func main() {
	roomsDir := flag.String("dir", "data/rooms", "房間 JSON 目錄")
	dryRun := flag.Bool("dry-run", false, "只報告不寫回")
	flag.Parse()

	var totalRooms, totalFixed, totalObjectsFixed int
	var fixedFiles []string

	err := filepath.WalkDir(*roomsDir, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || strings.ToLower(filepath.Ext(d.Name())) != ".json" {
			return nil
		}
		if strings.HasPrefix(strings.TrimSuffix(d.Name(), filepath.Ext(d.Name())), "_") {
			return nil
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		var r roomFile
		if err := json.Unmarshal(data, &r); err != nil {
			return nil
		}
		totalRooms++
		n := processRoom(&r)
		if n > 0 {
			totalObjectsFixed += n
			totalFixed++
			rel, _ := filepath.Rel(*roomsDir, path)
			fixedFiles = append(fixedFiles, rel)
			if !*dryRun {
				out, err := json.MarshalIndent(r, "", "  ")
				if err != nil {
					return fmt.Errorf("%s: marshal: %w", path, err)
				}
				if err := os.WriteFile(path, out, 0644); err != nil {
					return fmt.Errorf("%s: write: %w", path, err)
				}
			}
		}
		return nil
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("掃描房間數: %d\n", totalRooms)
	fmt.Printf("有修正的檔案數: %d\n", totalFixed)
	fmt.Printf("修正的物件/處數: %d\n", totalObjectsFixed)
	if *dryRun && len(fixedFiles) > 0 {
		fmt.Println("以下檔案將被寫入（使用時請去掉 -dry-run）：")
		for _, f := range fixedFiles {
			fmt.Println("  ", f)
		}
	}
}
