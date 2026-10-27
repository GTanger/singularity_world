//! 已接受世界地理的座標無關拓撲；不替玩家表面、格制或移動方式作決定。

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

const WHITE_TOWER_TOPOLOGY_JSON: &str =
    include_str!("../../data/world/white_tower_topology_v1.json");

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WorldTopology {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub status: String,
    pub canon: bool,
    pub sources: Vec<String>,
    pub nodes: Vec<TopologyNode>,
    pub links: Vec<TopologyLink>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TopologyNode {
    pub id: String,
    /// 地圖、場所清單與玩家文字表面使用的簡潔名稱。
    /// `name` 保留地理設計稿的結構標籤，兩者不得混作資料庫工程編號。
    pub display_name: String,
    pub name: String,
    pub kind: String,
    pub parent_ref: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TopologyLink {
    pub id: String,
    pub from_ref: String,
    pub to_ref: String,
    pub relation: String,
    pub direction: String,
    pub summary: String,
}

pub fn builtin_white_tower_topology() -> Result<WorldTopology> {
    let topology: WorldTopology = serde_json::from_str(WHITE_TOWER_TOPOLOGY_JSON)?;
    validate_topology(&topology)?;
    Ok(topology)
}

pub fn validate_topology(topology: &WorldTopology) -> Result<()> {
    validate_header(topology)?;
    let nodes = validate_nodes(&topology.nodes)?;
    validate_hierarchy(&topology.nodes, &nodes)?;
    validate_links(&topology.links, &nodes)
}

fn validate_header(topology: &WorldTopology) -> Result<()> {
    ensure!(topology.schema_version > 0, "schema_version 必須大於零");
    ensure_nonblank("topology.id", &topology.id)?;
    ensure_nonblank("topology.name", &topology.name)?;
    ensure!(topology.status == "accepted", "runtime 拓撲必須是 accepted");
    ensure!(topology.canon, "runtime 拓撲必須明示 canon=true");
    ensure!(!topology.sources.is_empty(), "topology.sources 不可為空");
    Ok(())
}

fn validate_nodes(nodes: &[TopologyNode]) -> Result<BTreeMap<&str, &TopologyNode>> {
    ensure!(!nodes.is_empty(), "topology.nodes 不可為空");
    let mut by_id = BTreeMap::new();
    for node in nodes {
        ensure_nonblank("node.id", &node.id)?;
        ensure_nonblank(&format!("{}.name", node.id), &node.name)?;
        ensure_nonblank(&format!("{}.display_name", node.id), &node.display_name)?;
        ensure_nonblank(&format!("{}.kind", node.id), &node.kind)?;
        ensure_nonblank(&format!("{}.summary", node.id), &node.summary)?;
        ensure!(
            by_id.insert(node.id.as_str(), node).is_none(),
            "重複 node id：{}",
            node.id
        );
    }
    Ok(by_id)
}

fn validate_hierarchy(nodes: &[TopologyNode], by_id: &BTreeMap<&str, &TopologyNode>) -> Result<()> {
    let roots = nodes
        .iter()
        .filter(|node| node.parent_ref.is_none())
        .count();
    ensure!(roots == 1, "世界拓撲必須恰有一個 root，實際為 {roots}");
    for node in nodes {
        if let Some(parent) = &node.parent_ref {
            ensure!(parent != &node.id, "{} 不可把自己當 parent", node.id);
            ensure!(
                by_id.contains_key(parent.as_str()),
                "{} parent 不存在",
                node.id
            );
            ensure_acyclic(node, by_id)?;
        }
    }
    Ok(())
}

fn ensure_acyclic(node: &TopologyNode, by_id: &BTreeMap<&str, &TopologyNode>) -> Result<()> {
    let mut seen = BTreeSet::from([node.id.as_str()]);
    let mut cursor = node.parent_ref.as_deref();
    while let Some(parent_id) = cursor {
        ensure!(
            seen.insert(parent_id),
            "{} parent hierarchy 形成循環",
            node.id
        );
        cursor = by_id
            .get(parent_id)
            .and_then(|parent| parent.parent_ref.as_deref());
    }
    Ok(())
}

fn validate_links(links: &[TopologyLink], nodes: &BTreeMap<&str, &TopologyNode>) -> Result<()> {
    ensure!(!links.is_empty(), "topology.links 不可為空");
    let mut ids = BTreeSet::new();
    for link in links {
        ensure_nonblank("link.id", &link.id)?;
        ensure!(ids.insert(link.id.as_str()), "重複 link id：{}", link.id);
        ensure_nonblank(&format!("{}.relation", link.id), &link.relation)?;
        ensure_nonblank(&format!("{}.direction", link.id), &link.direction)?;
        ensure_nonblank(&format!("{}.summary", link.id), &link.summary)?;
        ensure!(link.from_ref != link.to_ref, "{} 不可連向自己", link.id);
        ensure!(
            nodes.contains_key(link.from_ref.as_str()),
            "{} from_ref 不存在",
            link.id
        );
        ensure!(
            nodes.contains_key(link.to_ref.as_str()),
            "{} to_ref 不存在",
            link.id
        );
    }
    Ok(())
}

fn ensure_nonblank(label: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{label} 不可為空白");
    Ok(())
}

#[cfg(test)]
#[path = "topology/tests.rs"]
mod tests;
