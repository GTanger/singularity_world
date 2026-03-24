// Package roomcheck 驗證房間 JSON 契約：objects 之 Move／Look 觸發字是否出現在 description 或 responses（對齊 web/mud-text.js）。
//
// 設計立場（與「腳本先跑再說」相反）：這是靜態契約檢查，失敗即拒絕啟動／合併。
// 正式啟動鏈（./start、Makefile verify）應搭配 Options.StrictBrackets 或 CLI -brackets -strict，
// 使 description 內每個〔…〕皆須對應 objects[].name，避免半套標記上線。
package roomcheck

import (
	"encoding/json"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

// Options 掃描選項。
type Options struct {
	TriggerSockets map[string]struct{} // 例如 Move、Look
	CheckBrackets  bool
	StrictBrackets bool
}

var bracketInner = regexp.MustCompile(`\x{3014}([^\x{3015}]*)\x{3015}`)

// DefaultTriggerSockets 預設與舊 Python 腳本一致：Move + Look。
func DefaultTriggerSockets() map[string]struct{} {
	return map[string]struct{}{
		"Move": {},
		"Look": {},
	}
}

// ParseSocketList 解析逗號分隔，如 "Move,Look"。
func ParseSocketList(s string) map[string]struct{} {
	out := make(map[string]struct{})
	for _, p := range strings.Split(s, ",") {
		p = strings.TrimSpace(p)
		if p != "" {
			out[p] = struct{}{}
		}
	}
	return out
}

// CollectJSONFiles 遞迴列出 root 下 .json（檔名 _ 開頭略過）。
func CollectJSONFiles(root string) ([]string, error) {
	var out []string
	fi, err := os.Stat(root)
	if err != nil {
		return nil, err
	}
	if !fi.IsDir() {
		if strings.HasSuffix(strings.ToLower(root), ".json") && !strings.HasPrefix(filepath.Base(root), "_") {
			return []string{root}, nil
		}
		return nil, nil
	}
	err = filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			return nil
		}
		if strings.HasPrefix(d.Name(), "_") || !strings.HasSuffix(strings.ToLower(d.Name()), ".json") {
			return nil
		}
		out = append(out, path)
		return nil
	})
	if err != nil {
		return nil, err
	}
	sort.Strings(out)
	return out, nil
}

func objectSockets(obj map[string]interface{}) []string {
	raw, ok := obj["sockets"].([]interface{})
	if !ok {
		return nil
	}
	var out []string
	for _, v := range raw {
		s, ok := v.(string)
		if ok && s != "" {
			out = append(out, s)
		}
	}
	return out
}

func roomResponseCorpus(objects []interface{}) string {
	var parts []string
	for _, o := range objects {
		m, ok := o.(map[string]interface{})
		if !ok {
			continue
		}
		resp, ok := m["responses"].(map[string]interface{})
		if !ok {
			continue
		}
		for _, v := range resp {
			s, ok := v.(string)
			if ok && s != "" {
				parts = append(parts, s)
			}
		}
	}
	return strings.Join(parts, "\n")
}

// CheckFile 解析單一房間 JSON 位元組，回傳錯誤與警告訊息（不寫入 stderr）。
func CheckFile(path string, data []byte, opts Options) (errs, warns []string) {
	var top map[string]interface{}
	if err := json.Unmarshal(data, &top); err != nil {
		return []string{fmt.Sprintf("%s: JSON 解析失敗: %v", path, err)}, nil
	}

	roomID := "?"
	if v, ok := top["id"].(string); ok && v != "" {
		roomID = v
	}

	desc := ""
	if v, ok := top["description"].(string); ok {
		desc = v
	}

	rawObjs, ok := top["objects"].([]interface{})
	if !ok {
		return nil, nil
	}

	namesInRoom := make(map[string]struct{})
	for _, o := range rawObjs {
		m, ok := o.(map[string]interface{})
		if !ok {
			continue
		}
		if n, ok := m["name"].(string); ok {
			n = strings.TrimSpace(n)
			if n != "" {
				namesInRoom[n] = struct{}{}
			}
		}
	}

	corpus := desc + "\n" + roomResponseCorpus(rawObjs)

	for i, o := range rawObjs {
		obj, ok := o.(map[string]interface{})
		if !ok {
			continue
		}
		socks := objectSockets(obj)
		sockSet := make(map[string]struct{})
		for _, s := range socks {
			sockSet[s] = struct{}{}
		}

		var trig []string
		for s := range sockSet {
			if _, ok := opts.TriggerSockets[s]; ok {
				trig = append(trig, s)
			}
		}
		if len(trig) == 0 {
			continue
		}
		sort.Strings(trig)

		name := ""
		if v, ok := obj["name"].(string); ok {
			name = strings.TrimSpace(v)
		}
		oid := fmt.Sprintf("objects[%d]", i)
		if v, ok := obj["id"].(string); ok && v != "" {
			oid = v
		}
		if name == "" {
			errs = append(errs, fmt.Sprintf("%s [%s] %s: 具觸發動作 %v 但 name 為空", path, roomID, oid, trig))
			continue
		}

		_, hasMove := sockSet["Move"]
		_, hasLook := sockSet["Look"]
		_, moveTrig := opts.TriggerSockets["Move"]
		_, lookTrig := opts.TriggerSockets["Look"]

		if moveTrig && hasMove {
			if !strings.Contains(desc, name) {
				errs = append(errs, fmt.Sprintf(
					"%s [%s] %s: 檢查 Move，name「%s」未在 description 出現（無法從主描述點擊移動）",
					path, roomID, oid, name))
			}
		}

		if lookTrig && hasLook {
			if moveTrig && hasMove {
				continue
			}
			if !strings.Contains(corpus, name) {
				errs = append(errs, fmt.Sprintf(
					"%s [%s] %s: 檢查 Look，name「%s」未在 description 亦未在同房任一 responses 文字出現（無法點擊 Look）",
					path, roomID, oid, name))
			}
		}
	}

	if opts.CheckBrackets {
		for _, m := range bracketInner.FindAllStringSubmatchIndex(desc, -1) {
			// m[2], m[3] = inner group
			inner := strings.TrimSpace(desc[m[2]:m[3]])
			if inner == "" {
				continue
			}
			if _, ok := namesInRoom[inner]; !ok {
				msg := fmt.Sprintf("%s [%s]: description 有〔%s〕但無 objects[].name 完全一致", path, roomID, inner)
				if opts.StrictBrackets {
					errs = append(errs, msg)
				} else {
					warns = append(warns, msg)
				}
			}
		}
	}

	return errs, warns
}

// Run 掃描 roots（相對或絕對路徑），工作目錄用於解析相對路徑。
func Run(roots []string, workDir string, opts Options) (errs, warns []string) {
	if len(roots) == 0 {
		roots = []string{"data/rooms/editor"}
	}
	for _, rel := range roots {
		root := rel
		if !filepath.IsAbs(rel) {
			root = filepath.Join(workDir, rel)
		}
		if _, statErr := os.Stat(root); statErr != nil {
			errs = append(errs, fmt.Sprintf("路徑不存在: %s", root))
			continue
		}
		files, walkErr := CollectJSONFiles(root)
		if walkErr != nil {
			errs = append(errs, fmt.Sprintf("%s: %v", root, walkErr))
			continue
		}
		for _, fp := range files {
			data, readErr := os.ReadFile(fp)
			if readErr != nil {
				errs = append(errs, fmt.Sprintf("%s: 讀取失敗: %v", fp, readErr))
				continue
			}
			e, w := CheckFile(fp, data, opts)
			errs = append(errs, e...)
			warns = append(warns, w...)
		}
	}
	return errs, warns
}
