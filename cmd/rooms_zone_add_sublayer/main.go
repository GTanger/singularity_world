// 第二層子資料夾：依 id 前綴分組，子資料夾名稱從同 zone 的 hub（name==zone）的 exit direction 查詢。
// 例：sun_lc_01、sun_lc_02 前綴皆 sun_lc；hub 有 exit to sun_lc_01 direction「流光採集廣場」→ 皆進 向陽大街/流光採集廣場/。Hub 本身放 zone 根目錄。
package main

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
)

type room struct {
	ID    string `json:"id"`
	Name  string `json:"name"`
	Zone  string `json:"zone"`
	Exits []struct {
		Direction string `json:"direction"`
		To        string `json:"to"`
	} `json:"exits"`
}

// 只剝「房間尾綴」：_r數字；以及結尾 _0數字（_01～_09），不剝單數字如 _1 以免傷到 ironlane_smith1。
// 例：ironlane_smith1_r1→ironlane_smith1，sun_lc_01→sun_lc，feishuang_home12_r5→feishuang_home12。
var roomSuffix = regexp.MustCompile(`_r\d+$`)
var trailingZeroNum = regexp.MustCompile(`_0\d$`)

func idPrefix(id string) string {
	for roomSuffix.MatchString(id) {
		id = roomSuffix.ReplaceAllString(id, "")
	}
	for trailingZeroNum.MatchString(id) {
		id = trailingZeroNum.ReplaceAllString(id, "")
	}
	return id
}

func sanitize(s string) string {
	for _, c := range []string{"/", "\\", ":", "*", "?", "\"", "<", ">", "|"} {
		s = strings.ReplaceAll(s, c, "_")
	}
	s = strings.TrimSpace(s)
	if s == "" {
		return ""
	}
	return s
}

// 夜鴞巷：yaxiao_N 編號區間 → 子資料夾名稱（一雙層 10 組 + 二三層 5 組）
var yaxiaoRanges = []struct {
	start, end int
	name       string
}{
	{1, 25, "日常雜貨"},
	{26, 43, "自助洗衣"},
	{44, 73, "舊物回收"},
	{74, 95, "電器修理"},
	{96, 115, "快餐小店"},
	{116, 127, "巷口六號"},
	{128, 137, "空置房"}, // 含 136、137 避免落單
	{138, 152, "張氏寓所"},
	{153, 165, "管道維護站"},
	{166, 173, "雜物間"},
	{174, 184, "舊瓦舍"},
	{185, 198, "街角一樓"},
	{199, 207, "後巷平房"},
	{208, 219, "石牆側室"},
	{220, 229, "窄門寓所"},
}

func main() {
	base := "data/rooms"

	var all []struct {
		path string
		r    room
	}
	err := filepath.WalkDir(base, func(path string, d os.DirEntry, err error) error {
		if err != nil || d.IsDir() || filepath.Ext(d.Name()) != ".json" {
			return err
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		var r room
		if json.Unmarshal(data, &r) != nil {
			return nil
		}
		all = append(all, struct{ path string; r room }{path, r})
		return nil
	})
	if err != nil {
		log.Fatal(err)
	}

	// 每個 zone：hub 的 to_id -> direction；再用 to_id 的前綴 -> 子資料夾名稱
	// zone -> (prefix -> subfolder name)
	zonePrefixToName := make(map[string]map[string]string)
	for _, p := range all {
		if p.r.Name != p.r.Zone || p.r.Zone == "" {
			continue
		}
		z := p.r.Zone
		if zonePrefixToName[z] == nil {
			zonePrefixToName[z] = make(map[string]string)
		}
		for _, e := range p.r.Exits {
			if e.To == "" || e.Direction == "" {
				continue
			}
			pref := idPrefix(e.To)
			zonePrefixToName[z][pref] = e.Direction
		}
	}

	zoneBase := filepath.Clean(base)
	for _, p := range all {
		id, z := p.r.ID, p.r.Zone
		if z == "" {
			continue
		}
		// hub 放 zone 根目錄
		if p.r.Name == z {
			targetDir := filepath.Join(zoneBase, z)
			targetPath := filepath.Join(targetDir, id+".json")
			currentPath := filepath.Clean(p.path)
			if currentPath == targetPath {
				continue
			}
			_ = os.MkdirAll(targetDir, 0755)
			if err := os.Rename(currentPath, targetPath); err != nil {
				log.Printf("rename %s -> %s: %v", currentPath, targetPath, err)
			} else {
				fmt.Printf("%s -> %s\n", currentPath, targetPath)
			}
			continue
		}
		pref := idPrefix(id)
		sub := zonePrefixToName[z][pref]
		// 浮生大街：life_ 開頭且非 lifestreet_ 的為客棧內房間，歸到「客棧」
		if sub == "" && z == "浮生大街" && strings.HasPrefix(id, "life_") && !strings.HasPrefix(id, "lifestreet_") {
			sub = zonePrefixToName[z]["life_garden"]
		}
		// 飛霜大街：feishuang_lodge1_* 歸霜華館、feishuang_lodge2* 歸凝霜閣
		if sub == "" && z == "飛霜大街" {
			if strings.HasPrefix(id, "feishuang_lodge1") {
				sub = zonePrefixToName[z]["feishuang_lodge1"]
			} else if strings.HasPrefix(id, "feishuang_lodge2") {
				sub = zonePrefixToName[z]["feishuang_lodge2"]
			}
		}
		// 向陽大街：hub 只列 sun_xx_01，sun_lc_10、sun_gs_12 等剝尾段對應（sun_lc_10→sun_lc→流光採集廣場）
		if sub == "" && z == "向陽大街" && strings.HasPrefix(id, "sun_") {
			parts := strings.Split(pref, "_")
			for len(parts) >= 2 {
				try := strings.Join(parts, "_")
				if d := zonePrefixToName[z][try]; d != "" {
					sub = d
					break
				}
				parts = parts[:len(parts)-1]
			}
		}
		// 夜鴞巷：依 yaxiao_N 編號區間對應到指定子資料夾名稱（巷口六號、雜物間、張氏寓所…）
		if z == "夜鴞巷" && strings.HasPrefix(id, "yaxiao_") {
			if n, err := strconv.Atoi(strings.TrimPrefix(id, "yaxiao_")); err == nil {
				for _, r := range yaxiaoRanges {
					if n >= r.start && n <= r.end {
						sub = r.name
						break
					}
				}
			}
		}
		// 梧桐大街：wutong_cq_* 全進一資料夾、wutong_cy_* 全進一資料夾；其餘 wutong_* 用 hub 的 to 前綴對應
		if sub == "" && z == "梧桐大街" && strings.HasPrefix(id, "wutong_") {
			if strings.HasPrefix(id, "wutong_cq") {
				sub = "wutong_cq"
			} else if strings.HasPrefix(id, "wutong_cy") {
				sub = "wutong_cy"
			} else {
				longest := ""
				for key := range zonePrefixToName[z] {
					if strings.HasPrefix(key, pref) && len(key) > len(longest) {
						longest = key
					}
				}
				if longest != "" {
					sub = zonePrefixToName[z][longest]
				}
			}
		}
		sub = sanitize(sub)
		if sub == "" {
			sub = pref
		}
		targetDir := filepath.Join(zoneBase, z, sub)
		targetPath := filepath.Join(targetDir, id+".json")
		currentPath := filepath.Clean(p.path)
		if currentPath == targetPath {
			continue
		}
		if err := os.MkdirAll(targetDir, 0755); err != nil {
			log.Fatal(err)
		}
		if err := os.Rename(currentPath, targetPath); err != nil {
			log.Printf("rename %s -> %s: %v", currentPath, targetPath, err)
		} else {
			fmt.Printf("%s -> %s\n", currentPath, targetPath)
		}
	}
	log.Print("done: zone/(direction from hub by id prefix)/id.json")
}
