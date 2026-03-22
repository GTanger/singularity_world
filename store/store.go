// Package store 提供以 JSON 為唯一數據源的記憶體層（無 DB）：
// 啟動時從指定 JSON 檔與目錄載入全部資料，執行期只讀寫記憶體，必要時原子寫回對應 JSON。
// store.Default 初始化後，db 層的房間／實體／排班等皆由此讀寫。
package store

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"time"

	"singularity_world/model"
)

// entitiesWriteDebounce 實體寫回防抖間隔：此時間內多次更新只觸發一次寫入。
const entitiesWriteDebounce = 3 * time.Second

// Default 全域 store；Init 後 db 的房間／entity_room 會優先由此提供。
var Default *Store

var defaultMu sync.RWMutex

// Venue 場所：id、名稱、room_ids、max_staff（可選；0 表示用全局限額）。
type Venue struct {
	ID       string   `json:"id"`
	Name     string   `json:"name"`
	RoomIDs  []string `json:"room_ids"`
	MaxStaff int      `json:"max_staff,omitempty"`
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
	ID             string `json:"id"`
	Kind           string `json:"kind"`
	DisplayChar    string `json:"display_char"`
	X              int    `json:"x"`
	Y              int    `json:"y"`
	MoveState      string `json:"move_state"`
	TargetX        *int   `json:"target_x,omitempty"`
	TargetY        *int   `json:"target_y,omitempty"`
	WalkOrRun      string `json:"walk_or_run,omitempty"`
	MoveStartedAt  *int64 `json:"move_started_at,omitempty"`
	Vit            int    `json:"vit"`
	Qi             int    `json:"qi"`
	Dex            int    `json:"dex"`
	Magnesium      int    `json:"magnesium"`
	LastObservedAt *int64 `json:"last_observed_at,omitempty"`
	CreatedAt      int64  `json:"created_at"`
	Gender         string `json:"gender,omitempty"`
	SoulSeed       *int64 `json:"soul_seed,omitempty"`
	DisplayTitle   string `json:"display_title,omitempty"`
	ActivatedNodes string `json:"activated_nodes,omitempty"`
	EquipmentSlots string `json:"equipment_slots,omitempty"`
	Inventory      string `json:"inventory,omitempty"`
	Disposition    int    `json:"disposition,omitempty"` // 心境 -100~+100，0=中性
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

// ArchivalEntry NPC 長期記憶一條（依 entity_id 分區）。定義在 store 包，避免 store↔db 循環依賴。
type ArchivalEntry struct {
	EntityID  string `json:"entity_id"`
	Content   string `json:"content"`
	Tag       string `json:"tag"`
	CreatedAt int64  `json:"created_at"`
}

// NpcMemory NPC 短期記憶（10.18）：對某位玩家 subject 的見面次數與好感度。
type NpcMemory struct {
	EntityID     string `json:"entity_id"`  // NPC
	SubjectID    string `json:"subject_id"` // 玩家 entity_id
	MeetCount    int    `json:"meet_count"`
	Favorability int    `json:"favorability"` // -100～+100，0=普通
}

// NpcThread 為兩名 NPC 的短期話題線狀態（P1）。
type NpcThread struct {
	ThreadKey     string   `json:"thread_key"`
	TopicType     string   `json:"topic_type"`
	Phase         string   `json:"phase"` // opening / elaborate / cooling
	Anchors       []string `json:"anchors,omitempty"`
	TurnCount     int      `json:"turn_count"`
	CooldownUntil int64    `json:"cooldown_until"`
	UpdatedAt     int64    `json:"updated_at"`
}

// NpcDyad 為兩名 NPC 的關係狀態（P1 先提供儲存與讀取）。
type NpcDyad struct {
	AID         string   `json:"a_id"`
	BID         string   `json:"b_id"`
	Familiarity int      `json:"familiarity"`
	Sentiment   int      `json:"sentiment"`
	Tags        []string `json:"tags,omitempty"`
	UpdatedAt   int64    `json:"updated_at"`
}

// NpcRumor 為可衰減的社會傳聞（P4）。
type NpcRumor struct {
	ID                string `json:"id"`
	Text              string `json:"text"`
	RoomID            string `json:"room_id,omitempty"`
	Zone              string `json:"zone,omitempty"`
	Source            string `json:"source,omitempty"`       // dialogue_anchor / room_event / economy / spawn / job
	SourceScore       int    `json:"source_score,omitempty"` // 來源基礎權重
	Weight            int    `json:"weight"`
	MentionCount      int    `json:"mention_count,omitempty"` // 被挑入 context 次數（P8）
	LastUsedAt        int64  `json:"last_used_at,omitempty"`  // 最近被挑入 context 時間（P8）
	BlockedUntil      int64  `json:"blocked_until,omitempty"` // 因衝突暫時封鎖到何時（P9）
	PenaltyCount      int    `json:"penalty_count,omitempty"` // 反事實懲罰次數（P10）
	LastPenaltyAt     int64  `json:"last_penalty_at,omitempty"`
	LastPenaltyReason string `json:"last_penalty_reason,omitempty"`
	UpdatedAt         int64  `json:"updated_at"`
	ExpiresAt         int64  `json:"expires_at"`
}

// NpcRumorDigest 為傳聞池批次摘要（P6）。
type NpcRumorDigest struct {
	Text        string `json:"text"`
	SourceCount int    `json:"source_count"`
	UpdatedAt   int64  `json:"updated_at"`
}

// Store 記憶體中的房間、出口、實體、場所、指派、排班、物品、事件日誌、密碼。
type Store struct {
	mu                  sync.RWMutex
	Rooms               map[string]*model.Room  // id -> Room
	Exits               map[string][]model.Exit // from_room_id -> 出口列表
	EntityRooms         map[string]string       // entity_id -> room_id
	Venues              map[string]*Venue       // id -> Venue
	Assignments         map[string][]Assignment // entity_id -> 指派列表
	Schedules           map[string]*Schedule    // entity_id -> 排班
	Entities            map[string]*Entity      // id -> Entity
	Items               map[string]*Item        // id -> Item
	EventLog            []EventEntry            // 事件日誌（append）
	Archival            []ArchivalEntry         // NPC 長期記憶（依 entity_id 查詢）
	Auth                map[string]string       // entity_id -> password_hash
	NpcSummaries        map[string]string       // entity_id -> 與玩家的最近印象（背版用）
	NpcNpcSummaries     map[string]string       // "idA|idB"（idA<idB）-> A 與 B 的最近交談摘要，供長碰面不重複
	NpcMemories         map[string]*NpcMemory   // "entity_id|subject_id" -> 短期記憶（見面次數、好感度）
	NpcThreads          map[string]*NpcThread   // "idA|idB" -> NPC 對話 thread（P1）
	NpcDyads            map[string]*NpcDyad     // "idA|idB" -> NPC 關係（P1）
	NpcRumors           map[string]*NpcRumor    // rumor_id -> 傳聞（P4）
	NpcRumorDigest      *NpcRumorDigest         // 傳聞摘要（P6）
	runtimeDir          string
	summariesPath       string
	npcNpcSummariesPath string
	npcThreadPath       string
	npcDyadPath         string
	npcRumorPath        string
	npcRumorDigestPath  string
	archivalPath        string
	npcMemoryPath       string
	entityRoomsPath     string
	assignmentsPath     string
	schedulesPath       string
	venuesPath          string
	entitiesPath        string
	itemsPath           string
	eventLogPath        string
	authPath            string
	roomsPath           string // 房間來源：目錄 data/rooms 或單檔
	// entities 寫回防抖：避免每筆更新就寫整份 JSON
	entitiesDirty   bool
	entitiesTimer   *time.Timer
	entitiesTimerMu sync.Mutex
}

// roomsFile 單檔格式（舊版 data/rooms.json：rooms + exits 陣列）。
type roomsFile struct {
	Rooms []roomDef `json:"rooms"`
	Exits []exitDef `json:"exits"`
}

// roomFileOne 一房一檔格式（data/rooms/<id>.json）：單一房間含其出口與可互動物件。
type roomFileOne struct {
	ID          string             `json:"id"`
	Name        string             `json:"name"`
	Description string             `json:"description"`
	Tags        []string           `json:"tags"`
	Zone        string             `json:"zone"`
	Exits       []exitOut          `json:"exits"`
	Objects     []model.RoomObject `json:"objects,omitempty"`
}
type exitOut struct {
	Direction string `json:"direction"`
	To        string `json:"to"`
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
		Rooms:               make(map[string]*model.Room),
		Exits:               make(map[string][]model.Exit),
		EntityRooms:         make(map[string]string),
		Venues:              make(map[string]*Venue),
		Assignments:         make(map[string][]Assignment),
		Schedules:           make(map[string]*Schedule),
		Entities:            make(map[string]*Entity),
		Items:               make(map[string]*Item),
		EventLog:            nil,
		Auth:                make(map[string]string),
		runtimeDir:          runtimeDir,
		entityRoomsPath:     filepath.Join(runtimeDir, "entity_rooms.json"),
		assignmentsPath:     filepath.Join(dataDir, "assignments.json"),
		schedulesPath:       filepath.Join(dataDir, "schedules.json"),
		venuesPath:          filepath.Join(dataDir, "venues.json"),
		entitiesPath:        filepath.Join(dataDir, "entities.json"),
		itemsPath:           filepath.Join(dataDir, "items.json"),
		eventLogPath:        filepath.Join(runtimeDir, "event_log.json"),
		archivalPath:        filepath.Join(runtimeDir, "npc_archival.json"),
		npcMemoryPath:       filepath.Join(runtimeDir, "npc_memory.json"),
		authPath:            filepath.Join(runtimeDir, "auth.json"),
		summariesPath:       filepath.Join(runtimeDir, "npc_summaries.json"),
		npcNpcSummariesPath: filepath.Join(runtimeDir, "npc_npc_summaries.json"),
		npcThreadPath:       filepath.Join(runtimeDir, "npc_thread.json"),
		npcDyadPath:         filepath.Join(runtimeDir, "npc_dyad.json"),
		npcRumorPath:        filepath.Join(runtimeDir, "npc_rumors.json"),
		npcRumorDigestPath:  filepath.Join(runtimeDir, "npc_rumor_digest.json"),
		NpcSummaries:        make(map[string]string),
		NpcNpcSummaries:     make(map[string]string),
		NpcMemories:         make(map[string]*NpcMemory),
		NpcThreads:          make(map[string]*NpcThread),
		NpcDyads:            make(map[string]*NpcDyad),
		NpcRumors:           make(map[string]*NpcRumor),
		NpcRumorDigest:      nil,
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
		_ = s.loadArchival()
		_ = s.loadNpcMemory()
		_ = s.loadAuth()
		_ = s.loadSummaries()
		_ = s.loadNpcNpcSummaries()
		_ = s.loadNpcThreads()
		_ = s.loadNpcDyads()
		_ = s.loadNpcRumors()
		_ = s.loadNpcRumorDigest()
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
	// 需求：僅保留街道/巷道房間，移除其連接建築室內空間。
	s.pruneNonStreetRoomsLocked(nil)
	return nil
}

// loadRoomsFromDir 遞迴掃描 dir 下所有 .json（含任意層子資料夾，例 data/rooms/editor/），每檔一房＋其 exits。
// data/rooms/editor/ 子目錄下的房間豁免街道篩選，全數保留。
func (s *Store) loadRoomsFromDir(dir string) error {
	type fileEntry struct {
		room     roomFileOne
		isEditor bool
	}
	var list []fileEntry
	err := filepath.WalkDir(dir, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || filepath.Ext(d.Name()) != ".json" {
			return nil
		}
		if strings.HasPrefix(strings.TrimSuffix(d.Name(), filepath.Ext(d.Name())), "_") {
			return nil
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		var one roomFileOne
		if json.Unmarshal(data, &one) != nil {
			return nil
		}
		rel, _ := filepath.Rel(dir, path)
		isEditor := strings.HasPrefix(filepath.ToSlash(rel), "editor/")
		list = append(list, fileEntry{room: one, isEditor: isEditor})
		return nil
	})
	if err != nil {
		return err
	}
	editorIDs := make(map[string]bool)
	s.mu.Lock()
	defer s.mu.Unlock()
	nameByID := make(map[string]string)
	for i := range list {
		one := &list[i].room
		s.Rooms[one.ID] = &model.Room{
			ID:          one.ID,
			Name:        one.Name,
			Tags:        one.Tags,
			Zone:        one.Zone,
			Description: one.Description,
			Objects:     one.Objects,
		}
		nameByID[one.ID] = one.Name
		if list[i].isEditor && one.ID != "" {
			editorIDs[one.ID] = true
		}
	}
	for i := range list {
		one := &list[i].room
		for _, ex := range one.Exits {
			toName := nameByID[ex.To]
			s.Exits[one.ID] = append(s.Exits[one.ID], model.Exit{
				Direction:  ex.Direction,
				ToRoomID:   ex.To,
				ToRoomName: toName,
			})
		}
	}
	// 需求：僅保留街道/巷道房間，移除其連接建築室內空間；editor/ 子目錄的房間全數豁免。
	s.pruneNonStreetRoomsLocked(editorIDs)
	return nil
}

func isStreetOrAlleyRoom(r *model.Room) bool {
	if r == nil {
		return false
	}
	if strings.Contains(r.Name, "大街") || strings.Contains(r.Name, "巷") {
		return true
	}
	for _, t := range r.Tags {
		lt := strings.ToLower(strings.TrimSpace(t))
		if lt == "street" || lt == "alley" {
			return true
		}
	}
	return false
}

// pruneNonStreetRoomsLocked 僅保留街道/巷道房間，並移除指向刪除房間的出口。
// exemptIDs 中的房間不參與篩選（如 editor/ 子目錄的手工房間）。
// 呼叫端需持有 s.mu.Lock()。
func (s *Store) pruneNonStreetRoomsLocked(exemptIDs map[string]bool) {
	keep := make(map[string]bool, len(s.Rooms))
	for id, r := range s.Rooms {
		if exemptIDs[id] || isStreetOrAlleyRoom(r) {
			keep[id] = true
		}
	}
	for id := range s.Rooms {
		if !keep[id] {
			delete(s.Rooms, id)
			delete(s.Exits, id)
		}
	}
	for from, exits := range s.Exits {
		if !keep[from] {
			delete(s.Exits, from)
			continue
		}
		var filtered []model.Exit
		for _, ex := range exits {
			if keep[ex.ToRoomID] {
				filtered = append(filtered, ex)
			}
		}
		s.Exits[from] = filtered
	}
}

// firstStreetOrAlleyRoomIDLocked 回傳目前保留房間中的第一個 id（排序後），供遷移失效 entity_room 使用。
// 呼叫端需持有 s.mu.RLock()/s.mu.Lock()。
func (s *Store) firstStreetOrAlleyRoomIDLocked() string {
	if len(s.Rooms) == 0 {
		return ""
	}
	ids := make([]string, 0, len(s.Rooms))
	for id := range s.Rooms {
		ids = append(ids, id)
	}
	sort.Strings(ids)
	return ids[0]
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
	fallbackRoomID := s.firstStreetOrAlleyRoomIDLocked()
	for _, e := range f.Entries {
		if _, ok := s.Rooms[e.RoomID]; ok {
			s.EntityRooms[e.EntityID] = e.RoomID
		} else if fallbackRoomID != "" {
			s.EntityRooms[e.EntityID] = fallbackRoomID
		}
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

// UpsertRoomData 更新或新增單一房間與其出口（供編輯器即時同步記憶體）。
func (s *Store) UpsertRoomData(room *model.Room, exits []model.Exit) {
	if s == nil || room == nil || room.ID == "" {
		return
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	cp := *room
	s.Rooms[room.ID] = &cp
	out := make([]model.Exit, 0, len(exits))
	for _, ex := range exits {
		if ex.Direction == "" || ex.ToRoomID == "" {
			continue
		}
		ec := ex
		if ec.ToRoomName == "" {
			if tr, ok := s.Rooms[ec.ToRoomID]; ok && tr != nil {
				ec.ToRoomName = tr.Name
			}
		}
		out = append(out, ec)
	}
	s.Exits[room.ID] = out
}

// DeleteRoomData 刪除單一房間並移除指向它的入口（供編輯器即時同步記憶體）。
func (s *Store) DeleteRoomData(roomID string) {
	if s == nil || roomID == "" {
		return
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.Rooms, roomID)
	delete(s.Exits, roomID)
	for from, list := range s.Exits {
		filtered := make([]model.Exit, 0, len(list))
		for _, ex := range list {
			if ex.ToRoomID == roomID {
				continue
			}
			filtered = append(filtered, ex)
		}
		s.Exits[from] = filtered
	}
	// 房間被刪時，將該房間內實體遷回第一個可用房間，避免懸空。
	fallback := s.firstStreetOrAlleyRoomIDLocked()
	for eid, rid := range s.EntityRooms {
		if rid == roomID {
			if fallback != "" {
				s.EntityRooms[eid] = fallback
			} else {
				delete(s.EntityRooms, eid)
			}
		}
	}
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

// GetRoomIDByName 依房間名稱回傳第一個符合的房間 id；若無則空字串。創生預設格以名稱「界壁」唯一識別。
func (s *Store) GetRoomIDByName(name string) string {
	if s == nil || name == "" {
		return ""
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	for id, r := range s.Rooms {
		if r.Name == name {
			return id
		}
	}
	return ""
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

// GetNPCIDsWithRoom 回傳所有有房間且 Vit>0（存活）的 NPC 實體 ID（供主迴圈註冊腦驅動用）。死亡（Vit≤0）NPC 不參與。
func (s *Store) GetNPCIDsWithRoom() []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []string
	for eid := range s.EntityRooms {
		e, ok := s.Entities[eid]
		if !ok || e == nil || e.Kind != "npc" || e.Vit <= 0 {
			continue
		}
		out = append(out, eid)
	}
	return out
}

// GetPlayerIDsWithRoom 回傳所有有房間的玩家實體 ID（池總量計入玩家時使用）。
func (s *Store) GetPlayerIDsWithRoom() []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []string
	for eid := range s.EntityRooms {
		e, ok := s.Entities[eid]
		if !ok || e == nil || e.Kind != "player" {
			continue
		}
		out = append(out, eid)
	}
	return out
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

// AllEntityIDs 回傳目前載入之所有實體 ID（供 SyncRoomsFromFile 等補齊 entity_room 用）。
func (s *Store) AllEntityIDs() []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make([]string, 0, len(s.Entities))
	for id := range s.Entities {
		out = append(out, id)
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

// GetVenueIDsForRoom 回傳包含該房間的所有場所 ID，供求職撮合用。
func (s *Store) GetVenueIDsForRoom(roomID string) []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []string
	for vid, v := range s.Venues {
		if v == nil {
			continue
		}
		for _, rid := range v.RoomIDs {
			if rid == roomID {
				out = append(out, vid)
				break
			}
		}
	}
	return out
}

// GetVenueMaxStaff 回傳該場所職缺上限；0 或未設表示用全局限額 defaultMax。
func (s *Store) GetVenueMaxStaff(venueID string, defaultMax int) int {
	if s == nil {
		return defaultMax
	}
	s.mu.RLock()
	v := s.Venues[venueID]
	s.mu.RUnlock()
	if v == nil || v.MaxStaff <= 0 {
		return defaultMax
	}
	return v.MaxStaff
}

// GetFirstOccupationIDForVenue 回傳該場所既有指派中的第一個職業 ID（10.15 撮合用）；無則空字串。
func (s *Store) GetFirstOccupationIDForVenue(venueID string) string {
	if s == nil {
		return ""
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	for _, list := range s.Assignments {
		for _, a := range list {
			if a.VenueID == venueID {
				return a.OccupationID
			}
		}
	}
	return ""
}

// GetAssignmentCountByVenue 回傳該場所目前的指派數量（與 max_staff 比對可判斷是否有缺）。
func (s *Store) GetAssignmentCountByVenue(venueID string) int {
	if s == nil {
		return 0
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	n := 0
	for _, list := range s.Assignments {
		for _, a := range list {
			if a.VenueID == venueID {
				n++
			}
		}
	}
	return n
}

// GetAllVenueIDs 回傳所有場所 ID（供未觀測背景模擬隨機求職撮合用）。
func (s *Store) GetAllVenueIDs() []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make([]string, 0, len(s.Venues))
	for id := range s.Venues {
		out = append(out, id)
	}
	return out
}

// GetAllVenueRoomIDs 回傳所有場所涵蓋的房間 ID（去重），供決策引擎 SeekJob 意圖尋路用。
func (s *Store) GetAllVenueRoomIDs() []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	seen := make(map[string]bool)
	var out []string
	for _, v := range s.Venues {
		if v == nil {
			continue
		}
		for _, rid := range v.RoomIDs {
			if !seen[rid] {
				seen[rid] = true
				out = append(out, rid)
			}
		}
	}
	return out
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

// RemoveAssignmentsForEntity 移除某實體的全部指派（死亡除名用）。會寫回 assignments.json。
func (s *Store) RemoveAssignmentsForEntity(entityID string) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	delete(s.Assignments, entityID)
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

// RemoveScheduleForEntity 移除某實體的排班（死亡除名用）。會寫回 schedules.json。
func (s *Store) RemoveScheduleForEntity(entityID string) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	delete(s.Schedules, entityID)
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

// NPCIDsWithMissingSoulSeed 回傳所有 kind=npc 且 SoulSeed 為 nil 的實體 ID，供啟動時補寫 soul_seed 用。
func (s *Store) NPCIDsWithMissingSoulSeed() []string {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []string
	for id, e := range s.Entities {
		if e != nil && e.Kind == "npc" && e.SoulSeed == nil {
			out = append(out, id)
		}
	}
	return out
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

// PutEntity 新增或覆寫一筆實體，並排程防抖寫回 entities.json（見 entitiesWriteDebounce）。
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
	s.scheduleEntitiesPersist()
	return nil
}

// TransferMagnesiumBetween 在同一鎖內將 amount 鎂自 fromID 轉至 toID；餘額不足或實體不存在則回錯。
// 對齊經濟／物流規格：鎂轉手原子性（第一版 store 路徑單一臨界區）。
func (s *Store) TransferMagnesiumBetween(fromID, toID string, amount int) error {
	if s == nil {
		return fmt.Errorf("store nil")
	}
	if amount <= 0 {
		return fmt.Errorf("轉帳鎂數須為正")
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	fe, ok1 := s.Entities[fromID]
	te, ok2 := s.Entities[toID]
	if !ok1 || !ok2 || fe == nil || te == nil {
		return fmt.Errorf("找不到轉出或轉入實體")
	}
	if fe.Magnesium < amount {
		return fmt.Errorf("鎂不足")
	}
	fc := *fe
	tc := *te
	fc.Magnesium -= amount
	tc.Magnesium += amount
	s.Entities[fromID] = &fc
	s.Entities[toID] = &tc
	s.scheduleEntitiesPersist()
	return nil
}

// UpdateEntity 對指定實體執行 fn，並排程防抖寫回 entities.json。若實體不存在則不寫入。
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
	s.scheduleEntitiesPersist()
	return nil
}

// scheduleEntitiesPersist 標記 entities 為髒並在 debounce 後寫回；若已排程則只重設計時器。
func (s *Store) scheduleEntitiesPersist() {
	if s == nil || s.entitiesPath == "" {
		return
	}
	s.entitiesTimerMu.Lock()
	defer s.entitiesTimerMu.Unlock()
	s.entitiesDirty = true
	if s.entitiesTimer == nil {
		s.entitiesTimer = time.AfterFunc(entitiesWriteDebounce, s.entitiesPersistFired)
	} else {
		s.entitiesTimer.Reset(entitiesWriteDebounce)
	}
}

// entitiesPersistFired 由防抖計時器呼叫：執行一次 entities 寫回並清除排程。
func (s *Store) entitiesPersistFired() {
	s.entitiesTimerMu.Lock()
	s.entitiesDirty = false
	if s.entitiesTimer != nil {
		s.entitiesTimer.Stop()
		s.entitiesTimer = nil
	}
	s.entitiesTimerMu.Unlock()
	if err := s.persistEntities(); err != nil {
		log.Printf("[store] persist entities: %v", err)
	}
}

// FlushEntities 立即寫回 entities.json 並取消尚未執行的防抖排程。關機前應呼叫以確保不遺失未寫入的變更。
func (s *Store) FlushEntities() error {
	if s == nil {
		return nil
	}
	s.entitiesTimerMu.Lock()
	if s.entitiesTimer != nil {
		s.entitiesTimer.Stop()
		s.entitiesTimer = nil
	}
	s.entitiesDirty = false
	s.entitiesTimerMu.Unlock()
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

// RecentByEntity 取得某 entity 最近 n 筆事件（由新到舊），供 NPC 背版／Talk 引用。
func (s *Store) RecentByEntity(entityID string, n int) []EventEntry {
	if s == nil || n <= 0 {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []EventEntry
	for i := len(s.EventLog) - 1; i >= 0 && len(out) < n; i-- {
		if s.EventLog[i].EntityID == entityID {
			out = append(out, s.EventLog[i])
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

type archivalFile struct {
	Entries []ArchivalEntry `json:"entries"`
}

func (s *Store) loadArchival() error {
	if s.archivalPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.archivalPath)
	if err != nil {
		if os.IsNotExist(err) {
			s.Archival = []ArchivalEntry{}
			return nil
		}
		return err
	}
	var f archivalFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	if f.Entries == nil {
		f.Entries = []ArchivalEntry{}
	}
	s.mu.Lock()
	s.Archival = f.Entries
	s.mu.Unlock()
	return nil
}

func (s *Store) persistArchival() error {
	if s.archivalPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.archivalPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	entries := s.Archival
	if entries == nil {
		entries = []ArchivalEntry{}
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(archivalFile{Entries: entries}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.archivalPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.archivalPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

// ArchivalMaxPerEntity 單一 NPC 最多保留條數，超過則刪最舊（與 db.ArchivalMaxPerEntity 同步）。
const ArchivalMaxPerEntity = 100

// AppendArchival 追加一筆 NPC 長期記憶並持久化；若任一新舊條數超過 ArchivalMaxPerEntity 會先修剪再寫回。
func (s *Store) AppendArchival(entry ArchivalEntry) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	s.Archival = append(s.Archival, entry)
	s.trimArchivalPerEntity(ArchivalMaxPerEntity)
	s.mu.Unlock()
	return s.persistArchival()
}

// trimArchivalPerEntity 保留每 entity 最多 max 條（取最新），其餘從 s.Archival 移除；呼叫方需持有 mu。
func (s *Store) trimArchivalPerEntity(max int) {
	if max <= 0 || len(s.Archival) == 0 {
		return
	}
	byEntity := make(map[string][]ArchivalEntry)
	for _, e := range s.Archival {
		byEntity[e.EntityID] = append(byEntity[e.EntityID], e)
	}
	var out []ArchivalEntry
	for _, entries := range byEntity {
		if len(entries) <= max {
			out = append(out, entries...)
			continue
		}
		sort.Slice(entries, func(i, j int) bool { return entries[i].CreatedAt > entries[j].CreatedAt })
		out = append(out, entries[:max]...)
	}
	s.Archival = out
}

// GetArchivalByEntity 回傳該 entity 的全部記憶條目（由新到舊）。
func (s *Store) GetArchivalByEntity(entityID string) []ArchivalEntry {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	var out []ArchivalEntry
	for i := len(s.Archival) - 1; i >= 0; i-- {
		if s.Archival[i].EntityID == entityID {
			out = append(out, s.Archival[i])
		}
	}
	return out
}

func npcMemoryKey(entityID, subjectID string) string { return entityID + "|" + subjectID }

// GetNpcMemory 回傳該 NPC 對某玩家的短期記憶；無則 nil。
func (s *Store) GetNpcMemory(entityID, subjectID string) *NpcMemory {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.NpcMemories[npcMemoryKey(entityID, subjectID)]
}

// RecordMeet 記錄一次見面：若無則建立 (meet_count=1, favorability=0)，若有則 meet_count+1。會持久化。
func (s *Store) RecordMeet(entityID, subjectID string) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	k := npcMemoryKey(entityID, subjectID)
	if s.NpcMemories[k] == nil {
		s.NpcMemories[k] = &NpcMemory{EntityID: entityID, SubjectID: subjectID, MeetCount: 1, Favorability: 0}
	} else {
		s.NpcMemories[k].MeetCount++
	}
	s.mu.Unlock()
	return s.persistNpcMemory()
}

// AdjustFavorability 調整該 NPC 對某玩家的好感度，clamp [-100, +100]。會持久化。
func (s *Store) AdjustFavorability(entityID, subjectID string, delta int) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	k := npcMemoryKey(entityID, subjectID)
	if s.NpcMemories[k] == nil {
		s.NpcMemories[k] = &NpcMemory{EntityID: entityID, SubjectID: subjectID, MeetCount: 0, Favorability: 0}
	}
	m := s.NpcMemories[k]
	m.Favorability += delta
	if m.Favorability > 100 {
		m.Favorability = 100
	}
	if m.Favorability < -100 {
		m.Favorability = -100
	}
	s.mu.Unlock()
	return s.persistNpcMemory()
}

type npcMemoryFile struct {
	Entries []NpcMemory `json:"entries"`
}

func (s *Store) loadNpcMemory() error {
	if s.npcMemoryPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.npcMemoryPath)
	if err != nil {
		if os.IsNotExist(err) {
			s.NpcMemories = make(map[string]*NpcMemory)
			return nil
		}
		return err
	}
	var f npcMemoryFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	if s.NpcMemories == nil {
		s.NpcMemories = make(map[string]*NpcMemory)
	}
	for i := range f.Entries {
		e := &f.Entries[i]
		s.NpcMemories[npcMemoryKey(e.EntityID, e.SubjectID)] = e
	}
	s.mu.Unlock()
	return nil
}

func (s *Store) persistNpcMemory() error {
	if s.npcMemoryPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.npcMemoryPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	var entries []NpcMemory
	for _, m := range s.NpcMemories {
		entries = append(entries, *m)
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(npcMemoryFile{Entries: entries}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.npcMemoryPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.npcMemoryPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

type summariesFile struct {
	Summaries map[string]string `json:"summaries"`
}

func (s *Store) loadSummaries() error {
	if s == nil || s.summariesPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.summariesPath)
	if err != nil {
		if os.IsNotExist(err) {
			s.mu.Lock()
			if s.NpcSummaries == nil {
				s.NpcSummaries = make(map[string]string)
			}
			s.mu.Unlock()
			return nil
		}
		return err
	}
	var f summariesFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	if f.Summaries != nil {
		s.NpcSummaries = f.Summaries
	} else if s.NpcSummaries == nil {
		s.NpcSummaries = make(map[string]string)
	}
	s.mu.Unlock()
	return nil
}

func (s *Store) persistSummaries() error {
	if s == nil || s.summariesPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.summariesPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	summaries := s.NpcSummaries
	if summaries == nil {
		summaries = make(map[string]string)
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(summariesFile{Summaries: summaries}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.summariesPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.summariesPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

// GetNpcSummary 回傳該 NPC 的「與玩家最近印象」；背版用，空則無。
func (s *Store) GetNpcSummary(entityID string) string {
	if s == nil || s.NpcSummaries == nil {
		return ""
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.NpcSummaries[entityID]
}

// SetNpcSummary 設定該 NPC 的最近印象並持久化。
func (s *Store) SetNpcSummary(entityID, summary string) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	if s.NpcSummaries == nil {
		s.NpcSummaries = make(map[string]string)
	}
	s.NpcSummaries[entityID] = summary
	s.mu.Unlock()
	return s.persistSummaries()
}

// npcNpcSummariesFile 持久化格式。
type npcNpcSummariesFile struct {
	Summaries map[string]string `json:"summaries"`
}

func (s *Store) loadNpcNpcSummaries() error {
	if s == nil || s.npcNpcSummariesPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.npcNpcSummariesPath)
	if err != nil {
		if os.IsNotExist(err) {
			s.mu.Lock()
			if s.NpcNpcSummaries == nil {
				s.NpcNpcSummaries = make(map[string]string)
			}
			s.mu.Unlock()
			return nil
		}
		return err
	}
	var f npcNpcSummariesFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	if f.Summaries != nil {
		s.NpcNpcSummaries = f.Summaries
	} else if s.NpcNpcSummaries == nil {
		s.NpcNpcSummaries = make(map[string]string)
	}
	s.mu.Unlock()
	return nil
}

func (s *Store) persistNpcNpcSummaries() error {
	if s == nil || s.npcNpcSummariesPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.npcNpcSummariesPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	summaries := s.NpcNpcSummaries
	if summaries == nil {
		summaries = make(map[string]string)
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(npcNpcSummariesFile{Summaries: summaries}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.npcNpcSummariesPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.npcNpcSummariesPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

// canonicalNpcNpcKey 回傳 idA、idB 的標準鍵（idA <= idB 字典序）。
func canonicalNpcNpcKey(idA, idB string) string {
	if idA > idB {
		return idB + "|" + idA
	}
	return idA + "|" + idB
}

// GetNpcNpcSummary 回傳 A 與 B 的最近交談摘要，供下次觸發時帶入 context、避免重複。
func (s *Store) GetNpcNpcSummary(idA, idB string) string {
	if s == nil || s.NpcNpcSummaries == nil {
		return ""
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.NpcNpcSummaries[canonicalNpcNpcKey(idA, idB)]
}

// SetNpcNpcSummary 寫入 A 與 B 的最近交談摘要並持久化。
func (s *Store) SetNpcNpcSummary(idA, idB, summary string) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	if s.NpcNpcSummaries == nil {
		s.NpcNpcSummaries = make(map[string]string)
	}
	s.NpcNpcSummaries[canonicalNpcNpcKey(idA, idB)] = summary
	s.mu.Unlock()
	return s.persistNpcNpcSummaries()
}

type npcThreadFile struct {
	Threads map[string]*NpcThread `json:"threads"`
}

func (s *Store) loadNpcThreads() error {
	if s == nil || s.npcThreadPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.npcThreadPath)
	if err != nil {
		if os.IsNotExist(err) {
			s.mu.Lock()
			if s.NpcThreads == nil {
				s.NpcThreads = make(map[string]*NpcThread)
			}
			s.mu.Unlock()
			return nil
		}
		return err
	}
	var f npcThreadFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	if f.Threads != nil {
		s.NpcThreads = f.Threads
	} else if s.NpcThreads == nil {
		s.NpcThreads = make(map[string]*NpcThread)
	}
	s.mu.Unlock()
	return nil
}

func (s *Store) persistNpcThreads() error {
	if s == nil || s.npcThreadPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.npcThreadPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	threads := s.NpcThreads
	if threads == nil {
		threads = make(map[string]*NpcThread)
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(npcThreadFile{Threads: threads}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.npcThreadPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.npcThreadPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

// GetNpcThread 回傳該 pair 的 thread 狀態（拷貝）；若不存在回傳 nil。
func (s *Store) GetNpcThread(idA, idB string) *NpcThread {
	if s == nil {
		return nil
	}
	key := canonicalNpcNpcKey(idA, idB)
	s.mu.RLock()
	defer s.mu.RUnlock()
	t := s.NpcThreads[key]
	if t == nil {
		return nil
	}
	cp := *t
	if t.Anchors != nil {
		cp.Anchors = append([]string(nil), t.Anchors...)
	}
	return &cp
}

// SetNpcThread 寫入 pair 的 thread 狀態並持久化。
func (s *Store) SetNpcThread(idA, idB string, t *NpcThread) error {
	if s == nil || t == nil {
		return nil
	}
	key := canonicalNpcNpcKey(idA, idB)
	cp := *t
	cp.ThreadKey = key
	if cp.Anchors != nil {
		cp.Anchors = append([]string(nil), cp.Anchors...)
	}
	s.mu.Lock()
	if s.NpcThreads == nil {
		s.NpcThreads = make(map[string]*NpcThread)
	}
	s.NpcThreads[key] = &cp
	s.mu.Unlock()
	return s.persistNpcThreads()
}

// DeleteNpcThread 刪除 pair 的 thread 並持久化。
func (s *Store) DeleteNpcThread(idA, idB string) error {
	if s == nil {
		return nil
	}
	key := canonicalNpcNpcKey(idA, idB)
	s.mu.Lock()
	if s.NpcThreads != nil {
		delete(s.NpcThreads, key)
	}
	s.mu.Unlock()
	return s.persistNpcThreads()
}

type npcDyadFile struct {
	Dyads map[string]*NpcDyad `json:"dyads"`
}

func (s *Store) loadNpcDyads() error {
	if s == nil || s.npcDyadPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.npcDyadPath)
	if err != nil {
		if os.IsNotExist(err) {
			s.mu.Lock()
			if s.NpcDyads == nil {
				s.NpcDyads = make(map[string]*NpcDyad)
			}
			s.mu.Unlock()
			return nil
		}
		return err
	}
	var f npcDyadFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	if f.Dyads != nil {
		s.NpcDyads = f.Dyads
	} else if s.NpcDyads == nil {
		s.NpcDyads = make(map[string]*NpcDyad)
	}
	s.mu.Unlock()
	return nil
}

func (s *Store) persistNpcDyads() error {
	if s == nil || s.npcDyadPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.npcDyadPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	dyads := s.NpcDyads
	if dyads == nil {
		dyads = make(map[string]*NpcDyad)
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(npcDyadFile{Dyads: dyads}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.npcDyadPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.npcDyadPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

// GetNpcDyad 回傳 pair 的關係狀態（拷貝）；若不存在則建立預設值回傳。
func (s *Store) GetNpcDyad(idA, idB string) *NpcDyad {
	if s == nil {
		return nil
	}
	key := canonicalNpcNpcKey(idA, idB)
	aID, bID := idA, idB
	if aID > bID {
		aID, bID = bID, aID
	}
	s.mu.RLock()
	d := s.NpcDyads[key]
	s.mu.RUnlock()
	if d != nil {
		cp := *d
		if d.Tags != nil {
			cp.Tags = append([]string(nil), d.Tags...)
		}
		return &cp
	}
	return &NpcDyad{AID: aID, BID: bID, Familiarity: 0, Sentiment: 0}
}

// SetNpcDyad 寫入 pair 的關係狀態並持久化。
func (s *Store) SetNpcDyad(idA, idB string, d *NpcDyad) error {
	if s == nil || d == nil {
		return nil
	}
	key := canonicalNpcNpcKey(idA, idB)
	aID, bID := idA, idB
	if aID > bID {
		aID, bID = bID, aID
	}
	cp := *d
	cp.AID = aID
	cp.BID = bID
	if cp.Tags != nil {
		cp.Tags = append([]string(nil), cp.Tags...)
	}
	s.mu.Lock()
	if s.NpcDyads == nil {
		s.NpcDyads = make(map[string]*NpcDyad)
	}
	s.NpcDyads[key] = &cp
	s.mu.Unlock()
	return s.persistNpcDyads()
}

type npcRumorFile struct {
	Rumors map[string]*NpcRumor `json:"rumors"`
}

func (s *Store) loadNpcRumors() error {
	if s == nil || s.npcRumorPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.npcRumorPath)
	if err != nil {
		if os.IsNotExist(err) {
			s.mu.Lock()
			if s.NpcRumors == nil {
				s.NpcRumors = make(map[string]*NpcRumor)
			}
			s.mu.Unlock()
			return nil
		}
		return err
	}
	var f npcRumorFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	if f.Rumors != nil {
		s.NpcRumors = f.Rumors
	} else if s.NpcRumors == nil {
		s.NpcRumors = make(map[string]*NpcRumor)
	}
	s.mu.Unlock()
	return nil
}

func (s *Store) persistNpcRumors() error {
	if s == nil || s.npcRumorPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.npcRumorPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	rumors := s.NpcRumors
	if rumors == nil {
		rumors = make(map[string]*NpcRumor)
	}
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(npcRumorFile{Rumors: rumors}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.npcRumorPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.npcRumorPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

// UpsertNpcRumor 寫入/更新一條傳聞；同 ID 會疊加權重並刷新時間。
func (s *Store) UpsertNpcRumor(r *NpcRumor) error {
	if s == nil || r == nil {
		return nil
	}
	id := strings.TrimSpace(r.ID)
	if id == "" {
		return nil
	}
	now := r.UpdatedAt
	if now <= 0 {
		now = time.Now().Unix()
	}
	incomingKey := canonicalRumorText(r.Text)
	s.mu.Lock()
	if s.NpcRumors == nil {
		s.NpcRumors = make(map[string]*NpcRumor)
	}
	// P7：同文本冷卻去重（10 分鐘）。若近期已有同文本，直接疊加到既有條目，避免洗版。
	if incomingKey != "" {
		for oldID, old := range s.NpcRumors {
			if old == nil {
				continue
			}
			if canonicalRumorText(old.Text) != incomingKey {
				continue
			}
			if old.UpdatedAt > 0 && now-old.UpdatedAt <= 600 {
				if r.Source != "" {
					old.Source = r.Source
				}
				if r.SourceScore > 0 {
					old.SourceScore = r.SourceScore
				}
				if r.RoomID != "" {
					old.RoomID = r.RoomID
				}
				if r.Zone != "" {
					old.Zone = r.Zone
				}
				old.Weight += max(1, r.Weight)
				old.UpdatedAt = now
				if r.ExpiresAt > old.ExpiresAt {
					old.ExpiresAt = r.ExpiresAt
				}
				s.NpcRumors[oldID] = old
				s.mu.Unlock()
				return s.persistNpcRumors()
			}
		}
	}
	if old := s.NpcRumors[id]; old != nil {
		if strings.TrimSpace(r.Text) != "" {
			old.Text = r.Text
		}
		if r.RoomID != "" {
			old.RoomID = r.RoomID
		}
		if r.Zone != "" {
			old.Zone = r.Zone
		}
		if r.Source != "" {
			old.Source = r.Source
		}
		if r.SourceScore > 0 {
			old.SourceScore = r.SourceScore
		}
		old.Weight += r.Weight
		if old.Weight < 1 {
			old.Weight = 1
		}
		if r.UpdatedAt > 0 {
			old.UpdatedAt = r.UpdatedAt
		}
		if r.ExpiresAt > old.ExpiresAt {
			old.ExpiresAt = r.ExpiresAt
		}
	} else {
		cp := *r
		if cp.Weight <= 0 {
			cp.Weight = 1
		}
		if cp.SourceScore <= 0 {
			cp.SourceScore = 1
		}
		s.NpcRumors[id] = &cp
	}
	s.mu.Unlock()
	return s.persistNpcRumors()
}

// TopNpcRumors 取指定 room/zone 最相關 topK 傳聞；會略過過期與空文本。
func (s *Store) TopNpcRumors(roomID, zone string, nowUnix int64, topK int) []NpcRumor {
	if s == nil || topK <= 0 {
		return nil
	}
	s.mu.RLock()
	var list []NpcRumor
	for _, r := range s.NpcRumors {
		if r == nil || strings.TrimSpace(r.Text) == "" {
			continue
		}
		if r.ExpiresAt > 0 && nowUnix > r.ExpiresAt {
			continue
		}
		if r.BlockedUntil > nowUnix {
			continue
		}
		score := r.Weight + r.SourceScore
		switch r.Source {
		case "job":
			score += 3
		case "economy":
			score += 0
		case "room_event":
			score += 2
		case "spawn":
			score += 1
		}
		if roomID != "" && r.RoomID == roomID {
			score += 5
		}
		if zone != "" && r.Zone == zone {
			score += 2
		}
		cp := *r
		cp.Weight = score
		list = append(list, cp)
	}
	s.mu.RUnlock()
	sort.Slice(list, func(i, j int) bool {
		if list[i].Weight == list[j].Weight {
			return list[i].UpdatedAt > list[j].UpdatedAt
		}
		return list[i].Weight > list[j].Weight
	})
	// P7：來源配額（每來源先取 1）+ 同文本去重，再補齊到 topK。
	selected := make([]NpcRumor, 0, topK)
	usedSource := map[string]int{}
	usedText := map[string]bool{}
	rest := make([]NpcRumor, 0, len(list))
	for _, it := range list {
		key := canonicalRumorText(it.Text)
		if key == "" || usedText[key] {
			continue
		}
		src := strings.TrimSpace(it.Source)
		if src != "" && usedSource[src] >= 1 {
			rest = append(rest, it)
			continue
		}
		selected = append(selected, it)
		usedText[key] = true
		if src != "" {
			usedSource[src]++
		}
		if len(selected) >= topK {
			return selected
		}
	}
	for _, it := range rest {
		key := canonicalRumorText(it.Text)
		if key == "" || usedText[key] {
			continue
		}
		selected = append(selected, it)
		usedText[key] = true
		if len(selected) >= topK {
			break
		}
	}
	return selected
}

// DecayNpcRumors 依時間衰減權重並移除過期；每次呼叫後持久化。
func (s *Store) DecayNpcRumors(nowUnix int64) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	changed := false
	for id, r := range s.NpcRumors {
		if r == nil {
			delete(s.NpcRumors, id)
			changed = true
			continue
		}
		if r.ExpiresAt > 0 && nowUnix > r.ExpiresAt {
			delete(s.NpcRumors, id)
			changed = true
			continue
		}
		// 每 15 分鐘衰減 1 點（最低 1）
		if r.UpdatedAt > 0 && nowUnix-r.UpdatedAt >= 900 && r.Weight > 1 {
			r.Weight--
			changed = true
		}
		// P8：長時間未被引用則可信度下修（每小時最多降 1）
		if r.LastUsedAt > 0 && nowUnix-r.LastUsedAt >= 3600 {
			if r.SourceScore > 1 {
				r.SourceScore--
				changed = true
			} else if r.Weight > 1 {
				r.Weight--
				changed = true
			}
		}
	}
	s.mu.Unlock()
	if changed {
		return s.persistNpcRumors()
	}
	return nil
}

// MarkRumorUsedByText 標記某條傳聞被引用（P8：動態可信度學習）。
func (s *Store) MarkRumorUsedByText(text string, nowUnix int64) error {
	if s == nil {
		return nil
	}
	key := canonicalRumorText(text)
	if key == "" {
		return nil
	}
	s.mu.Lock()
	changed := false
	for _, r := range s.NpcRumors {
		if r == nil {
			continue
		}
		if canonicalRumorText(r.Text) != key {
			continue
		}
		r.MentionCount++
		r.LastUsedAt = nowUnix
		// 被反覆引用的傳聞逐步升權（上限避免失控）
		if r.MentionCount%3 == 0 && r.Weight < 20 {
			r.Weight++
		}
		if r.MentionCount%5 == 0 && r.SourceScore < 8 {
			r.SourceScore++
		}
		changed = true
		break
	}
	s.mu.Unlock()
	if changed {
		return s.persistNpcRumors()
	}
	return nil
}

// PenalizeRumorByText 對衝突傳聞做降權與短期封鎖（P9）。
func (s *Store) PenalizeRumorByText(text string, nowUnix int64, reason string) error {
	if s == nil {
		return nil
	}
	key := canonicalRumorText(text)
	if key == "" {
		return nil
	}
	s.mu.Lock()
	changed := false
	for _, r := range s.NpcRumors {
		if r == nil {
			continue
		}
		if canonicalRumorText(r.Text) != key {
			continue
		}
		if r.Weight > 1 {
			r.Weight -= 2
			if r.Weight < 1 {
				r.Weight = 1
			}
		}
		if r.SourceScore > 1 {
			r.SourceScore--
		}
		// 衝突傳聞暫停 15 分鐘，不再注入 prompt
		r.BlockedUntil = nowUnix + 900
		r.UpdatedAt = nowUnix
		r.PenaltyCount++
		r.LastPenaltyAt = nowUnix
		r.LastPenaltyReason = strings.TrimSpace(reason)
		changed = true
		break
	}
	s.mu.Unlock()
	if changed {
		return s.persistNpcRumors()
	}
	return nil
}

// DebugNpcRumors 回傳 room/zone 相關傳聞（含封鎖中項目）供觀測面板使用（P10）。
func (s *Store) DebugNpcRumors(roomID, zone string, nowUnix int64, limit int) []NpcRumor {
	if s == nil || limit <= 0 {
		return nil
	}
	s.mu.RLock()
	list := make([]NpcRumor, 0, len(s.NpcRumors))
	for _, r := range s.NpcRumors {
		if r == nil || strings.TrimSpace(r.Text) == "" {
			continue
		}
		if r.ExpiresAt > 0 && nowUnix > r.ExpiresAt {
			continue
		}
		if roomID != "" && r.RoomID != "" && r.RoomID != roomID {
			continue
		}
		if zone != "" && r.Zone != "" && r.Zone != zone {
			continue
		}
		cp := *r
		list = append(list, cp)
	}
	s.mu.RUnlock()
	sort.Slice(list, func(i, j int) bool {
		si := list[i].Weight + list[i].SourceScore
		sj := list[j].Weight + list[j].SourceScore
		if si == sj {
			return list[i].UpdatedAt > list[j].UpdatedAt
		}
		return si > sj
	})
	if len(list) > limit {
		list = list[:limit]
	}
	return list
}

// ResetNpcRumorSignals 清空傳聞的動態訊號（引用/封鎖/懲罰），保留文本與基礎權重。
func (s *Store) ResetNpcRumorSignals() error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	changed := false
	for _, r := range s.NpcRumors {
		if r == nil {
			continue
		}
		if r.MentionCount != 0 || r.LastUsedAt != 0 || r.BlockedUntil != 0 || r.PenaltyCount != 0 || r.LastPenaltyAt != 0 || r.LastPenaltyReason != "" {
			r.MentionCount = 0
			r.LastUsedAt = 0
			r.BlockedUntil = 0
			r.PenaltyCount = 0
			r.LastPenaltyAt = 0
			r.LastPenaltyReason = ""
			changed = true
		}
	}
	s.mu.Unlock()
	if changed {
		return s.persistNpcRumors()
	}
	return nil
}

type npcRumorDigestFile struct {
	Digest *NpcRumorDigest `json:"digest"`
}

func (s *Store) loadNpcRumorDigest() error {
	if s == nil || s.npcRumorDigestPath == "" {
		return nil
	}
	data, err := os.ReadFile(s.npcRumorDigestPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var f npcRumorDigestFile
	if err := json.Unmarshal(data, &f); err != nil {
		return err
	}
	s.mu.Lock()
	s.NpcRumorDigest = f.Digest
	s.mu.Unlock()
	return nil
}

func (s *Store) persistNpcRumorDigest() error {
	if s == nil || s.npcRumorDigestPath == "" {
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(s.npcRumorDigestPath), 0755); err != nil {
		return err
	}
	s.mu.RLock()
	d := s.NpcRumorDigest
	s.mu.RUnlock()
	raw, err := json.MarshalIndent(npcRumorDigestFile{Digest: d}, "", "  ")
	if err != nil {
		return err
	}
	tmpPath := s.npcRumorDigestPath + ".tmp"
	if err := os.WriteFile(tmpPath, raw, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmpPath, s.npcRumorDigestPath); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

func (s *Store) GetNpcRumorDigest() *NpcRumorDigest {
	if s == nil {
		return nil
	}
	s.mu.RLock()
	defer s.mu.RUnlock()
	if s.NpcRumorDigest == nil {
		return nil
	}
	cp := *s.NpcRumorDigest
	return &cp
}

// BuildNpcRumorDigest 用目前傳聞池建立一句摘要並持久化（可離線批次定時執行）。
func (s *Store) BuildNpcRumorDigest(nowUnix int64) error {
	if s == nil {
		return nil
	}
	top := s.TopNpcRumors("", "", nowUnix, 5)
	if len(top) == 0 {
		return nil
	}
	parts := make([]string, 0, 3)
	for i := 0; i < len(top) && i < 3; i++ {
		t := strings.TrimSpace(top[i].Text)
		if t == "" {
			continue
		}
		parts = append(parts, truncToRunes(t, 16))
	}
	if len(parts) == 0 {
		return nil
	}
	d := &NpcRumorDigest{
		Text:        "近日鎮上：" + strings.Join(parts, "；"),
		SourceCount: len(top),
		UpdatedAt:   nowUnix,
	}
	s.mu.Lock()
	s.NpcRumorDigest = d
	s.mu.Unlock()
	return s.persistNpcRumorDigest()
}

func truncToRunes(s string, n int) string {
	r := []rune(s)
	if len(r) <= n {
		return s
	}
	return string(r[:n]) + "…"
}

func canonicalRumorText(s string) string {
	s = strings.TrimSpace(strings.ToLower(s))
	if s == "" {
		return ""
	}
	replacer := strings.NewReplacer(
		" ", "", "\t", "", "\n", "",
		"，", "", "。", "", "！", "", "？", "",
		",", "", ".", "", "!", "", "?", "",
		"；", "", ";", "", "：", "", ":", "",
		"「", "", "」", "", "\"", "", "'", "",
	)
	return replacer.Replace(s)
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
