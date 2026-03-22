package db

import (
	"fmt"

	"singularity_world/store"
)

// TransferMagnesium 將 amount 鎂自 fromID 轉至 toID；餘額不足則錯誤。
func TransferMagnesium(fromID, toID string, amount int) error {
	if amount <= 0 {
		return fmt.Errorf("轉帳鎂數須為正")
	}
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.TransferMagnesiumBetween(fromID, toID, amount)
}
