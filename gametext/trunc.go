package gametext

// TruncRune truncates s to at most n runes, appending an ellipsis (…) if shortened.
func TruncRune(s string, n int) string {
	r := []rune(s)
	if len(r) <= n {
		return s
	}
	return string(r[:n]) + string(rune(0x2026))
}
