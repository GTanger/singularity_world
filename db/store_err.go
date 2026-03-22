package db

import "errors"

// ErrNoStore 表示 store.Default 未初始化（專案已改為僅 JSON/store，不再使用 SQLite）。
var ErrNoStore = errors.New("store not initialized")

// ErrItemNotFound 表示 items 定義中無此 id。
var ErrItemNotFound = errors.New("item not found")
