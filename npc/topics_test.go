package npc

import (
	"path/filepath"
	"runtime"
	"testing"
)

func testTopicsPath(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller")
	}
	dir := filepath.Dir(file)
	return filepath.Join(dir, "..", "data", "npc_to_npc_topics.json")
}

func TestFindNpcNpcTopicIDByHint(t *testing.T) {
	LoadNpcNpcTopics(testTopicsPath(t))
	if id := FindNpcNpcTopicIDByHint("換班、交接時的情境，簡短交代或叮囑"); id != "交班" {
		t.Fatalf("exact hint: got %q", id)
	}
	if id := FindNpcNpcTopicIDByHint("隨口閒聊，天氣、見聞、抱怨（路人在旁，兩句務必極短。）"); id != "閒聊" {
		t.Fatalf("hint with 路人在旁 suffix: got %q", id)
	}
	if id := FindNpcNpcTopicIDByHint("鎮上傳聞城外惡地、霜林、游離輻射或富態化巨獸（非荒蕪末世，而是生機過剩的危險）"); id != "城外聞" {
		t.Fatalf("城外聞: got %q", id)
	}
	if id := FindNpcNpcTopicIDByHint(""); id != "" {
		t.Fatalf("empty: got %q", id)
	}
}
