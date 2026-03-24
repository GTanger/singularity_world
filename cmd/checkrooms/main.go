// checkrooms：房間 JSON 反向檢查（Move／Look 觸發字是否在 description／responses 可見）。
// 用法見 -h；專案根執行：go run ./cmd/checkrooms
package main

import (
	"flag"
	"fmt"
	"os"
	"sort"

	"singularity_world/internal/roomcheck"
)

func main() {
	socketsFlag := flag.String("sockets", "Move,Look", "逗號分隔：要檢查的 sockets 名稱（與 objects[].sockets 交集觸發規則）")
	brackets := flag.Bool("brackets", false, "額外檢查 description 內每個〔…〕是否對應某 objects[].name")
	strict := flag.Bool("strict", false, "與 -brackets 併用：無對應 name 時視為錯誤（預設僅警告）")
	flag.Parse()
	roots := flag.Args()

	wd, err := os.Getwd()
	if err != nil {
		fmt.Fprintf(os.Stderr, "getwd: %v\n", err)
		os.Exit(1)
	}

	trigger := roomcheck.ParseSocketList(*socketsFlag)
	if len(trigger) == 0 {
		fmt.Fprintln(os.Stderr, "-sockets 不可為空")
		os.Exit(1)
	}

	opts := roomcheck.Options{
		TriggerSockets: trigger,
		CheckBrackets:  *brackets,
		StrictBrackets: *strict,
	}

	errs, warns := roomcheck.Run(roots, wd, opts)

	if len(warns) > 0 {
		fmt.Fprintln(os.Stderr, "--- 警告 ---")
		for _, w := range warns {
			fmt.Fprintln(os.Stderr, w)
		}
	}
	if len(errs) > 0 {
		fmt.Fprintln(os.Stderr, "--- 錯誤 ---")
		for _, e := range errs {
			fmt.Fprintln(os.Stderr, e)
		}
		fmt.Fprintf(os.Stderr, "--- 房間檢查失敗：%d 筆錯誤 ---\n", len(errs))
		os.Exit(1)
	}

	var keys []string
	for k := range trigger {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	fmt.Printf("OK：房間檢查通過 trigger_sockets=%v", keys)
	if len(warns) > 0 {
		fmt.Printf("（警告 %d 筆已列於 stderr）", len(warns))
	}
	fmt.Println()
	os.Exit(0)
}
