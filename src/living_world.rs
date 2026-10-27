//! 活世界共同契約：人物連續性、共同行動與來源層級。
//!
//! 這裡只定義領域資料與驗證，不認識 WebSocket、NPC 專用流程或 PostgreSQL。

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

pub const LIVING_WORLD_CONTRACT_ID: &str = "white_tower_living_world_v1";
pub const LIVING_WORLD_CONTRACT_JSON: &str =
    include_str!("../data/world/white_tower_living_world_v1.json");

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    Player,
    Npc,
}

impl ActorKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Npc => "npc",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum SourceTier {
    U,
    A,
    W,
    G,
    R,
}

impl SourceTier {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::U => "U",
            Self::A => "A",
            Self::W => "W",
            Self::G => "G",
            Self::R => "R",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SourceRecord {
    pub fact_id: String,
    pub subject_ref: String,
    pub fact_key: String,
    pub fact_value: String,
    pub source_tier: SourceTier,
    pub source_ref: String,
    pub status: String,
    pub canon: bool,
    pub runtime_authority: bool,
    pub runtime_projection: String,
}

impl SourceRecord {
    pub fn validate(&self) -> Result<()> {
        for (label, value) in [
            ("fact_id", self.fact_id.as_str()),
            ("subject_ref", self.subject_ref.as_str()),
            ("fact_key", self.fact_key.as_str()),
            ("fact_value", self.fact_value.as_str()),
            ("source_ref", self.source_ref.as_str()),
            ("status", self.status.as_str()),
        ] {
            ensure!(!value.trim().is_empty(), "{label} 不可空白");
        }
        ensure!(
            !self.canon || matches!(self.source_tier, SourceTier::U | SourceTier::A),
            "{}：W/G/R 不可直接冒充正典",
            self.fact_id
        );
        ensure!(
            !(self.source_tier == SourceTier::R && self.runtime_authority),
            "{}：退出內容不可作 runtime 權威",
            self.fact_id
        );
        ensure!(
            !self.runtime_authority || !self.runtime_projection.trim().is_empty(),
            "{}：runtime 權威必須指出投影位置",
            self.fact_id
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ArchitectureOption {
    pub id: String,
    pub label: String,
    pub verdict: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SharedActionSlice {
    pub action_key: String,
    pub authority: String,
    pub player_entry: String,
    pub npc_entry: String,
    pub receipt_table: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct LivingWorldContract {
    pub schema_version: u32,
    pub id: String,
    pub status: String,
    pub canon: bool,
    pub design_scope: String,
    pub chosen_architecture: String,
    pub architecture_options: Vec<ArchitectureOption>,
    pub person_fact_groups: Vec<String>,
    pub shared_action_slices: Vec<SharedActionSlice>,
    pub source_records: Vec<SourceRecord>,
    pub falsifiers: Vec<String>,
    pub known_debts: Vec<String>,
}

impl LivingWorldContract {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema_version == 1, "不支援的活世界契約版本");
        ensure!(self.id == LIVING_WORLD_CONTRACT_ID, "活世界契約 id 不符");
        ensure!(
            self.status == "working_premise",
            "契約尚未進入 working premise"
        );
        ensure!(!self.canon, "working premise 不可冒充正典");
        ensure!(self.chosen_architecture == "C", "本輪收斂結果必須是 C");
        ensure!(self.architecture_options.len() == 3, "必須保留三個受審方案");
        ensure!(
            self.person_fact_groups.len() == 8,
            "人物連續性必須覆蓋八組事實"
        );
        for action in ["move", "gather", "trade", "work"] {
            let Some(slice) = self
                .shared_action_slices
                .iter()
                .find(|slice| slice.action_key == action)
            else {
                anyhow::bail!("缺少共同行動切片：{action}");
            };
            ensure!(
                slice.player_entry == slice.npc_entry,
                "{action} 的玩家與居民未走同一權威入口"
            );
            ensure!(
                slice.receipt_table == "world_action_receipts",
                "{action} 沒有進共同行動收據"
            );
        }
        ensure!(!self.source_records.is_empty(), "來源對照不可為空");
        for record in &self.source_records {
            record.validate()?;
        }
        ensure!(!self.falsifiers.is_empty(), "缺少可否證驗收");
        Ok(())
    }
}

pub fn builtin_contract() -> Result<LivingWorldContract> {
    let contract: LivingWorldContract = serde_json::from_str(LIVING_WORLD_CONTRACT_JSON)?;
    contract.validate()?;
    Ok(contract)
}

/// 同一 action key 的規則身分只由世界動作決定，不由玩家／NPC 身分分叉。
pub fn shared_policy_key(actor_kind: ActorKind, action_key: &str) -> Result<String> {
    let action_key = action_key.trim().to_ascii_lowercase();
    ensure!(
        matches!(
            action_key.as_str(),
            "move"
                | "gather"
                | "trade"
                | "work"
                | "talk"
                | "borrow"
                | "refused_borrow"
                | "refusedborrow"
                | "subdue"
                | "slay"
        ),
        "尚未接通的共同行動"
    );
    let _metadata_only = actor_kind;
    Ok(format!("world-action-v1:{action_key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_contract_keeps_canon_and_runtime_separate() -> Result<()> {
        let contract = builtin_contract()?;
        assert!(
            contract
                .source_records
                .iter()
                .any(|record| record.source_tier == SourceTier::G
                    && record.runtime_authority
                    && !record.canon)
        );
        assert!(
            contract
                .source_records
                .iter()
                .any(|record| record.source_tier == SourceTier::W
                    && !record.runtime_authority
                    && !record.canon)
        );
        Ok(())
    }

    #[test]
    fn player_and_npc_use_the_same_world_action_policy() -> Result<()> {
        for action in ["move", "gather", "trade", "work"] {
            assert_eq!(
                shared_policy_key(ActorKind::Player, action)?,
                shared_policy_key(ActorKind::Npc, action)?
            );
        }
        Ok(())
    }

    #[test]
    fn retired_source_cannot_be_runtime_authority() {
        let record = SourceRecord {
            fact_id: "retired-example".into(),
            subject_ref: "world".into(),
            fact_key: "setting".into(),
            fact_value: "retired".into(),
            source_tier: SourceTier::R,
            source_ref: "decision:retired".into(),
            status: "retired".into(),
            canon: false,
            runtime_authority: true,
            runtime_projection: "somewhere".into(),
        };
        assert!(record.validate().is_err());
    }
}
