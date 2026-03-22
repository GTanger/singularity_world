// Package db NPC 日常消耗：每遊戲日扣減食宿鎂，驅動馬斯洛生存層。
package db

// DailyExpenseBase 每遊戲日基礎消耗鎂數（食宿）。
const DailyExpenseBase = 8

// DeductDailyExpense 扣減所有 NPC 的每日食宿消耗。
// 扣到 0 時記 EvtBroke 並呼叫 AdjustDisposition(DispBroke)；每人每日呼叫 AdjustDisposition(DispDaily)。
func DeductDailyExpense() int {
	ids, _ := GetNPCIDsWithRoom()
	affected := 0
	for _, id := range ids {
		c, _ := GetEntity(id)
		if c == nil {
			continue
		}
		if c.Magnesium > 0 {
			_ = AddMagnesium(id, -DailyExpenseBase)
			affected++
			newMag := c.Magnesium - DailyExpenseBase
			if newMag <= 0 {
				LogNPCEvent(id, EvtBroke, "鎂歸零")
				AdjustDisposition(id, DispBroke)
			}
		}
		AdjustDisposition(id, DispDaily)
	}
	return affected
}
