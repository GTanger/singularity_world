// Package db NPC-NPC 微互動：同房多 NPC 時隨機產生互動敘事。
package db

import (
	"database/sql"
	"fmt"
	"math/rand"
)

var microInteractions = []string{
	"【%s】朝【%s】點了點頭。",
	"【%s】與【%s】閒聊了幾句。",
	"【%s】看了【%s】一眼，沒有說話。",
	"【%s】對【%s】笑了笑。",
	"【%s】低聲問【%s】：「知道哪裡有活幹嗎？」",
	"【%s】向【%s】抱怨道：「唉，日子不好過啊。」",
	"【%s】和【%s】並肩站著，望向遠方。",
	"【%s】拍了拍【%s】的肩膀。",
}

// PickMicroInteraction 從同房 NPC 中挑兩人產生一句微互動敘事。chance 為觸發機率 0–100；未觸發或 NPC<2 回傳空字串。
func PickMicroInteraction(database *sql.DB, roomID string, chance int) string {
	npcs := getNPCNamesInRoom(database, roomID)
	if len(npcs) < 2 {
		return ""
	}
	if rand.Intn(100) >= chance {
		return ""
	}
	i := rand.Intn(len(npcs))
	j := i
	for j == i {
		j = rand.Intn(len(npcs))
	}
	tmpl := microInteractions[rand.Intn(len(microInteractions))]
	return fmt.Sprintf(tmpl, npcs[i], npcs[j])
}

func getNPCNamesInRoom(database *sql.DB, roomID string) []string {
	entities, _ := GetEntitiesInRoom(database, roomID)
	var names []string
	for _, e := range entities {
		if e.Kind != "npc" {
			continue
		}
		name := e.DisplayTitle
		if name == "" {
			name = e.ID
		}
		names = append(names, name)
	}
	return names
}
