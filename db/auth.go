// Package db 玩家登入密碼：entity_auth 表，僅 kind=player 有列（決策 006 選項甲）。
package db

import (
	"database/sql"

	"golang.org/x/crypto/bcrypt"
)

const bcryptCost = 10

// CreateAuth 為指定 entity_id 建立密碼（雜湊後寫入 entity_auth）。創角時呼叫。
func CreateAuth(db *sql.DB, entityID, password string) error {
	hash, err := bcrypt.GenerateFromPassword([]byte(password), bcryptCost)
	if err != nil {
		return err
	}
	_, err = db.Exec("INSERT INTO entity_auth (entity_id, password_hash) VALUES (?, ?)", entityID, string(hash))
	return err
}

// VerifyPassword 檢查 entity_id 對應之密碼是否正確；僅玩家有列，無列或錯誤回傳 false。
func VerifyPassword(db *sql.DB, entityID, password string) (bool, error) {
	var hash string
	err := db.QueryRow("SELECT password_hash FROM entity_auth WHERE entity_id = ?", entityID).Scan(&hash)
	if err == sql.ErrNoRows {
		return false, nil
	}
	if err != nil {
		return false, err
	}
	err = bcrypt.CompareHashAndPassword([]byte(hash), []byte(password))
	return err == nil, nil
}
