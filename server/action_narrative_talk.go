// do_action 敘事：Talk fallback（無 LLM 時）。
package server

import (
	"math/rand"
	"strings"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/game"
)

func buildTalkNarrative(playerRoom string, target *entity.Character, personality *db.Personality, cfg config.Server, playerInput string) string {
	name := target.DisplayTitle
	if name == "" {
		name = target.ID
	}
	seed := int64(0)
	for _, r := range target.ID {
		seed = seed*31 + int64(r)
	}
	seed += int64(len(playerInput))<<10 + int64(len(playerRoom))<<16
	// §10.16：玩家有輸入時先做關鍵字檢索
	if strings.TrimSpace(playerInput) != "" && playerInput != "（搭話）" {
		if line := db.TryMatchKeyword(playerInput, seed); line != "" {
			return "你向【" + name + "】搭話。" + strings.ReplaceAll(line, "{name}", name)
		}
	}
	// 優先從職業對話檔抽句並填佔位符（{name}、{room}、{time}、{mood}、{verb}、{thing}、{goods}）
	assignments, _ := db.GetAssignmentsForEntity(target.ID)
	if len(assignments) > 0 {
		roomName, _ := db.GetRoomName(playerRoom)
		timeLabel := ""
		if now := game.NowUnix(); cfg.GameTimeEpochUnix != 0 {
			_, hour, _, _ := game.GameTimeNow(now, cfg.GameTimeEpochUnix, cfg.GameTimeScale)
			if hour >= 5 && hour < 10 {
				timeLabel = "清晨"
			} else if hour >= 10 && hour < 14 {
				timeLabel = "正午"
			} else if hour >= 14 && hour < 18 {
				timeLabel = "傍晚"
			} else {
				timeLabel = "夜裡"
			}
		}
		line := db.PickFromDialogue(target.ID, playerRoom, assignments[0].OccupationID, "talk", personality, "", roomName, timeLabel, "")
		if line != "" {
			return "你向【" + name + "】搭話。" + line
		}
	}
	// §10.16：公共 fallback — 公版對話 public_dialogue.json talk.lines
	if line := db.PickFromPublicTalk(name, seed+1); line != "" {
		return "你向【" + name + "】搭話。" + line
	}
	// 終極 fallback：內建句池
	responses := []string{
		"「你好，有什麼事嗎？」",
		"「這裡最近不太平靜，你小心點。」",
		"「我只是個路人，別找我麻煩。」",
		"「你看起來像是個新手。」",
		"「嗯？」",
		"「別擋路。」",
		"「你也是來這裡討生活的？」",
		"「聽說城外最近出了些怪事。」",
		"「有事快說，我還有活要幹。」",
		"「這條街什麼人都有，自己多留神。」",
		"「想打聽事？找別人吧。」",
		"「靈脈這幾日不穩，少往地縫邊湊。」",
		"「買東西往那頭，我這兒不賣。」",
		"「路過就路過，別瞎瞧。」",
		"「……你誰啊？」",
		"「有緣再聊，先走了。」",
		"「城裡規矩多，別亂闖。」",
		"「丹藥鋪在東邊，兵器在西邊。」",
		"「沒見過你，新來的？」",
		"「天快黑了，早點找地方落腳。」",
		"「最近生意不好做啊。」",
		"「修行之人，少管閒事。」",
		"「要問路的話，前頭有告示。」",
		"「沒什麼好說的。」",
		"「嗯，怎麼了？」",
		"「這兒不興白打聽，要問拿誠意來。」",
		"「你身上氣息挺雜的，哪條道上的？」",
		"「閒話少說，我忙。」",
		"「初來乍到？先摸清地頭再說。」",
		"「別擋著光。」",
		"「有事說事。」",
		"「……唔。」",
		"「今日不宜多話。」",
		"「你找錯人了。」",
		"「街上人多口雜，別亂搭話。」",
		"「哦。」",
		"「沒見過像你這樣問的。」",
		"「要歇腳往客棧去。」",
		"「靈氣稀薄處少待，傷身。」",
		"「說完了？那我走了。」",
		"「你問的我不清楚。」",
		"「路還長，省點力氣吧。」",
		"「……隨便你。」",
		"「這年頭誰都不容易。」",
		"「別扯上我。」",
		"「有那閒心不如多練兩手。」",
		"「嗯，聽著呢。」",
		"「話多招禍。」",
		"「你問別人吧。」",
		"「沒什麼，隨便聊聊也行。」",
		"「初來？先找個地方住下。」",
		"「這兒就這樣，習慣就好。」",
		"「別耽誤我做事。」",
		"「……有事？」",
		"「風大，聽不清。」",
		"「你自便。」",
		"「少打聽，多做事。」",
		"「今日不順，別惹我。」",
		"「哦，然後呢？」",
		"「路過的？路過就快走。」",
		"「沒什麼好聊的。」",
		"「你誰？」",
		"「嗯。」",
		"「說吧，我聽著。」",
		"「這條街就這樣，熱鬧歸熱鬧，小心點。」",
		"「修行要緊，別瞎晃。」",
		"「……怎麼？」",
		"「有事明日再說。」",
		"「你找別人問去。」",
		"「沒空。」",
		"「隨便。」",
		"「唔。」",
		"「罷了，你說。」",
		"「聽過就忘，別外傳。」",
		"「這兒人多，不方便說。」",
		"「你倒是會挑人問。」",
		"「算了，當我沒說。」",
		"「……行吧。」",
		"「有什麼事？」",
		"「別礙事。」",
		"「嗯，你說。」",
		"「今日不宜久談。」",
		"「路過的人多了，你算一個。」",
		"「要幫手？找掌櫃。」",
		"「我沒什麼好說的。」",
		"「你問的我不懂。」",
		"「少來套近乎。」",
		"「……何事？」",
		"「聽著呢。」",
		"「有事快說。」",
		"「這地兒就這樣。」",
		"「別亂打聽。」",
		"「嗯哼。」",
		"「你自求多福。」",
		"「沒什麼。」",
		"「說。」",
		"「……哦。」",
		"「隨便聊聊可以。」",
		"「別耽誤工夫。」",
		"「今日沒興致。」",
		"「你問別人。」",
		"「聽到了。」",
		"「嗯，然後？」",
		"「有事？」",
		"「少廢話。」",
		"「……說完了？」",
		"「你忙你的。」",
		"「這條街規矩多。」",
		"「別惹事。」",
		"「唔，好吧。」",
		"「聽著。」",
		"「沒什麼好問的。」",
		"「你問吧。」",
		"「今日不宜多言。」",
		"「路過。」",
		"「……嗯。」",
		"「有事說。」",
		"「別擋道。」",
		"「隨便你。」",
		"「聽你的。」",
		"「沒空聊。」",
		"「你誰啊？」",
		"「嗯。」",
		"「說來聽聽。」",
		"「這兒不興多問。」",
		"「別多事。」",
		"「……好。」",
		"「有事？」",
		"「聽著呢。」",
		"「沒什麼。」",
		"「你說。」",
		"「今日事多。」",
		"「路過的都這樣問。」",
		"「……唔。」",
		"「有事快說。」",
		"「別礙著。」",
		"「隨便。」",
		"「聽到了。」",
		"「沒興趣。」",
		"「你問什麼？」",
		"「嗯。」",
		"「說吧。」",
		"「這地兒雜。」",
		"「別亂來。」",
		"「……哦。」",
		"「有事？」",
		"「聽著。」",
		"「沒啥。」",
		"「你說說看。」",
		"「今日忙。」",
		"「路過的人不少。」",
		"「……嗯。」",
		"「有事說事。」",
		"「別擋。」",
		"「隨你。」",
		"「聽。」",
		"「沒。」",
		"「你講。」",
		"「嗯。」",
		"「說。」",
		"「這兒就這樣。」",
		"「別。」",
		"「……。」",
	}
	h := 0
	for _, r := range target.ID {
		h += int(r)
	}
	now := game.NowUnix()
	// 用遊戲時間 + NPC ID hash 做 seed，同一刻同一 NPC 固定，不同時刻或不同 NPC 則分散，降低重複感
	fallbackSeed := int64(now)*1000 + int64(h)
	rng := rand.New(rand.NewSource(fallbackSeed))
	idx := rng.Intn(len(responses))
	if personality != nil {
		shift := int(personality.Boldness * float64(len(responses)/2))
		// Sensitivity 高→偏後（較多話／熱絡感）、低→偏前（較短／冷淡）
		if personality.Sensitivity > 0.6 {
			shift += len(responses) / 4
		} else if personality.Sensitivity < 0.3 {
			shift -= len(responses) / 4
		}
		idx = (idx + shift + len(responses)) % len(responses)
	}
	return "你向【" + name + "】搭話。" + name + "說道：" + responses[idx]
}
