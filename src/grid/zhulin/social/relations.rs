//! 「關係」作為插件兼插座：底層不可洗白共同履歷 × 表層具體權利轉移帳本（0 好感數字條）。
//!
//! 六大關係插座（對齊規格書 §5.1）：
//! 1. `zl_rel_debt`（債權與賒欠）
//! 2. `zl_rel_supply_pact`（上下游熟客契）
//! 3. `zl_rel_trade_rivalry`（同行競合）
//! 4. `zl_rel_expedition_bond`（進山搭檔與生死債）
//! 5. `zl_rel_craft_lineage`（手藝幫工與心傳）
//! 6. `zl_rel_suspicion_hook`（主觀懷疑與把柄）

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::person::{BeliefKind, ConductEdge, SocialResident, SubjectiveBelief};

/// 共同歷史履歷事件（不可洗白，永遠留存於雙方關係底層）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedHistoryKind {
    SavedLife,
    AbandonedInDanger,
    LoanExtended {
        amount_mg: i32,
    },
    DebtRepaid {
        amount_mg: i32,
    },
    DebtDefaulted {
        amount_mg: i32,
    },
    VesselSeizedForDebt {
        vessel_id: String,
    },
    PaidDebtForOther {
        payer_id: String,
        amount_mg: i32,
    },
    SupplyDelivered {
        item_id: String,
        qty: i32,
        total_mg: i32,
    },
    UndercutRival,
    SharedSecretGrove {
        gx: i32,
        gy: i32,
    },
    CraftMentored,
    AccusedAndConfronted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHistoryEntry {
    pub game_day: i32,
    pub gx: i32,
    pub gy: i32,
    pub kind: SharedHistoryKind,
}

/// 表層可移轉之具體權利槽（取代抽象好感度分數）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrantedRight {
    /// 優先留貨權（不先賣給外客）
    PriorityReserve { holder_id: String, item_id: String },
    /// 帳期展延權
    CreditExtension { holder_id: String, extra_days: i32 },
    /// 私房筍窩／秘境路權
    SecretGroveAccess { holder_id: String, gx: i32, gy: i32 },
    /// 街坊見證作保權
    StreetWitnessGuarantee {
        guarantor_id: String,
        beneficiary_id: String,
    },
    /// 熟客街口代辦整備權（省去逐店跑腿，帶洩密代價）
    StreetMouthPrep { holder_id: String },
}

/// 六大關係插座。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationSocket {
    /// `zl_rel_debt`：債權與賒欠（`creditor_id` 借給 `debtor_id`）
    Debt {
        creditor_id: String,
        debtor_id: String,
        amount_mg: i32,
        due_day: i32,
        collateral_vessel_id: Option<String>,
    },
    /// `zl_rel_supply_pact`：上下游熟客契
    SupplyPact {
        supplier_id: String,
        buyer_id: String,
        item_id: String,
        agreed_unit_mg: i32,
        deliveries: u32,
    },
    /// `zl_rel_trade_rivalry`：同行競合
    TradeRivalry {
        contested_gx: i32,
        contested_gy: i32,
        clash_count: u32,
    },
    /// `zl_rel_expedition_bond`：進山搭檔與生死債
    ExpeditionBond {
        trips: u32,
        saved_count: u32,
        betrayal_count: u32,
    },
    /// `zl_rel_craft_lineage`：手藝幫工與心傳
    CraftLineage {
        mentor_id: String,
        helper_id: String,
        sessions: u32,
    },
    /// `zl_rel_suspicion_hook`：主觀懷疑與把柄
    SuspicionHook {
        accuser_id: String,
        suspect_id: String,
        allegation: String,
        evidence_vessel_id: Option<String>,
    },
}

impl RelationSocket {
    pub fn socket_code(&self) -> &'static str {
        match self {
            Self::Debt { .. } => "zl_rel_debt",
            Self::SupplyPact { .. } => "zl_rel_supply_pact",
            Self::TradeRivalry { .. } => "zl_rel_trade_rivalry",
            Self::ExpeditionBond { .. } => "zl_rel_expedition_bond",
            Self::CraftLineage { .. } => "zl_rel_craft_lineage",
            Self::SuspicionHook { .. } => "zl_rel_suspicion_hook",
        }
    }
}

/// 雙方不可洗白之關係邊（鍵為正規化排序之 `(min_id, max_id)`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationEdge {
    pub a: String,
    pub b: String,
    pub ledger: Vec<SharedHistoryEntry>,
    pub rights: Vec<GrantedRight>,
    pub sockets: Vec<RelationSocket>,
}

impl RelationEdge {
    pub fn new(id1: &str, id2: &str) -> Self {
        let (a, b) = canonical_pair(id1, id2);
        Self {
            a,
            b,
            ledger: Vec::new(),
            rights: Vec::new(),
            sockets: Vec::new(),
        }
    }

    /// 永久追加共同歷史履歷（絕不抹除既有重大事件）。
    pub fn append_history(&mut self, entry: SharedHistoryEntry) {
        self.ledger.push(entry);
        // 保留最多 24 條，優先保留救命／背叛／欠債違約等不可洗白底層事件
        if self.ledger.len() > 24 {
            if let Some(pos) = self.ledger.iter().position(|e| {
                matches!(
                    e.kind,
                    SharedHistoryKind::SupplyDelivered { .. } | SharedHistoryKind::CraftMentored
                )
            }) {
                self.ledger.remove(pos);
            } else {
                self.ledger.remove(0);
            }
        }
    }

    pub fn grant_right(&mut self, right: GrantedRight) {
        if !self.rights.contains(&right) {
            self.rights.push(right);
        }
    }

    /// 取得目前 `debtor_id` 欠 `creditor_id` 的剩餘債額。
    pub fn active_debt(&self, creditor_id: &str, debtor_id: &str) -> i32 {
        self.sockets
            .iter()
            .filter_map(|s| match s {
                RelationSocket::Debt {
                    creditor_id: c,
                    debtor_id: d,
                    amount_mg,
                    ..
                } if c == creditor_id && d == debtor_id => Some(*amount_mg),
                _ => None,
            })
            .sum()
    }
}

/// 正規化關係邊鍵，保證 `(a, b)` 與 `(b, a)` 對應同一筆邊。
pub fn canonical_pair(id1: &str, id2: &str) -> (String, String) {
    if id1 <= id2 {
        (id1.to_string(), id2.to_string())
    } else {
        (id2.to_string(), id1.to_string())
    }
}

/// 全鎮關係帳本（儲存所有雙邊不可洗白履歷、權利槽與六大插座）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationLedger {
    pub edges: HashMap<String, RelationEdge>,
}

impl RelationLedger {
    fn key(id1: &str, id2: &str) -> String {
        let (a, b) = canonical_pair(id1, id2);
        format!("{a}::{b}")
    }

    pub fn get(&self, id1: &str, id2: &str) -> Option<&RelationEdge> {
        self.edges.get(&Self::key(id1, id2))
    }

    pub fn get_or_create_mut(&mut self, id1: &str, id2: &str) -> &mut RelationEdge {
        let k = Self::key(id1, id2);
        self.edges
            .entry(k)
            .or_insert_with(|| RelationEdge::new(id1, id2))
    }

    /// 記錄一筆賒欠債務並掛載 `zl_rel_debt` 插座（底層留履歷）。
    pub fn record_loan(
        &mut self,
        creditor_id: &str,
        debtor_id: &str,
        amount_mg: i32,
        game_day: i32,
        due_day: i32,
        gx: i32,
        gy: i32,
    ) {
        if amount_mg <= 0 || creditor_id == debtor_id {
            return;
        }
        let edge = self.get_or_create_mut(creditor_id, debtor_id);
        edge.append_history(SharedHistoryEntry {
            game_day,
            gx,
            gy,
            kind: SharedHistoryKind::LoanExtended { amount_mg },
        });
        if let Some(RelationSocket::Debt {
            amount_mg: cur,
            due_day: cur_due,
            ..
        }) = edge.sockets.iter_mut().find(|s| {
            matches!(
                s,
                RelationSocket::Debt {
                    creditor_id: c,
                    debtor_id: d,
                    ..
                } if c == creditor_id && d == debtor_id
            )
        }) {
            *cur += amount_mg;
            *cur_due = (*cur_due).max(due_day);
        } else {
            edge.sockets.push(RelationSocket::Debt {
                creditor_id: creditor_id.to_string(),
                debtor_id: debtor_id.to_string(),
                amount_mg,
                due_day,
                collateral_vessel_id: None,
            });
        }
    }

    /// 償還部分或全部債務：債務清零時移除表層 `Debt` 插座，**但底層歷史履歷永久保留**。
    pub fn repay_loan(
        &mut self,
        creditor_id: &str,
        debtor_id: &str,
        repay_mg: i32,
        game_day: i32,
        gx: i32,
        gy: i32,
    ) -> i32 {
        if repay_mg <= 0 {
            return 0;
        }
        let edge = self.get_or_create_mut(creditor_id, debtor_id);
        let mut paid = 0;
        for sock in &mut edge.sockets {
            if let RelationSocket::Debt {
                creditor_id: c,
                debtor_id: d,
                amount_mg,
                ..
            } = sock
                && c == creditor_id
                && d == debtor_id
            {
                let take = (*amount_mg).min(repay_mg - paid);
                *amount_mg -= take;
                paid += take;
            }
        }
        edge.sockets
            .retain(|s| !matches!(s, RelationSocket::Debt { amount_mg, .. } if *amount_mg <= 0));
        if paid > 0 {
            edge.append_history(SharedHistoryEntry {
                game_day,
                gx,
                gy,
                kind: SharedHistoryKind::DebtRepaid { amount_mg: paid },
            });
            // 如期清帳後授予帳期展延權
            edge.grant_right(GrantedRight::CreditExtension {
                holder_id: debtor_id.to_string(),
                extra_days: 3,
            });
        }
        paid
    }

    /// 記錄一次上下游交割，達 3 次自動長出 `zl_rel_supply_pact` 熟客契與優先留貨權（優先連接樞紐自然湧現）。
    pub fn record_supply_trade(
        &mut self,
        supplier_id: &str,
        buyer_id: &str,
        item_id: &str,
        qty: i32,
        total_mg: i32,
        game_day: i32,
        gx: i32,
        gy: i32,
    ) {
        if supplier_id == buyer_id || qty <= 0 {
            return;
        }
        let edge = self.get_or_create_mut(supplier_id, buyer_id);
        edge.append_history(SharedHistoryEntry {
            game_day,
            gx,
            gy,
            kind: SharedHistoryKind::SupplyDelivered {
                item_id: item_id.to_string(),
                qty,
                total_mg,
            },
        });
        let unit_mg = (total_mg / qty).max(1);
        if let Some(RelationSocket::SupplyPact {
            deliveries,
            agreed_unit_mg,
            ..
        }) = edge.sockets.iter_mut().find(|s| {
            matches!(
                s,
                RelationSocket::SupplyPact {
                    supplier_id: sup,
                    buyer_id: buy,
                    item_id: it,
                    ..
                } if sup == supplier_id && buy == buyer_id && it == item_id
            )
        }) {
            *deliveries += 1;
            *agreed_unit_mg = unit_mg;
            if *deliveries >= 3 {
                edge.grant_right(GrantedRight::PriorityReserve {
                    holder_id: buyer_id.to_string(),
                    item_id: item_id.to_string(),
                });
                edge.grant_right(GrantedRight::StreetMouthPrep {
                    holder_id: supplier_id.to_string(),
                });
            }
        } else {
            edge.sockets.push(RelationSocket::SupplyPact {
                supplier_id: supplier_id.to_string(),
                buyer_id: buyer_id.to_string(),
                item_id: item_id.to_string(),
                agreed_unit_mg: unit_mg,
                deliveries: 1,
            });
        }
    }

    /// 統計某人的欠債總額（他欠別人的）與債權總額（別人欠他的）。
    pub fn debt_summary_of(&self, actor_id: &str) -> (i32, i32) {
        let mut owed_out = 0i32;
        let mut owed_in = 0i32;
        for edge in self.edges.values() {
            if edge.a != actor_id && edge.b != actor_id {
                continue;
            }
            for s in &edge.sockets {
                if let RelationSocket::Debt {
                    creditor_id,
                    debtor_id,
                    amount_mg,
                    ..
                } = s
                {
                    if debtor_id == actor_id {
                        owed_out += *amount_mg;
                    }
                    if creditor_id == actor_id {
                        owed_in += *amount_mg;
                    }
                }
            }
        }
        (owed_out, owed_in)
    }

    /// 一次線性掃描計算全鎮每位居民之 `(欠外債, 應收債權)`（$O(E)$，供每拍批次決策使用）。
    pub fn all_debt_summaries(&self) -> HashMap<String, (i32, i32)> {
        let mut map: HashMap<String, (i32, i32)> = HashMap::new();
        for edge in self.edges.values() {
            for s in &edge.sockets {
                if let RelationSocket::Debt {
                    creditor_id,
                    debtor_id,
                    amount_mg,
                    ..
                } = s
                {
                    map.entry(debtor_id.clone()).or_default().0 += *amount_mg;
                    map.entry(creditor_id.clone()).or_default().1 += *amount_mg;
                }
            }
        }
        map
    }

    /// 取得目前所有活躍債務插座 `(creditor_id, debtor_id, amount_mg, due_day)`（確定性排序）。
    pub fn active_debts_sorted(&self) -> Vec<(String, String, i32, i32)> {
        let mut out = Vec::new();
        for edge in self.edges.values() {
            for s in &edge.sockets {
                if let RelationSocket::Debt {
                    creditor_id,
                    debtor_id,
                    amount_mg,
                    due_day,
                    ..
                } = s
                {
                    out.push((creditor_id.clone(), debtor_id.clone(), *amount_mg, *due_day));
                }
            }
        }
        out.sort();
        out
    }

    /// 統計每位居民的活躍關係插座度數（用於自然湧現全鎮前 8～12 名風雲樞紐）。
    pub fn hub_degrees(&self) -> HashMap<String, usize> {
        let mut deg: HashMap<String, usize> = HashMap::new();
        for edge in self.edges.values() {
            let weight = edge.sockets.len() + edge.rights.len();
            if weight > 0 {
                *deg.entry(edge.a.clone()).or_insert(0) += weight;
                *deg.entry(edge.b.clone()).or_insert(0) += weight;
            }
        }
        deg
    }
}

/// 在場目擊者共振（規格書 §6.2）：同格 `z == 0` 旁觀第三人將事件寫入其主觀認知槽，並依耳根軟／多疑稜角傳播回聲。
pub fn propagate_witness_resonance(
    residents: &mut [SocialResident],
    actor_a: &str,
    actor_b: &str,
    gx: i32,
    gy: i32,
    gz: i16,
    game_day: i32,
    belief_kind: BeliefKind,
) -> usize {
    // 僅公開層（z == 0）觸發街坊目擊共振；z > 0 內室／樓上私密無證（規格書 §4.3）
    if gz != 0 {
        return 0;
    }
    let mut witnessed = 0usize;
    for r in residents.iter_mut() {
        if r.id == actor_a || r.id == actor_b {
            continue;
        }
        if r.gx == gx && r.gy == gy && r.gz == 0 {
            let conf = match r.temperament.edge {
                ConductEdge::LooseTongued => 92,
                ConductEdge::Suspicious => 88,
                _ => 75,
            };
            r.push_belief(SubjectiveBelief {
                kind: belief_kind.clone(),
                source_actor_id: actor_a.to_string(),
                game_day,
                confidence: conf,
            });
            witnessed += 1;
        }
    }
    witnessed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debt_repayment_clears_socket_but_preserves_history_and_grants_right() {
        let mut ledger = RelationLedger::default();
        ledger.record_loan("zl_res_0001", "zl_res_0002", 15, 3, 10, -4, 0);
        assert_eq!(ledger.debt_summary_of("zl_res_0002"), (15, 0));
        assert_eq!(ledger.debt_summary_of("zl_res_0001"), (0, 15));

        let paid = ledger.repay_loan("zl_res_0001", "zl_res_0002", 15, 6, -4, 0);
        assert_eq!(paid, 15);
        assert_eq!(ledger.debt_summary_of("zl_res_0002"), (0, 0));
        let edge = ledger
            .get("zl_res_0001", "zl_res_0002")
            .expect("edge exists");
        assert!(edge.sockets.is_empty());
        // 底層不可洗白履歷同時留存借款與清償
        assert_eq!(edge.ledger.len(), 2);
        // 表層獲得帳期展延權
        assert!(edge.rights.iter().any(|r| matches!(
            r,
            GrantedRight::CreditExtension { holder_id, .. } if holder_id == "zl_res_0002"
        )));
    }
}
