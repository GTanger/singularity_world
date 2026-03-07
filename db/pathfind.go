package db

import (
	"database/sql"
	"encoding/json"
	"log"
	"sync"

	"singularity_world/store"
)

// RoomGraph 房間鄰接圖，從 DB exits 表建立，供 BFS 尋路使用。
type RoomGraph struct {
	mu   sync.RWMutex
	adj  map[string][]string
	tags map[string][]string
	zone map[string]string
	name map[string]string
}

var (
	graphOnce sync.Once
	graph     *RoomGraph
)

func GetGraph() *RoomGraph {
	graphOnce.Do(func() {
		graph = &RoomGraph{
			adj:  make(map[string][]string),
			tags: make(map[string][]string),
			zone: make(map[string]string),
			name: make(map[string]string),
		}
	})
	return graph
}

// BuildGraph 從 store（JSON）或 DB 讀取房間與出口建立鄰接表。若 store.Default 已初始化則優先從 JSON 建圖。
func (g *RoomGraph) BuildGraph(database *sql.DB) error {
	if store.Default != nil {
		return g.BuildGraphFromStore()
	}
	return g.buildGraphFromDB(database)
}

// BuildGraphFromStore 從 store.Default（JSON 背板）建圖。store.Init 後由 BuildGraph 自動使用。
func (g *RoomGraph) BuildGraphFromStore() error {
	if store.Default == nil {
		return nil
	}
	g.mu.Lock()
	defer g.mu.Unlock()

	g.adj = store.Default.Adjacency()
	g.name = store.Default.NameMap()
	g.zone = store.Default.ZoneMap()
	g.tags = store.Default.RoomTagsMap()
	if g.adj == nil {
		g.adj = make(map[string][]string)
	}
	if g.tags == nil {
		g.tags = make(map[string][]string)
	}
	if g.name == nil {
		g.name = make(map[string]string)
	}
	if g.zone == nil {
		g.zone = make(map[string]string)
	}

	log.Printf("[pathfind] graph built from JSON: %d rooms, %d edges", len(g.name), g.edgeCount())
	return nil
}

func (g *RoomGraph) buildGraphFromDB(database *sql.DB) error {
	g.mu.Lock()
	defer g.mu.Unlock()

	g.adj = make(map[string][]string)
	g.tags = make(map[string][]string)
	g.zone = make(map[string]string)
	g.name = make(map[string]string)

	rooms, err := database.Query("SELECT id, name, tags, zone FROM rooms")
	if err != nil {
		return err
	}
	defer rooms.Close()
	for rooms.Next() {
		var id, n, tagsJSON, z string
		if err := rooms.Scan(&id, &n, &tagsJSON, &z); err != nil {
			return err
		}
		g.name[id] = n
		g.zone[id] = z
		var t []string
		if err := json.Unmarshal([]byte(tagsJSON), &t); err == nil {
			g.tags[id] = t
		}
		if _, ok := g.adj[id]; !ok {
			g.adj[id] = nil
		}
	}

	exits, err := database.Query("SELECT from_room_id, to_room_id FROM exits")
	if err != nil {
		return err
	}
	defer exits.Close()
	for exits.Next() {
		var from, to string
		if err := exits.Scan(&from, &to); err != nil {
			return err
		}
		g.adj[from] = append(g.adj[from], to)
	}

	log.Printf("[pathfind] graph built from DB: %d rooms, %d edges", len(g.name), g.edgeCount())
	return nil
}

func (g *RoomGraph) edgeCount() int {
	n := 0
	for _, neighbors := range g.adj {
		n += len(neighbors)
	}
	return n
}

// FindPath BFS 最短路徑，回傳不含起點的節點序列。不可達回傳 nil。
func (g *RoomGraph) FindPath(from, to string) []string {
	if from == to {
		return nil
	}
	g.mu.RLock()
	defer g.mu.RUnlock()

	prev := map[string]string{from: ""}
	queue := []string{from}

	for len(queue) > 0 {
		cur := queue[0]
		queue = queue[1:]

		for _, nb := range g.adj[cur] {
			if _, seen := prev[nb]; seen {
				continue
			}
			prev[nb] = cur
			if nb == to {
				return reconstructPath(prev, from, to)
			}
			queue = append(queue, nb)
		}
	}
	return nil
}

func reconstructPath(prev map[string]string, from, to string) []string {
	var path []string
	for cur := to; cur != from; cur = prev[cur] {
		path = append(path, cur)
	}
	for i, j := 0, len(path)-1; i < j; i, j = i+1, j-1 {
		path[i], path[j] = path[j], path[i]
	}
	return path
}

// FindNearestByTag BFS 找最近帶指定 tag 的房間。maxDist=0 不限深度。
func (g *RoomGraph) FindNearestByTag(origin, tag string, maxDist int) (string, int) {
	g.mu.RLock()
	defer g.mu.RUnlock()

	type bfsNode struct {
		id   string
		dist int
	}
	visited := map[string]bool{origin: true}
	queue := []bfsNode{{origin, 0}}

	for len(queue) > 0 {
		cur := queue[0]
		queue = queue[1:]

		for _, nb := range g.adj[cur.id] {
			if visited[nb] {
				continue
			}
			visited[nb] = true
			nd := cur.dist + 1
			if maxDist > 0 && nd > maxDist {
				continue
			}
			for _, t := range g.tags[nb] {
				if t == tag {
					return nb, nd
				}
			}
			queue = append(queue, bfsNode{nb, nd})
		}
	}
	return "", -1
}

// FindRoomsWithinDist 找 origin 周圍 maxDist 步內帶任一指定 tag 的房間。
func (g *RoomGraph) FindRoomsWithinDist(origin string, tags []string, maxDist int) []string {
	if len(tags) == 0 || maxDist <= 0 {
		return nil
	}
	g.mu.RLock()
	defer g.mu.RUnlock()

	tagSet := make(map[string]bool, len(tags))
	for _, t := range tags {
		tagSet[t] = true
	}

	type bfsNode struct {
		id   string
		dist int
	}
	visited := map[string]bool{origin: true}
	queue := []bfsNode{{origin, 0}}
	var result []string

	for len(queue) > 0 {
		cur := queue[0]
		queue = queue[1:]

		for _, nb := range g.adj[cur.id] {
			if visited[nb] {
				continue
			}
			visited[nb] = true
			nd := cur.dist + 1
			if nd > maxDist {
				continue
			}
			for _, t := range g.tags[nb] {
				if tagSet[t] {
					result = append(result, nb)
					break
				}
			}
			queue = append(queue, bfsNode{nb, nd})
		}
	}
	return result
}

// RoomName 取得快取的房間名稱。
func (g *RoomGraph) RoomName(roomID string) string {
	g.mu.RLock()
	defer g.mu.RUnlock()
	return g.name[roomID]
}

// Neighbors 取得房間的所有相鄰房間 ID。
func (g *RoomGraph) Neighbors(roomID string) []string {
	g.mu.RLock()
	defer g.mu.RUnlock()
	dst := make([]string, len(g.adj[roomID]))
	copy(dst, g.adj[roomID])
	return dst
}

// RoomCount 回傳圖中房間總數。
func (g *RoomGraph) RoomCount() int {
	g.mu.RLock()
	defer g.mu.RUnlock()
	return len(g.name)
}
