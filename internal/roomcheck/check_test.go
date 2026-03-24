package roomcheck_test

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"

	"singularity_world/internal/roomcheck"
)

func moduleRoot(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller")
	}
	dir := filepath.Dir(file)
	for d := dir; d != "" && d != "."; d = filepath.Dir(d) {
		st, err := os.Stat(filepath.Join(d, "go.mod"))
		if err == nil && !st.IsDir() {
			return d
		}
	}
	t.Fatal("go.mod not found from " + dir)
	return ""
}

func TestEditorRoomsDefaultRules(t *testing.T) {
	root := moduleRoot(t)
	opts := roomcheck.Options{
		TriggerSockets: roomcheck.DefaultTriggerSockets(),
	}
	errs, warns := roomcheck.Run([]string{"data/rooms/editor"}, root, opts)
	for _, w := range warns {
		t.Log("warn:", w)
	}
	if len(errs) > 0 {
		for _, e := range errs {
			t.Error(e)
		}
	}
}
