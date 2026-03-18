package db

import (
	"testing"

	"singularity_world/store"
)

func TestSplitQueryTerms(t *testing.T) {
	tests := []struct {
		q    string
		want int
	}{
		{"", 0},
		{"   ", 0},
		{"你好", 1},
		{"不付錢 就不用錢", 2},
		{"  a  b  c  ", 3},
	}
	for _, tt := range tests {
		got := splitQueryTerms(tt.q)
		if len(got) != tt.want {
			t.Errorf("splitQueryTerms(%q) len = %d, want %d", tt.q, len(got), tt.want)
		}
	}
	// SearchArchival 無 store 時回傳 nil（多數測試環境未 Init store）
	if store.Default == nil {
		out := SearchArchival("npc1", "你好", 5)
		if out != nil {
			t.Errorf("SearchArchival with nil store want nil, got %v", out)
		}
	}
}
