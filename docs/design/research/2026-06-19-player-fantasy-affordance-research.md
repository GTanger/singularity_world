# 2026-06-19 玩家幻想與非日常可為性研究

狀態：外部研究筆記，不是奇點權威
第一權威：`docs/DESIGN_KEYWORDS.md`
對接文件：`docs/design/gdd/16-player-fantasy-affordances.md`

## 研究物件

本輪鎖定的不是「奇幻元素」或「場景氣氛」，而是使用者修正的遊戲觀問題：

> 遊戲如果不能讓玩家做到平常做不到的事，就沒有吸引力。

研究要回答：

- 這句話在遊戲設計上是否成立。
- 它是不是只等於「超能力」或「力量幻想」。
- 文字 MUD / 奇點世界應如何把它落成玩家可見的行動與後果。

## 外部資料重點

| 來源 | 查到的重點 | 對奇點的轉譯 |
|---|---|---|
| Przybylski, Rigby, Ryan, `A Motivational Model of Video Game Engagement` | 遊戲吸引力可用自我決定理論解釋；自主、勝任、關係需求能預測享受與未來遊玩。 | 「平常做不到」不能只理解為強大；它要讓玩家有自主選擇、能掌握一套規則、並和人/世界產生關係。 |
| Hunicke, LeBlanc, Zubek, `MDA` | 從玩家看，先是美感/體驗，再落到可觀察動態與可操作機制；`Fantasy` 是 make-believe，`Discovery` 是未知領域，`Expression` 是留下個人痕跡。 | 奇點不能從資料表或地名往上補吸引力；要先定玩家要獲得哪種非日常體驗，再反推動詞與狀態。 |
| Salen & Zimmerman, `Rules of Play / meaningful play` | 有意義遊玩來自行動與結果的關係；結果要可辨識，並整合進整體遊戲脈絡。 | 做到平常做不到的事，必須有可見後果。只寫「你感覺很特別」或只追加文字不算。 |
| James Paul Gee, `Good Games and Good Learning` | 好遊戲要求玩家住進一個身份，發現規則，並學會如何利用規則達成目標。 | 奇點的吸引力應來自「成為世界內某種人」並學會奇點規則，而不是當旁觀者讀設定。 |
| Bartle, `Hearts, Clubs, Diamonds, Spades` / MUD 玩家類型 | MUD 吸引力至少分散在成就、探索、社交、競合/衝突等不同玩家動機。 | 第一切片不能只給一種搬貨/賺錢小循環；要讓玩家看見多種非日常路徑。 |
| Raph Koster, `Theory of Fun` 後續文章 | 對遊戲系統設計有用的 fun 常是 mastery of problems；機制是在提出可被掌握的問題。 | 非日常能力不是白送爽感；它要形成能被玩家學會、熟練、變聰明的問題空間。 |
| Stang, `This Action Will Have Consequences` | 玩家 agency 常被過度宣稱，敘事選擇可能只是幻覺；真正的意義不必等於全局改變。 | 奇點不能承諾玩家改變整個世界；但必須讓局部世界事實真的改變，且玩家看得出來。 |
| Bogost, `The Rhetoric of Video Games` | 遊戲可透過程序與規則把玩家帶到另一種情境，也可用規則表達世界如何運作。 | 奇點的奇幻不能只是名詞；奇幻要藏在程序規則裡，讓玩家透過操作理解世界。 |

## 審辯

### 候選 A：遊戲吸引力來自平常做不到的事

支持：

- `Fantasy`、`Discovery`、`Expression` 本身就指向非日常身份、未知領域與自我痕跡。
- 角色扮演、MUD、奇幻世界若只還原日常瑣事，會失去進入另一個世界的理由。
- 奇點關鍵字中的 `探索`、`冒險`、`自創技能`、`玩家 = NPC`、`世界永遠在線` 都不是現實日常。

限制：

- 若只追求「現實做不到」，很容易變成亂加超能力、奇觀、專有名詞。
- 不一定要讓玩家全能；小範圍但真實的非日常後果，比大而假的全局選擇更有力。

### 候選 B：遊戲吸引力主要來自有意義後果

支持：

- Meaningful play 強調行動與結果的可辨識、整合。
- 現實可做的事也能在遊戲裡有吸引力，例如種田、開車、整理、經營；關鍵是遊戲讓它們有壓縮後的回饋、成長與後果。

限制：

- 只講後果會變成泛用框架，無法解決奇點目前「無感」問題。
- 如果後果只服務日常瑣事，玩家仍會問：我為什麼不去玩更直接、更爽的遊戲？

### 候選 C：遊戲吸引力主要來自掌握問題

支持：

- Koster 的 mastery of problems 能解釋為何困難、風險、系統學習會好玩。
- 奇點的 `有限框架延伸無限可能` 需要玩家逐步掌握規則，而不是只看演出。

限制：

- 只講掌握會把設計收窄成解謎/效率/管理。
- 若缺少非日常身份與幻想，玩家只會覺得在做系統練習。

## 修正結論

使用者的方向成立，但要精確化：

> 奇點需要的不是「日常模擬」，而是「非日常可為性」：玩家能以另一個身份，在一個現實沒有的世界規則裡，做現實不能、不能安全做、不能長期做、不能被世界承認的事，並看到局部世界真的因此改變。

這不等於把玩家做成救世主、超人或上帝。奇點的限制仍然成立：

- 玩家不是主角。
- 玩家不是上帝。
- 玩家與 NPC 同規則。
- 不可無中生有。
- 後端世界事實要守衡。

但限制不能把遊戲壓成現實雜務。限制的作用是讓非日常能力有代價、有邊界、有可學習性。

## Simple-first 修正

使用者進一步修正：遊戲不一定要複雜，沒有簡單何來複雜。

外部資料支持這個修正：

| 來源 | 查到的重點 | 對奇點的修正 |
|---|---|---|
| Salen & Zimmerman, `Rules of Play` | Meaningful play 來自玩家行動與系統結果的關係；結果必須可辨識並整合進更大脈絡。複雜與 meaningful play 不是同一件事。 | 第一個核心先做「一個動作 -> 一個可見結果 -> 接回世界」，不是先列多入口、多 NPC、多經濟流。 |
| Salen & Zimmerman, emergent systems 章 | 少量規則變體就可能推動系統越過 meaningful play 的門檻；關鍵是讓行動結果不再任意，且能接進後續脈絡。 | 複雜可以由少量好規則長出來，不需要一開始就架完整大系統。 |
| MDA | 設計者直接做 mechanics；dynamics 與 aesthetics 從 mechanics 的運行中產生。 | 奇點應先做最小可操作 mechanic，讓玩家看到 dynamic，再談世界味與長期美感。 |
| Raph Koster, `Game design is simple, actually` | 核心 loop 是玩家看問題、形成假設、操作、看結果、更新判斷；動詞可以重複，但情境不能完全相同。 | 第一屏應先有「看狀態、點入口、做動詞、看差異」；深度來自同一動詞在不同情境下反覆測試。 |
| Raph Koster, feedback 段 | 玩家需要知道可用動作、知道自己用了動作、看見狀態怎麼變，並知道變化對目標好壞。 | 奇點第一個互動要先把回饋做清楚，不要把沉浸感壓在長描述或多系統上。 |
| Game Developer, `Easy to Learn, Hard to Master` | 簡單是可接近性與設計的重要屬性。 | 手機文字 MUD 第一個核心應易懂，不應用深系統門檻把玩家擋在外面。 |

因此，`非日常可為性` 的落地順序要改為：

```text
1. 一個玩家可讀狀態。
2. 一個玩家可點入口。
3. 一個玩家動詞。
4. 一個世界狀態差異。
5. 一句清楚回饋。
6. 同一動詞在不同情境中反覆出現。
7. 多個簡單核心再連成 NPC、經濟、社會、離線與長期痕跡。
```

錯誤順序：

```text
世界根架構 -> 多場景 -> 多 NPC -> 多入口 -> 多經濟流 -> 期待玩家自然覺得好玩
```

正確順序：

```text
一個簡單好玩的非日常動作 -> 一個清楚回饋 -> 一個可回訪差異 -> 再擴成更多情境
```

## 奇點可用的非日常可為性分類

| 類型 | 玩家平常做不到什麼 | 奇點可給的遊戲承諾 |
|---|---|---|
| 身份越界 | 成為另一個世界裡有身體、位置、信用與被記住的人 | 玩家不是看故事，而是住進一個可累積的位置 |
| 世界越界 | 下線後世界與角色仍推進 | 離線安排、錯過、回訪痕跡成為玩法 |
| 規則越界 | 操作現實沒有的奇幻物候、元素、材料與地貌規則 | 讀霧、避雷、處理異常材料、用工具改變局勢 |
| 物件越界 | 幾乎每個看得見的東西都能被碰、用、修、搬、堵、交易 | 物件不是背景，而是動詞來源 |
| 社會越界 | 在陌生地方逐步被信任、拒絕、記住、利用或防備 | 關係與信用成為可玩資源 |
| 風險越界 | 冒險、衝突、危險嘗試不會造成現實傷害 | 可承擔虛構後果，學會風險判斷 |
| 建造/經營越界 | 改善一個地方，讓 NPC 與世界後續使用它 | 棚、路、倉、牌、工具、秩序能留下長期痕跡 |
| 觀測越界 | 透過接觸把未定故事線承認成局部事實 | 調查、跟蹤、詢問、驗證傳聞會改變玩家知道的世界 |

## 設計檢查句

每個場景、物件、NPC、元素與第一屏，都要能回答：

1. 玩家在這裡能做哪件現實生活中不能、不能安全做、不能長期做、或不能被世界承認的事？
2. 這件事是玩家親手做的，還是只是文字告訴玩家「你感覺很特別」？
3. 它是否滿足至少一種：自主、勝任、關係、探索、表達、風險、身份？
4. 它是否有可見世界後果？
5. 第二次來時是否因玩家/NPC/時間而不同？

若五題答不出來，該內容即使合理、漂亮、常理化，也缺乏遊戲吸引力。

## 來源

- Przybylski, Rigby, Ryan, `A Motivational Model of Video Game Engagement`: https://selfdeterminationtheory.org/SDT/documents/2010_PrzybylskiRigbyRyan_ROGP.pdf
- Hunicke, LeBlanc, Zubek, `MDA: A Formal Approach to Game Design and Game Research`: https://users.cs.northwestern.edu/~hunicke/MDA.pdf
- Salen & Zimmerman, `Rules of Play / Meaningful Play` excerpt: https://sites.cc.gatech.edu/classes/AY2006/cs4455_spring/documents/sz-unit11.pdf
- James Paul Gee, `Good Games and Good Learning`: https://jamespaulgee.com/pubs/Good%20Games%20and%20Good%20Learning.pdf
- Skeleton Code Machine summary of Bartle taxonomy, linking Bartle's original MUD paper: https://www.skeletoncodemachine.com/p/bartle-taxonomy-of-player-types
- Raph Koster, `Game design is simple, actually`: https://www.raphkoster.com/2025/11/03/game-design-is-simple-actually/
- Stang, `This Action Will Have Consequences`: https://gamestudies.org/1901/articles/stang
- Bogost, `The Rhetoric of Video Games`: https://se4n.org/papers/bogost-rhetoric.pdf
- Justin Neft, `Easy to Learn, Hard to Master`: https://www.gamedeveloper.com/design/easy-to-learn-hard-to-master
