// Package db 房間與出口查詢、實體所在房間，供傳統 MUD 節點連接節點機制（僅 store／JSON）。
package db

import (
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"os"
	"strings"

	"singularity_world/entity"
	"singularity_world/model"
	"singularity_world/store"
)

// 與 model 一致，供既有程式與 JSON 使用；store 只依賴 model 以打破 import cycle。
type Room = model.Room
type Exit = model.Exit

// roomsFile 房間定義檔 JSON 結構。
type roomsFile struct {
	Rooms []roomDef `json:"rooms"`
	Exits []exitDef `json:"exits"`
}
type roomDef struct {
	ID          string   `json:"id"`
	Name        string   `json:"name"`
	Tags        []string `json:"tags"`
	Zone        string   `json:"zone"`
	Description string   `json:"description"`
}
type exitDef struct {
	From      string `json:"from"`
	Direction string `json:"direction"`
	To        string `json:"to"`
}

// GetRoom 依 id 查詢房間。
func GetRoom(id string) (*model.Room, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	return store.Default.GetRoom(id)
}

// TerrainFromRoom 依房間 tags／zone 回傳戰鬥地形標籤（lush／chaos／silent／grip），供 combat.CombatOpt 使用；無則空字串。
func TerrainFromRoom(roomID string) string {
	room, err := GetRoom(roomID)
	if err != nil || room == nil {
		return ""
	}
	for _, t := range room.Tags {
		switch strings.ToLower(strings.TrimSpace(t)) {
		case "lush", "chaos", "silent", "grip":
			return strings.ToLower(strings.TrimSpace(t))
		}
	}
	if strings.ToLower(strings.TrimSpace(room.Zone)) == "chaos" {
		return "chaos"
	}
	return ""
}

// GetRoomsByTag 回傳所有帶有指定 tag 的房間 ID。
func GetRoomsByTag(tag string) ([]string, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	return store.Default.GetRoomsByTag(tag), nil
}

// GetRoomsByZone 回傳指定 zone 中的所有房間 ID。
func GetRoomsByZone(zone string) ([]string, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	return store.Default.GetRoomsByZone(zone), nil
}

// GetExitsForRoom 回傳某房間的所有出口（含目標房間名稱）。
func GetExitsForRoom(fromRoomID string) ([]Exit, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	return store.Default.GetExitsForRoom(fromRoomID)
}

// GetEntitiesInRoom 回傳指定房間內的所有實體。
// gameHour 為當前遊戲小時 0–23，供「在職場且當班」顯示「職稱|真名」、下班僅真名；未知時傳 -1（不套用下班規則）。
func GetEntitiesInRoom(roomID string, gameHour int) ([]*entity.Character, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	ids := store.Default.EntityIDsInRoom(roomID)
	if len(ids) == 0 {
		return nil, nil
	}
	var list []*entity.Character
	for _, id := range ids {
		se := store.Default.GetEntity(id)
		if se == nil || se.Vit <= 0 {
			continue
		}
		title := ""
		if se.Kind == "npc" {
			title = GetNPCTitleInRoomAtHour(id, roomID, gameHour)
		}
		list = append(list, storeEntityToCharacter(se, title))
	}
	return list, nil
}

// GetEntityRoom 回傳實體當前房間 id。
func GetEntityRoom(entityID string) (string, error) {
	if store.Default == nil {
		return "", ErrNoStore
	}
	return store.Default.GetEntityRoom(entityID)
}

// SetEntityRoom 將實體設為在指定房間；寫入記憶體並原子寫回 runtime/entity_rooms.json。
func SetEntityRoom(entityID, roomID string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.SetEntityRoom(entityID, roomID)
}

// GetNPCIDsWithRoom 回傳所有「有房間」且 Vit>0（存活）的 NPC 實體 ID。
func GetNPCIDsWithRoom() ([]string, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	return store.Default.GetNPCIDsWithRoom(), nil
}

// GetPlayerIDsWithRoom 回傳所有有房間的玩家實體 ID。
func GetPlayerIDsWithRoom() ([]string, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	return store.Default.GetPlayerIDsWithRoom(), nil
}

// GetRoomName 查詢房間名稱。
func GetRoomName(roomID string) (string, error) {
	if store.Default == nil {
		return "", ErrNoStore
	}
	return store.Default.GetRoomName(roomID), nil
}

// SpawnRoomName 創生預設格之唯一名稱；讀到這個名稱即為玩家創生預設房間。
const SpawnRoomName = "界壁"

// GetRoomIDByName 依房間名稱回傳第一個符合的房間 id；若無則空字串。
func GetRoomIDByName(name string) (string, error) {
	if store.Default == nil {
		return "", ErrNoStore
	}
	return store.Default.GetRoomIDByName(name), nil
}

// GetSpawnRoomID 回傳創生預設房間 id（名稱為「界壁」的房間）；若無則回傳 "lobby" 以相容舊資料。
func GetSpawnRoomID() string {
	id, _ := GetRoomIDByName(SpawnRoomName)
	if id == "" {
		return "lobby"
	}
	return id
}

// RoomWithExits 房間與其出口列表，供管理 API 使用。
type RoomWithExits struct {
	Room  Room  `json:"room"`
	Exits []Exit `json:"exits"`
}

// ListAllRooms 回傳所有房間及各自出口。
func ListAllRooms() ([]RoomWithExits, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	var list []RoomWithExits
	for _, id := range store.Default.RoomIDs() {
		r, _ := store.Default.GetRoom(id)
		if r == nil {
			continue
		}
		exits, _ := store.Default.GetExitsForRoom(id)
		list = append(list, RoomWithExits{Room: *r, Exits: exits})
	}
	return list, nil
}

// CreateRoom 新增房間（記憶體；持久化依房間編輯器或部署流程）。
func CreateRoom(id, name, description string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	store.Default.UpsertRoomData(&model.Room{ID: id, Name: name, Description: description}, nil)
	return nil
}

// ErrRoomNotFound 表示找不到要更新的房間。
var ErrRoomNotFound = errors.New("room not found")

// UpdateRoom 更新房間名稱與描述；若 id 不存在則回傳 ErrRoomNotFound。
func UpdateRoom(id, name, description string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	r, _ := store.Default.GetRoom(id)
	if r == nil {
		return ErrRoomNotFound
	}
	r.Name = name
	r.Description = description
	exits, _ := store.Default.GetExitsForRoom(id)
	store.Default.UpsertRoomData(r, exits)
	return nil
}

// DeleteRoom 刪除房間（store 會將房內實體遷回安全房間）。
func DeleteRoom(id string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	store.Default.DeleteRoomData(id)
	return nil
}

// AddExit 新增一筆出口。
func AddExit(fromRoomID, direction, toRoomID string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	r, _ := store.Default.GetRoom(fromRoomID)
	if r == nil {
		return ErrRoomNotFound
	}
	exits, _ := store.Default.GetExitsForRoom(fromRoomID)
	for _, ex := range exits {
		if ex.Direction == direction {
			return fmt.Errorf("exit direction %q already exists", direction)
		}
	}
	exits = append(exits, model.Exit{Direction: direction, ToRoomID: toRoomID})
	store.Default.UpsertRoomData(r, exits)
	return nil
}

// RemoveExit 刪除一筆出口。
func RemoveExit(fromRoomID, direction string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	r, _ := store.Default.GetRoom(fromRoomID)
	if r == nil {
		return ErrRoomNotFound
	}
	exits, _ := store.Default.GetExitsForRoom(fromRoomID)
	var filtered []model.Exit
	for _, ex := range exits {
		if ex.Direction != direction {
			filtered = append(filtered, ex)
		}
	}
	store.Default.UpsertRoomData(r, filtered)
	return nil
}

// SyncRoomsFromFile 讀取 rooms.json，將房間與出口同步進 store（單檔舊格式）。
func SyncRoomsFromFile(path string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			log.Printf("rooms: %s not found, skip sync", path)
			return nil
		}
		return err
	}
	var f roomsFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}

	for _, r := range f.Rooms {
		room := &model.Room{ID: r.ID, Name: r.Name, Description: r.Description, Tags: r.Tags, Zone: r.Zone}
		var exits []model.Exit
		for _, e := range f.Exits {
			if e.From == r.ID {
				exits = append(exits, model.Exit{Direction: e.Direction, ToRoomID: e.To})
			}
		}
		store.Default.UpsertRoomData(room, exits)
	}

	spawnID := GetSpawnRoomID()
	for _, eid := range store.Default.AllEntityIDs() {
		rid, _ := store.Default.GetEntityRoom(eid)
		if rid == "" {
			_ = store.Default.SetEntityRoom(eid, spawnID)
		}
	}
	return nil
}

// SeedRooms 向下相容：若存在單檔 data/rooms.json（舊格式）則同步進 store；檔案不存在則略過（現行以 data/rooms/ 目錄載入為主）。
func SeedRooms() error {
	return SyncRoomsFromFile("data/rooms.json")
}
