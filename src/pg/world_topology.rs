//! 已接受世界拓撲的 PostgreSQL 種子與讀回；不接觸玩家/NPC 位置。

use crate::store::sql::DbPool;
use crate::world::topology::{TopologyLink, TopologyNode, builtin_white_tower_topology};

const CREATE_TOPOLOGIES: &str = "
    CREATE TABLE IF NOT EXISTS world_topologies (
        world_id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        status TEXT NOT NULL,
        canon BOOLEAN NOT NULL,
        active_version INTEGER NOT NULL,
        sources TEXT NOT NULL
    )
";

const CREATE_NODES: &str = "
    CREATE TABLE IF NOT EXISTS world_topology_nodes (
        world_id TEXT NOT NULL,
        version INTEGER NOT NULL,
        node_id TEXT NOT NULL,
        name TEXT NOT NULL,
        kind TEXT NOT NULL,
        parent_ref TEXT,
        summary TEXT NOT NULL,
        PRIMARY KEY (world_id, version, node_id)
    )
";

const CREATE_LINKS: &str = "
    CREATE TABLE IF NOT EXISTS world_topology_links (
        world_id TEXT NOT NULL,
        version INTEGER NOT NULL,
        link_id TEXT NOT NULL,
        from_ref TEXT NOT NULL,
        to_ref TEXT NOT NULL,
        relation TEXT NOT NULL,
        direction TEXT NOT NULL,
        summary TEXT NOT NULL,
        PRIMARY KEY (world_id, version, link_id)
    )
";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologySeedReport {
    pub world_id: String,
    pub version: i32,
    pub node_count: usize,
    pub link_count: usize,
}

pub fn ensure_builtin(pool: &DbPool) -> anyhow::Result<TopologySeedReport> {
    let topology = builtin_white_tower_topology()?;
    let version = i32::try_from(topology.schema_version)?;
    let sources = serde_json::to_string(&topology.sources)?;
    let mut conn = pool.get()?;
    create_tables(&mut conn)?;
    let mut tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO world_topologies (world_id,name,status,canon,active_version,sources)
         VALUES ($1,$2,$3,$4,$5,$6)
         ON CONFLICT (world_id) DO UPDATE SET name=EXCLUDED.name,status=EXCLUDED.status,
         canon=EXCLUDED.canon,active_version=EXCLUDED.active_version,sources=EXCLUDED.sources",
        &[
            &topology.id,
            &topology.name,
            &topology.status,
            &topology.canon,
            &version,
            &sources,
        ],
    )?;
    upsert_nodes(&mut tx, &topology.id, version, &topology.nodes)?;
    upsert_links(&mut tx, &topology.id, version, &topology.links)?;
    tx.commit()?;
    Ok(TopologySeedReport {
        world_id: topology.id,
        version,
        node_count: topology.nodes.len(),
        link_count: topology.links.len(),
    })
}

fn create_tables(conn: &mut crate::store::sql::Connection) -> anyhow::Result<()> {
    conn.execute(CREATE_TOPOLOGIES, &[])?;
    conn.execute(CREATE_NODES, &[])?;
    conn.execute(CREATE_LINKS, &[])?;
    Ok(())
}

fn upsert_nodes(
    tx: &mut r2d2_postgres::postgres::Transaction<'_>,
    world_id: &str,
    version: i32,
    nodes: &[TopologyNode],
) -> anyhow::Result<()> {
    for node in nodes {
        tx.execute(
            "INSERT INTO world_topology_nodes
             (world_id,version,node_id,name,kind,parent_ref,summary) VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (world_id,version,node_id) DO UPDATE SET name=EXCLUDED.name,
             kind=EXCLUDED.kind,parent_ref=EXCLUDED.parent_ref,summary=EXCLUDED.summary",
            &[
                &world_id,
                &version,
                &node.id,
                &node.display_name,
                &node.kind,
                &node.parent_ref,
                &node.summary,
            ],
        )?;
    }
    Ok(())
}

fn upsert_links(
    tx: &mut r2d2_postgres::postgres::Transaction<'_>,
    world_id: &str,
    version: i32,
    links: &[TopologyLink],
) -> anyhow::Result<()> {
    for link in links {
        tx.execute(
            "INSERT INTO world_topology_links
             (world_id,version,link_id,from_ref,to_ref,relation,direction,summary)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT (world_id,version,link_id) DO UPDATE SET from_ref=EXCLUDED.from_ref,
             to_ref=EXCLUDED.to_ref,relation=EXCLUDED.relation,direction=EXCLUDED.direction,
             summary=EXCLUDED.summary",
            &[
                &world_id,
                &version,
                &link.id,
                &link.from_ref,
                &link.to_ref,
                &link.relation,
                &link.direction,
                &link.summary,
            ],
        )?;
    }
    Ok(())
}
