// 掃描 data/rooms 下所有房間 JSON，依「導航與出口融合」設計（docs/房間非人物件互動.md §2.3.2）：
// - 巷道／路段：僅保證 Move（可僅含 Move，點擊直接移動）
// - 建築／商鋪／住宅：補齊 Look＋Move（先 Look 再選進入）
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
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
	To       string `json:"to"`
	UIHidden bool   `json:"ui_hidden,omitempty"`
}

type roomObject struct {
	ID           string            `json:"id"`
	Name         string            `json:"name"`
	Owner        string            `json:"owner,omitempty"`
	Sockets      []string          `json:"sockets"`
	Responses    map[string]string `json:"responses"`
	MoveToRoomID string            `json:"move_to_room_id,omitempty"`
}

func hasSocket(sockets []string, s string) bool {
	for _, x := range sockets {
		if x == s {
			return true
		}
	}
	return false
}

// isRoadlike 依名稱判斷是否為「巷道／路段」類（點擊直接 Move，不強制 Look）。
// 參考：大街三段、前方緩坡、狹窄巷口、四路街口。
func isRoadlike(name string) bool {
	n := strings.TrimSpace(name)
	if n == "" {
		return false
	}
	// 明確巷道關鍵字
	if strings.Contains(n, "大街") && strings.Contains(n, "段") {
		return true
	}
	if strings.Contains(n, "巷口") || strings.Contains(n, "街口") || strings.Contains(n, "緩坡") ||
		strings.Contains(n, "步道") || strings.Contains(n, "隧道") || strings.Contains(n, "小徑") || strings.Contains(n, "橋") {
		return true
	}
	// 以段／路／巷結尾的路名感
	if strings.HasSuffix(n, "段") || strings.HasSuffix(n, "路") || strings.HasSuffix(n, "巷") {
		return true
	}
	return false
}

// ensureNavigation 依設計補齊：巷道類只補 Move；建築類補 Look＋Move。
func ensureNavigation(obj *roomObject) (fixed bool) {
	if obj.MoveToRoomID == "" {
		return false
	}
	if obj.Responses == nil {
		obj.Responses = make(map[string]string)
	}
	roadlike := isRoadlike(obj.Name)

	// 所有導航物件至少要有 Move
	if !hasSocket(obj.Sockets, "Move") {
		obj.Sockets = append(obj.Sockets, "Move")
		fixed = true
	}
	if obj.Responses["Move"] == "" {
		obj.Responses["Move"] = "你往" + obj.Name + "走去。"
		fixed = true
	}

	// 建築類：以 Look 為首並含 Move，供「先看再選進入」
	if !roadlike {
		if !hasSocket(obj.Sockets, "Look") {
			obj.Sockets = append([]string{"Look"}, obj.Sockets...)
			fixed = true
		}
		if obj.Responses["Look"] == "" {
			obj.Responses["Look"] = "此處可通往" + obj.Name + "。"
			fixed = true
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
		fileFixed := 0
		for i := range r.Objects {
			if ensureNavigation(&r.Objects[i]) {
				fileFixed++
			}
		}
		if fileFixed > 0 {
			totalObjectsFixed += fileFixed
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
	fmt.Printf("修正的物件數: %d\n", totalObjectsFixed)
	if *dryRun && len(fixedFiles) > 0 {
		fmt.Println("以下檔案將被寫入（使用時請去掉 -dry-run）：")
		for _, f := range fixedFiles {
			fmt.Println("  ", f)
		}
	}
}
