// Package game 遊戲時間：1 真實小時 ＝ 1 遊戲日，日晷式 9→3 為日、3→9 為夜。
package game

const secondsPerGameDay = 86400

// GameTimeNow 依真實 Unix 秒與 epoch、scale 算出當前遊戲時間。
// 回傳：遊戲內當日 0:00 起的秒數（0～86399）、時（0～23）、分（0～59）、自 epoch 起算的遊戲日數（奇點曆用）。
func GameTimeNow(realUnix int64, epoch int64, scale float64) (secSinceMidnight int, hour, min int, daysSinceEpoch int) {
	if scale <= 0 {
		scale = 24
	}
	gameSec := float64(realUnix-epoch) * scale
	secTotal := int(gameSec)
	if secTotal < 0 {
		secTotal = 0
	}
	daysSinceEpoch = secTotal / secondsPerGameDay
	secSinceMidnight = secTotal % secondsPerGameDay
	hour = secSinceMidnight / 3600
	min = (secSinceMidnight % 3600) / 60
	return secSinceMidnight, hour, min, daysSinceEpoch
}
