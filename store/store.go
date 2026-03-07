// Package store 提供以 JSON 為唯一數據源的記憶體層（無 DB）：
// 啟動時從指定 JSON 檔與目錄載入全部資料，執行期只讀寫記憶體，必要時原子寫回對應 JSON。
// store.Default 初始化後，db 層的房間／實體／排班等皆由此讀寫。
package store

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
	"sync"

	"singularity_world/model"
)

// Default 全域 store；Init 後 db 的房間／entity_room 會優先由此提供。
var Default *Store

var defaultMu sync.RWMutex

// Venue 場所：id、名稱、room_ids（與 db.Venue 對齊，供 JSON 背板）。
type Venue struct {
	ID      string   `json:"id"`
	Name    string   `json:"name"`
	RoomIDs []string `json:"room_ids"`
}

// Assignment 指派：誰、什麼職業、哪個場所（與 db.Assignment 對齊）。
type Assignment struct {
	EntityID     string `json:"entity_id"`
	OccupationID string `json:"occupation_id"`
	VenueID      string `json:"venue_id"`
	AssignedBy   string `json:"assigned_by,omitempty"`
}

// Schedule 排班：工作房、休息房、班次起迄（與 db.NPCSchedule 對齊）。
type Schedule struct {
	EntityID   string `json:"entity_id"`
	WorkRoom   string `json:"work_room"`
	RestRoom   string `json:"rest_room"`
	ShiftStart int    `json:"shift_start"`
	ShiftEnd   int    `json:"shift_end"`
}

// Entity 實體（玩家/NPC）與 entity.Character 對齊，供 JSON 背板。
type Entity struct {
	ID             string  `json:"id"`
	Kind           string  `json:"kind"`
	DisplayChar    string  `json:"display_char"`
	X              int     `json:"x"`
	Y              int     `json:"y"`
	MoveState      string  `json:"move_state"`
	TargetX        *int    `json:"target_x,omitempty"`
	TargetY        *int    `json:"target_y,omitempty"`
	WalkOrRun      string  `json:"walk_or_run,omitempty"`
	MoveStartedAt  *int64  `json:"move_started_at,omitempty"`
	Vit            int     `json:"vit"`
	Qi             int     `json:"qi"`
	Dex            int     `json:"dex"`
	Magnesium      int     `json:"magnesium"`
	LastObservedAt *int64  `json:"last_observed_at,omitempty"`
	CreatedAt      int64   `json:"created_at"`
	Gender         string  `json:"gender,omitempty"`
	SoulSeed       *int64  `json:"soul_seed,omitempty"`
	DisplayTitle   string  `json:"display_title,omitempty"`
	ActivatedNodes string  `json:"activated_nodes,omitempty"`
	EquipmentSlots string  `json:"equipment_slots,omitempty"`
	Inventory      string  `json:"inventory,omitempty"`
}

// Item 物品定義（與 items 表對齊）。
type Item struct {
	ID           string  `json:"id"`
	Name         string  `json:"name"`
	Slot         string  `json:"slot"`
	ItemType     string  `json:"item_type"`
	Weight       float64 `json:"weight"`
	Stackable    int     `json:"stackable"`
	Denomination int     `json:"denomination"`
	Description  string  `json:"description"`
}

// EventEntry 事件日誌一筆。
type EventEntry struct {
	At        int64  `json:"at"`
	EntityID  string `json:"entity_id"`
	EventType string `json:"event_type"`
	Payload   string `json:"payload"`
}

// Store 記憶體中的房間、出口、實體、場所、指派、排班、物品、事件日誌、密碼。
type Store struct {
	mu               sync.RWMutex
	Rooms            map[string]*model.Room   // id -> Room
	Exits            map[string][]model.Exit // from_room_id -> 出口列表
	EntityRooms      map[string]string        // entity_id -> room_id
	Venues           map[string]*Venue       // id -> Venue
	Assignments      map[string][]Assignment // entity_id -> 指派列表
	Schedules        map[string]*Schedule    // entity_id -> 排班
	Entities         map[string]*Entity     // id -> Entity
	Items            map[string]*Item       // id -> Item
	EventLog         []EventEntry            // 事件日誌（append）
	Auth             map[string]string       // entity_id -> password_hash
	runtimeDir       string
	entityRoomsPath  string
	assignmentsPath  string
	schedulesPath    string
	venuesPath       string
	entitiesPath     string
	itemsPath        string
	eventLogPath     string
	authPath         string
	roomsPath        string // 房間來源：目錄 data/rooms 或單檔
}

// roomsFile 單檔格式（舊版 data/rooms.json：rooms + exits 陣列）。
type roomsFile struct {
	Rooms []roomDef `json:"rooms"`
	Exits []exitDef `json:"exits"`
}

// roomFileOne 一房一檔格式（data/rooms/<id>.json）：單一房間含其出口。
type roomFileOne struct {
	ID          string     `json:"id"`
	Name        string     `json:"name"`
	Description string     `json:"description"`
	Tags        []string   `json:"tags"`
	Zone        string     `json:"zone"`
	Exits       []exitOut  `json:"exits"`
}
type exitOut struct {
	Direction string `json:"direction"`
	To       string `json:"to"`
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

// entityRoomsFile 用於讀寫 runtime/entity_rooms.json。
type entityRoomsFile struct {
	Entries []entityRoomEntry `json:"entries"`
}
type entityRoomEntry struct {
	EntityID string `json:"entity_id"`
	RoomID   string `json:"room_id"`
}

type venuesFile struct {
	Venues []Venue `json:"venues"`
}
type assignmentsFile struct {
	Entries []Assignment `json:"entries"`
}
type schedulesFile struct {
	Entries []Schedule `json:"entries"`
}

type entitiesFile struct {
	Entities []Entity `json:"entities"`
}

type itemsFile struct {
	Items []Item `json:"items"`
}

type eventLogFile struct {
	Entries []EventEntry `json:"entries"`
}

type authFile struct {
	Entries []authEntry `json:"entries"`
}
type authEntry struct {
	EntityID     string `json:"entity_id"`
	PasswordHash string `json:"password_hash"`
}

// Init 從 roomsPath 載入房間與出口，從 runtimeDir 載入 entity_rooms；若 dataDir 非空則再載入 venues/assignments/schedules。
// 完成後設定 store.Default，供 db 與 pathfind 使用。
func Init(roomsPath, runtimeDir, dataDir string) error {
	defaultMu.Lock()
	defer defaultMu.Unlock()

	s := &Store{
		Rooms:            make(map[string]*model.Room),
		Exits:            make(map[string][]model.Exit),
		EntityRooms:      make(map[string]string),
		Venues:           make(map[string]*Venue),
		Assignments:      make(map[string][]Assignment),
		Schedules:        make(map[string]*Schedule),
		Entities:         make(map[string]*Entity),
		Items:            make(map[string]*Item),
		EventLog:         nil,
		Auth:             make(map[string]string),
		runtimeDir:       runtimeDir,
		entityRoomsPath:  filepath.Join(runtimeDir, "entity_rooms.json"),
		assignmentsPath:  filepath.Join(dataDir, "assignments.json"),
		schedulesPath:    filepath.Join(dataDir, "schedules.json"),
		venuesPath:       filepath.Join(dataDir, "venues.json"),
		entitiesPath:     filepath.Join(dataDir, "entities.json"),
		itemsPath:        filepath.Join(dataDir, "items.json"),
		eventLogPath:     filepath.Join(runtimeDir, "event_log.json"),
		authPath:         filepath.Join(runtimeDir, "auth.json"),
	}

	if err := s.loadRooms(roomsPath); err != nil {
		return err
	}
	s.roomsPath = roomsPath
	_ = s.loadEntityRooms()
	if dataDir != "" {
		_ = s.loadVenues()
		_ = s.loadAssignments()
		_ = s.loadSchedules()
		_ = s.loadEntities()
		_ = s.loadItems()
		_ = s.loadEventLog()
		_ = s.loadAuth()
	}

	Default = s
	log.Printf("[store] loaded: %d rooms, %d entity_room, %d venues, %d assignments, %d schedules, %d entities, %d items, %d event_log, %d auth",
		len(s.Rooms), len(s.EntityRooms), len(s.Venues), s.assignmentsCount(), len(s.Schedules), len(s.Entities), len(s.Items), len(s.EventLog), len(s.Auth))
	return nil
}

func (s *Store) assignmentsCount() int {
	n := 0
	for _, list := range s.Assignments {
		n += len(list)
	}
	return n
}

func (s *Store) loadRooms(path string) error {
	fi, err := os.Stat(path)
	if err != nil {
		return err
	}
	if fi.IsDir() {
		return s.loadRoomsFromDir(path)
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	var f roomsFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	nameByID := make(map[string]string)
	for i := range f.Rooms {
		r := &f.Rooms[i]
		s.Rooms[r.ID] = &model.Room{
			ID:          r.ID,
			Name:        r.Name,
			Tags:        r.Tags,
			Zone:        r.Zone,
			Description: r.Description,
		}
		nameByID[r.ID] = r.Name
	}
	for _, e := range f.Exits {
		toName := nameByID[e.To]
		s.Exits[e.From] = append(s.Exits[e.From], model.Exit{
			Direction:  e.Direction,
			ToRoomID:   e.To,
			ToRoomName: toName,
		})
	}
	return nil
}

func (s *Store) loadRoomsFromDir(dir string) error {
	entries, err := os.ReadDir(dir)
	if err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	// 先掃一遍只建房間，再補出口（出口的 ToRoomName 需房間名）
	nameByID := make(map[string]string)
	for _, e := range entries {
		if e.IsDir() || filepath.Ext(e.Name()) != ".json" {
			continue
		}
		data, err := os.ReadFile(filepath.Join(dir, e.Name()))
		if err != nil {
			return err
		}
		var one roomFileOne
		if err := json.Unmarshal(data, &one); err != nil {
			return err
		}
		s.Rooms[one.ID] = &model.Room{
			ID:          one.ID,
			Name:        one.Name,
			Tags:        one.Tags,
			Zone:        one.Zone,
			Description: one.Description,
		}
		nameByID[one.ID] = one.Name
	}
	for _, e := range entries {
		if e.IsDir() || filepath.Ext(e.Name()) != ".json" {
			continue
		}
		data, _ := os.ReadFile(filepath.Join(dir, e.Name()))
		var one roomFileOne
		if json.Unmarshal(data, &one) != nil {
			continue
		}
		for _, ex := range one.Exits {
			toName := nameByID[ex.To]
			s.Exits[one.ID] = append(s.Exits[one.ID], model.Exit{
				Direction:  ex.Direction,
				ToRoomID:   ex.To,
				ToRoomName: toName,
			})
		}
	}
	return nil
}

func (s *Store) loadEntityRooms() error {
	data, err := os.ReadFile(s.entityRoomsPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var f entityRoomsFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for _, e := range f.Entries {
		s.EntityRooms[e.EntityID] = e.RoomID
	}
	return nil
}

// RoomIDs 回傳所有房間 ID（供 ListAllRooms 等安全迭代用）。
func (s *Store) RoomIDs() []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	ids := make([]string, 0, len(s.Rooms))
	for id := range s.Rooms {
		ids = append(ids, id)
	}
	return ids
}

// GetRoom 回傳房間；若無則 nil, nil。
func (s *Store) GetRoom(id string) (*model.Room, error) {
	if s == nil {
		return nil, nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	r, ok := s.Rooms[id]
	if !ok {
		return nil, nil
	}
	// 回傳副本，避免呼叫端改動
	cp := *r
	return &cp, nil
}

// GetExitsForRoom 回傳某房間的所有出口。
func (s *Store) GetExitsForRoom(fromRoomID string) ([]model.Exit, error) {
	if s == nil {
		return nil, nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.Exits[fromRoomID], nil
}

// GetRoomName 回傳房間名稱；若無則空字串。
func (s *Store) GetRoomName(roomID string) string {
	if s == nil {
		return ""
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	r, ok := s.Rooms[roomID]
	if !ok {
		return ""
	}
	return r.Name
}

// GetRoomsByTag 回傳所有帶有指定 tag 的房間 ID。
func (s *Store) GetRoomsByTag(tag string) []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []string
	for id, r := range s.Rooms {
		for _, t := range r.Tags {
			if t == tag {
				out = append(out, id)
				break
			}
		}
	}
	return out
}

// GetRoomsByZone 回傳指定 zone 內所有房間 ID。
func (s *Store) GetRoomsByZone(zone string) []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []string
	for id, r := range s.Rooms {
		if r.Zone == zone {
			out = append(out, id)
		}
	}
	return out
}

// GetEntityRoom 回傳實體所在房間 ID；若無則空字串。
func (s *Store) GetEntityRoom(entityID string) (string, error) {
	if s == nil {
		return "", nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.EntityRooms[entityID], nil
}

// SetEntityRoom 設定實體所在房間，並原子寫回 runtime/entity_rooms.json。
func (s *Store) SetEntityRoom(entityID, roomID string) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	s.EntityRooms[entityID] = roomID
	s.mu.Unlock()
	return s.persistEntityRooms()
}

// persistEntityRooms 將 EntityRooms 寫入 entity_rooms.json（原子寫入）。
func (s *Store) persistEntityRooms() error {
	if s.entityRoomsPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.entityRoomsPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	entries := make([]entityRoomEntry, 0, len(s.EntityRooms))
	for eid, rid := range s.EntityRooms {
		entries = append(entries, entityRoomEntry{EntityID: eid, RoomID: rid})
	}
	s.mu.RUnlock()

	raw, err := json.MarshalIndent(entityRoomsFile{Entries: entries}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.entityRoomsPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.entityRoomsPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

// Adjacency 供 pathfind 建圖：回傳 room_id -> 相鄰 room_id 列表。
func (s *Store) Adjacency() map[string][]string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	adj := make(map[string][]string)
	for id := range s.Rooms {
		adj[id] = nil
	}
	for from, exits := range s.Exits {
		for _, e := range exits {
			adj[from] = append(adj[from], e.ToRoomID)
		}
	}
	return adj
}

// NameMap 回傳 room_id -> name，供 pathfind 建圖。
func (s *Store) NameMap() map[string]string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make(map[string]string, len(s.Rooms))
	for id, r := range s.Rooms {
		out[id] = r.Name
	}
	return out
}

// ZoneMap 回傳 room_id -> zone。
func (s *Store) ZoneMap() map[string]string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make(map[string]string, len(s.Rooms))
	for id, r := range s.Rooms {
		out[id] = r.Zone
	}
	return out
}

// RoomTagsMap 回傳 room_id -> tags 切片，供 pathfind 使用。
func (s *Store) RoomTagsMap() map[string][]string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make(map[string][]string)
	for id, r := range s.Rooms {
		out[id] = r.Tags
	}
	return out
}

// EntityIDsInRoom 回傳在指定房間內的所有實體 ID（供 GetEntitiesInRoom 查 entities 用）。
func (s *Store) EntityIDsInRoom(roomID string) []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []string
	for eid, rid := range s.EntityRooms {
		if rid == roomID {
			out = append(out, eid)
		}
	}
	return out
}

func (s *Store) loadVenues() error {
	if s.venuesPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.venuesPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var f venuesFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for i := range f.Venues {
		v := &f.Venues[i]
		s.Venues[v.ID] = v
	}
	return nil
}

func (s *Store) loadAssignments() error {
	if s.assignmentsPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.assignmentsPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var f assignmentsFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for _, a := range f.Entries {
		s.Assignments[a.EntityID] = append(s.Assignments[a.EntityID], a)
	}
	return nil
}

func (s *Store) loadSchedules() error {
	if s.schedulesPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.schedulesPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var f schedulesFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for i := range f.Entries {
		sch := &f.Entries[i]
		s.Schedules[sch.EntityID] = sch
	}
	return nil
}

// GetVenue 回傳場所；若無則 nil。
func (s *Store) GetVenue(id string) *Venue {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	v, ok := s.Venues[id]
	if !ok || v == nil {
		return nil
	}
	cp := *v
	return &cp
}

// IsRoomInVenue 判斷房間是否在該場所的 room_ids 內。
func (s *Store) IsRoomInVenue(roomID, venueID string) bool {
	if s == nil {
		return false
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	v, ok := s.Venues[venueID]
	if !ok || v == nil {
		return false
	}
	for _, id := range v.RoomIDs {
		if id == roomID {
			return true
		}
	}
	return false
}

// GetAssignmentsForEntity 回傳某實體的全部指派。
func (s *Store) GetAssignmentsForEntity(entityID string) []Assignment {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	list := s.Assignments[entityID]
	if len(list) == 0 {
		return nil
	}
	out := make([]Assignment, len(list))
	copy(out, list)
	return out
}

// InsertAssignment 新增一筆指派；若已存在（同 entity+occupation+venue）則忽略。會寫回 assignments.json。
func (s *Store) InsertAssignment(entityID, occupationID, venueID, assignedBy string) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	list := s.Assignments[entityID]
	for _, a := range list {
		if a.OccupationID == occupationID && a.VenueID == venueID {
			s.mu.Unlock()
			return nil
		}
	}
	s.Assignments[entityID] = append(list, Assignment{
		EntityID: entityID, OccupationID: occupationID, VenueID: venueID, AssignedBy: assignedBy,
	})
	s.mu.Unlock()
	return s.persistAssignments()
}

// GetAllSchedules 回傳所有排班（供 main 註冊 TravelerManager）。
func (s *Store) GetAllSchedules() []Schedule {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []Schedule
	for _, sch := range s.Schedules {
		if sch != nil {
			out = append(out, *sch)
		}
	}
	return out
}

// GetSchedule 回傳單一實體的排班；若無則 nil。
func (s *Store) GetSchedule(entityID string) *Schedule {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	sch, ok := s.Schedules[entityID]
	if !ok || sch == nil {
		return nil
	}
	cp := *sch
	return &cp
}

// InsertSchedule 新增或覆寫一筆排班；會寫回 schedules.json。
func (s *Store) InsertSchedule(entityID, workRoom, restRoom string, shiftStart, shiftEnd int) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	s.Schedules[entityID] = &Schedule{
		EntityID: entityID, WorkRoom: workRoom, RestRoom: restRoom,
		ShiftStart: shiftStart, ShiftEnd: shiftEnd,
	}
	s.mu.Unlock()
	return s.persistSchedules()
}

func (s *Store) persistAssignments() error {
	if s.assignmentsPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.assignmentsPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	var entries []Assignment
	for _, list := range s.Assignments {
		entries = append(entries, list...)
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(assignmentsFile{Entries: entries}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.assignmentsPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.assignmentsPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

func (s *Store) persistSchedules() error {
	if s.schedulesPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.schedulesPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	var entries []Schedule
	for _, sch := range s.Schedules {
		if sch != nil {
			entries = append(entries, *sch)
		}
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(schedulesFile{Entries: entries}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.schedulesPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.schedulesPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

func (s *Store) loadEntities() error {
	if s.entitiesPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.entitiesPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var f entitiesFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for i := range f.Entities {
		e := &f.Entities[i]
		if e.ActivatedNodes == "" {
			e.ActivatedNodes = `["N000"]`
		}
		if e.Inventory == "" {
			e.Inventory = "[]"
		}
		s.Entities[e.ID] = e
	}
	return nil
}

func (s *Store) loadItems() error {
	if s.itemsPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.itemsPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var f itemsFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for i := range f.Items {
		it := &f.Items[i]
		s.Items[it.ID] = it
	}
	return nil
}

func (s *Store) loadEventLog() error {
	if s.eventLogPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.eventLogPath)
	if err != nil {
		if os.IsNotExist(err) {
			s.EventLog = []EventEntry{}
			return nil
		}
		return err
	}
	var f eventLogFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	s.EventLog = f.Entries
	if s.EventLog == nil {
		s.EventLog = []EventEntry{}
	}
	return nil
}

func (s *Store) loadAuth() error {
	if s.authPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.authPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var f authFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for _, e := range f.Entries {
		s.Auth[e.EntityID] = e.PasswordHash
	}
	return nil
}

// GetEntity 回傳實體；若無則 nil。
func (s *Store) GetEntity(id string) *Entity {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	e, ok := s.Entities[id]
	if !ok || e == nil {
		return nil
	}
	cp := *e
	return &cp
}

// PutEntity 新增或覆寫一筆實體並持久化 entities.json。
func (s *Store) PutEntity(e *Entity) error {
	if s == nil || e == nil {
		return nil
	}
	s.mu.Lock()
	cp := *e
	if cp.ActivatedNodes == "" {
		cp.ActivatedNodes = `["N000"]`
	}
	if cp.Inventory == "" {
		cp.Inventory = "[]"
	}
	s.Entities[e.ID] = &cp
	s.mu.Unlock()
	return s.persistEntities()
}

// UpdateEntity 對指定實體執行 fn，然後持久化。若實體不存在則不寫入。
func (s *Store) UpdateEntity(id string, fn func(*Entity)) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	e, ok := s.Entities[id]
	if !ok || e == nil {
		s.mu.Unlock()
		return nil
	}
	// 在副本上修改，避免並發讀到半更新
	cp := *e
	fn(&cp)
	s.Entities[id] = &cp
	s.mu.Unlock()
	return s.persistEntities()
}

func (s *Store) persistEntities() error {
	if s.entitiesPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.entitiesPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	var list []Entity
	for _, e := range s.Entities {
		if e != nil {
			list = append(list, *e)
		}
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(entitiesFile{Entities: list}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.entitiesPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.entitiesPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

// GetEntitiesInBox 回傳座標落在 [xMin,xMax]×[yMin,yMax] 內的實體；kind 為 "npc" 僅 NPC，空字串為全部。
func (s *Store) GetEntitiesInBox(xMin, xMax, yMin, yMax int, kind string) []*Entity {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []*Entity
	for _, e := range s.Entities {
		if e == nil {
			continue
		}
		if kind != "" && e.Kind != kind {
			continue
		}
		if e.X >= xMin && e.X <= xMax && e.Y >= yMin && e.Y <= yMax {
			cp := *e
			out = append(out, &cp)
		}
	}
	return out
}

// GetMovingEntityIDs 回傳所有 move_state = moving 且 target 非空的實體 ID（供 db.GetMovingEntities 用）。
func (s *Store) GetMovingEntityIDs() []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var ids []string
	for id, e := range s.Entities {
		if e != nil && e.MoveState == "moving" && e.TargetX != nil && e.TargetY != nil {
			ids = append(ids, id)
		}
	}
	return ids
}

// ClearAllEntities 清空所有實體、entity_room、event_log、auth 並持久化；供 db.DeleteAllEntities 使用。
func ClearAllEntities() error {
	if Default == nil {
		return nil
	}
	Default.mu.Lock()
	Default.Entities = make(map[string]*Entity)
	Default.EntityRooms = make(map[string]string)
	Default.EventLog = []EventEntry{}
	Default.Auth = make(map[string]string)
	Default.mu.Unlock()
	_ = Default.persistEntities()
	_ = Default.persistEntityRooms()
	_ = Default.persistEventLog()
	_ = Default.persistAuth()
	return nil
}

// GetItem 回傳物品定義；若無則 nil。
func (s *Store) GetItem(id string) *Item {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	it, ok := s.Items[id]
	if !ok || it == nil {
		return nil
	}
	cp := *it
	return &cp
}

// PutItem 新增或覆寫一筆物品並持久化。
func (s *Store) PutItem(it *Item) error {
	if s == nil || it == nil {
		return nil
	}
	s.mu.Lock()
	cp := *it
	s.Items[it.ID] = &cp
	s.mu.Unlock()
	return s.persistItems()
}

func (s *Store) persistItems() error {
	if s.itemsPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.itemsPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	var list []Item
	for _, it := range s.Items {
		if it != nil {
			list = append(list, *it)
		}
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(itemsFile{Items: list}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.itemsPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.itemsPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

// AppendEvent 追加一筆事件並持久化 event_log.json。
func (s *Store) AppendEvent(at int64, entityID, eventType, payload string) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	s.EventLog = append(s.EventLog, EventEntry{At: at, EntityID: entityID, EventType: eventType, Payload: payload})
	s.mu.Unlock()
	return s.persistEventLog()
}

// LastByEntity 回傳該實體在 at 之前最近一筆 eventType 的 payload；若無則空字串。
func (s *Store) LastByEntity(entityID, eventType string, at int64) string {
	if s == nil {
		return ""
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	for i := len(s.EventLog) - 1; i >= 0; i-- {
		e := s.EventLog[i]
		if e.EntityID == entityID && e.EventType == eventType && e.At <= at {
			return e.Payload
		}
	}
	return ""
}

// EventsInRange 回傳該實體在 [fromAt, toAt] 區間內的事件，依 at 升序。
func (s *Store) EventsInRange(entityID string, fromAt, toAt int64) []EventEntry {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []EventEntry
	for _, e := range s.EventLog {
		if e.EntityID == entityID && e.At >= fromAt && e.At <= toAt {
			out = append(out, e)
		}
	}
	return out
}

func (s *Store) persistEventLog() error {
	if s.eventLogPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.eventLogPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	entries := s.EventLog
	if entries == nil {
		entries = []EventEntry{}
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(eventLogFile{Entries: entries}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.eventLogPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.eventLogPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

// SetAuth 設定 entity 密碼雜湊並持久化 auth.json。
func (s *Store) SetAuth(entityID, passwordHash string) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	s.Auth[entityID] = passwordHash
	s.mu.Unlock()
	return s.persistAuth()
}

// GetAuth 回傳 entity 的密碼雜湊；若無則空字串。
func (s *Store) GetAuth(entityID string) string {
	if s == nil {
		return ""
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.Auth[entityID]
}

func (s *Store) persistAuth() error {
	if s.authPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.authPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	var entries []authEntry
	for eid, hash := range s.Auth {
		entries = append(entries, authEntry{EntityID: eid, PasswordHash: hash})
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(authFile{Entries: entries}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.authPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.authPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

