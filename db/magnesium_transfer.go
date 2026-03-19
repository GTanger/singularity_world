package db

import (
	"database/sql"
	"fmt"

	"singularity_world/store"
)

// TransferMagnesium 將 amount 鎂自 fromID 轉至 toID；餘額不足則錯誤。對齊經濟彙整 §四、物流規格金流。
func TransferMagnesium(database *sql.DB, fromID, toID string, amount int) error {
	if amount <= 0 {
		return fmt.Errorf("轉帳鎂數須為正")
	}
	if store.Default != nil {
		return store.Default.TransferMagnesiumBetween(fromID, toID, amount)
	}
	tx, err := database.Begin()
	if err != nil {
		return err
	}
	defer func() { _ = tx.Rollback() }()
	var fromMg int
	if err := tx.QueryRow("SELECT magnesium FROM entities WHERE id = ?", fromID).Scan(&fromMg); err != nil {
		return err
	}
	if fromMg < amount {
		return fmt.Errorf("鎂不足")
	}
	res, err := tx.Exec(
		"UPDATE entities SET magnesium = magnesium - ? WHERE id = ? AND magnesium >= ?",
		amount, fromID, amount,
	)
	if err != nil {
		return err
	}
	n, err := res.RowsAffected()
	if err != nil {
		return err
	}
	if n != 1 {
		return fmt.Errorf("鎂不足")
	}
	if _, err := tx.Exec("UPDATE entities SET magnesium = magnesium + ? WHERE id = ?", amount, toID); err != nil {
		return err
	}
	return tx.Commit()
}
