// Package db 玩家登入密碼：entity_auth 表，僅 kind=player 有列（決策 006 選項甲）。
package db

import (
	"singularity_world/store"

	"golang.org/x/crypto/bcrypt"
)

const bcryptCost = 10

// CreateAuth 為指定 entity_id 建立密碼；寫入 store 並持久化 auth.json。
func CreateAuth(entityID, password string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	hash, err := bcrypt.GenerateFromPassword([]byte(password), bcryptCost)
	if err != nil {
		return err
	}
	return store.Default.SetAuth(entityID, string(hash))
}

// HasPasswordForEntity 若 auth 中有該 entity 的密碼雜湊則為 true；無列或空字串視為未設定。
func HasPasswordForEntity(entityID string) bool {
	if store.Default == nil {
		return false
	}
	return store.Default.GetAuth(entityID) != ""
}

// VerifyPassword 檢查 entity_id 對應之密碼是否正確。
func VerifyPassword(entityID, password string) (bool, error) {
	if store.Default == nil {
		return false, ErrNoStore
	}
	hash := store.Default.GetAuth(entityID)
	if hash == "" {
		return false, nil
	}
	err := bcrypt.CompareHashAndPassword([]byte(hash), []byte(password))
	return err == nil, nil
}
