// Package db 的 NPC 活化系統模擬測試：從生成、soul_seed 展開、性格、排班到移動 Tick 的端到端驗證。
// 對應文件：docs/testing/NPC活化系統模擬測試報告.md

package db

import (
	"testing"
)

// TestSim_NPCGenerationWithSoulSeed 模擬：NPC 生成後必帶 soul_seed，且體敏氣由該 seed 展開。
func TestSim_NPCGenerationWithSoulSeed(t *testing.T) {
	setupTestStore(t)
	err := InsertNPC("模擬甲", "模", "M", "")
	if err != nil {
		t.Fatalf("InsertNPC: %v", err)
	}
	ent, err := GetEntity("模擬甲")
	if err != nil || ent == nil {
		t.Fatalf("GetEntity: %v", err)
	}
	if ent.SoulSeed == nil {
		t.Fatal("NPC 生成後應帶 soul_seed，實作規定創角即寫入")
	}
	seed := *ent.SoulSeed
	vit, qi, dex := ExpandSoulSeedToBaseStats(seed)
	if ent.Vit != vit || ent.Qi != qi || ent.Dex != dex {
		t.Errorf("體敏氣應與 ExpandSoulSeedToBaseStats(seed) 一致: got vit=%d qi=%d dex=%d, want vit=%d qi=%d dex=%d",
			ent.Vit, ent.Qi, ent.Dex, vit, qi, dex)
	}
}

// TestSim_SoulSeedDeterminism 模擬：同一 seed 多次展開，BaseStats / OriginSentence / Personality 皆確定性一致。
func TestSim_SoulSeedDeterminism(t *testing.T) {
	const seed int64 = 12345
	vit1, qi1, dex1 := ExpandSoulSeedToBaseStats(seed)
	vit2, qi2, dex2 := ExpandSoulSeedToBaseStats(seed)
	if vit1 != vit2 || qi1 != qi2 || dex1 != dex2 {
		t.Errorf("BaseStats 應確定性: (%d,%d,%d) vs (%d,%d,%d)", vit1, qi1, dex1, vit2, qi2, dex2)
	}
	origin1 := ExpandSoulSeedToOriginSentence(seed)
	origin2 := ExpandSoulSeedToOriginSentence(seed)
	if origin1 != origin2 {
		t.Errorf("OriginSentence 應確定性: %q vs %q", origin1, origin2)
	}
	p1 := ExpandSoulSeedToPersonality(seed)
	p2 := ExpandSoulSeedToPersonality(seed)
	if p1.Boldness != p2.Boldness || p1.Sensitivity != p2.Sensitivity || p1.Orderliness != p2.Orderliness {
		t.Errorf("Personality 應確定性: %+v vs %+v", p1, p2)
	}
	if p1.Boldness < 0 || p1.Boldness > 1 || p1.Sensitivity < 0 || p1.Sensitivity > 1 || p1.Orderliness < 0 || p1.Orderliness > 1 {
		t.Errorf("Personality 應在 [0,1]: %+v", p1)
	}
}

// TestSim_GetPersonalityForEntity 模擬：有 soul_seed 的實體可取得 Personality；無則回傳零值與 false。
func TestSim_GetPersonalityForEntity(t *testing.T) {
	setupTestStore(t)
	_ = InsertNPC("有種子", "有", "M", "")
	p, ok := GetPersonalityForEntity("有種子")
	if !ok {
		t.Fatal("有 soul_seed 的 NPC 應回傳 ok=true")
	}
	if p.Boldness < 0 || p.Boldness > 1 {
		t.Errorf("Boldness 應在 [0,1]: %f", p.Boldness)
	}
	_, ok = GetPersonalityForEntity("不存在ID")
	if ok {
		t.Error("不存在的實體應回傳 ok=false")
	}
}
