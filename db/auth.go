// Package db 玩家登入密碼：entity_auth 表，僅 kind=player 有列（決策 006 選項甲）。
package db

import (
	"database/sql"

	"singularity_world/store"

	"golang.org/x/crypto/bcrypt"
)

const bcryptCost = 10

// CreateAuth 為指定 entity_id 建立密碼；store 啟用時寫入 store 並持久化 auth.json。
func CreateAuth(db *sql.DB, entityID, password string) error {
	hash, err := bcrypt.GenerateFromPassword([]byte(password), bcryptCost)
	if err != nil {
		return err
	}
	if store.Default != nil {
		return store.Default.SetAuth(entityID, string(hash))
	}
	_, err = db.Exec("INSERT INTO entity_auth (entity_id, password_hash) VALUES (?, ?)", entityID, string(hash))
	return err
}

// VerifyPassword 檢查 entity_id 對應之密碼是否正確；store 啟用時從 store 讀取。
func VerifyPassword(db *sql.DB, entityID, password string) (bool, error) {
	var hash string
	if store.Default != nil {
		hash = store.Default.GetAuth(entityID)
		if hash == "" {
			return false, nil
		}
	} else {
		err := db.QueryRow("SELECT password_hash FROM entity_auth WHERE entity_id = ?", entityID).Scan(&hash)
		if err == sql.ErrNoRows {
			return false, nil
		}
		if err != nil {
			return false, err
		}
	}
	err := bcrypt.CompareHashAndPassword([]byte(hash), []byte(password))
	return err == nil, nil
}
