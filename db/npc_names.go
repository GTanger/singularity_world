package db

import (
	"math/rand"
	"unicode/utf8"
)

// 單姓（一字）：常見姓，便於隨機取一字。
var singleSurnames = []rune("王李張劉陳楊黃趙周吳徐孫馬朱胡郭何高林羅鄭梁謝宋唐許韓馮鄧曹彭曾蕭田董潘袁蔡蔣余杜葉程蘇魏呂丁任沈姚盧姜崔鍾譚陸汪范金石廖賈夏韋傅方白鄒孟熊秦邱江尹薛閻段雷龍黎史陶賀顧毛郝龔邵萬錢嚴覃武戴莫孔向湯")

// 複姓（二字）：取一個即為兩字。
var compoundSurnames = []string{
	"歐陽", "司馬", "諸葛", "上官", "司徒", "司空", "皇甫", "慕容",
	"宇文", "長孫", "聞人", "東方", "獨孤", "南宮", "萬俟", "尉遲",
}

// 名字用字（單字名或雙字名的字庫）：取 1 字＝單名，取 2 字＝雙名。
var givenNameChars = []rune("明華強建國志偉芳娜敏靜麗磊軍洋勇艷杰濤超秀英慧萍玲紅梅飛燕霞蘭平康安寧俊傑慧敏雅琪俊偉志明淑芬建國美玲志偉淑惠")

func init() {
	if len(singleSurnames) == 0 || len(givenNameChars) == 0 {
		panic("npc_names: name lists must be non-empty")
	}
}

// GenerateNPCName 產生「姓＋名」共 2～4 字：單姓(1)＋單名(1)＝2，單姓＋雙名 或 複姓＋單名＝3，複姓＋雙名＝4。
// gender 目前未用，預留給日後男女用字分流（如 "M"/"F"）。
func GenerateNPCName(gender string) string {
	useCompound := rand.Intn(5) == 0 // 約 20% 複姓
	var surname string
	if useCompound {
		surname = compoundSurnames[rand.Intn(len(compoundSurnames))]
	} else {
		surname = string(singleSurnames[rand.Intn(len(singleSurnames))])
	}
	// 名：1 字或 2 字，使總長為 2～4
	givenLen := 1 + rand.Intn(2) // 1 或 2
	given := make([]rune, givenLen)
	for i := 0; i < givenLen; i++ {
		given[i] = givenNameChars[rand.Intn(len(givenNameChars))]
	}
	return surname + string(given)
}

// FirstRune 回傳 s 的第一個 rune 字串；若為空則回傳 "人"。
func FirstRune(s string) string {
	if s == "" {
		return "人"
	}
	r, _ := utf8.DecodeRuneInString(s)
	if r == utf8.RuneError {
		return "人"
	}
	return string(r)
}
