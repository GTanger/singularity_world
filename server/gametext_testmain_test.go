package server

import (
	"os"
	"testing"

	"singularity_world/gametext"
)

func TestMain(m *testing.M) {
	gametext.SetPathForTest("data/config/gametext.json")
	if err := gametext.Load(""); err != nil {
		gametext.SetPathForTest("../data/config/gametext.json")
		_ = gametext.Load("")
	}
	os.Exit(m.Run())
}
