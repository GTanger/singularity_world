// 交易輸入欄：判斷是否為取消／拒絕用語。
package server

import (
	"strings"
)

func isTradeRejectInput(s string) bool {
	s = strings.TrimSpace(strings.ToLower(s))
	switch s {
	case "拒絕", "取消", "算了", "不要", "no", "n":
		return true
	default:
		return false
	}
}
