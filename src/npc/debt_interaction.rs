use std::collections::HashMap;

pub type ActorId = String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorCreditState {
    pub id: ActorId,
    pub cash: i32,
    pub trust: HashMap<ActorId, i32>,
}

impl ActorCreditState {
    pub fn trust_for(&self, other: &str) -> i32 {
        self.trust.get(other).copied().unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeIntent {
    pub item_id: String,
    pub price: i32,
    pub urgency: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NeedSignal {
    pub item_id: String,
    pub max_price: i32,
    pub urgency: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferSignal {
    pub item_id: String,
    pub price: i32,
    pub stock: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebtPolicy {
    pub min_trust: i32,
    pub min_urgency: i32,
    pub max_single_amount: i32,
    pub max_active_debts_total: usize,
    pub max_active_per_debtor: usize,
    pub max_active_per_creditor: usize,
    pub pair_cooldown_ticks: i64,
    pub due_after_ticks: i64,
}

impl Default for DebtPolicy {
    fn default() -> Self {
        Self {
            min_trust: 50,
            min_urgency: 40,
            max_single_amount: 20,
            max_active_debts_total: 32,
            max_active_per_debtor: 2,
            max_active_per_creditor: 6,
            pair_cooldown_ticks: 24,
            due_after_ticks: 72,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebtStatus {
    Active,
    Settled,
    Defaulted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Debt {
    pub id: u64,
    pub debtor: ActorId,
    pub creditor: ActorId,
    pub amount: i32,
    pub due_tick: i64,
    pub source_item_id: String,
    pub status: DebtStatus,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DebtLedger {
    pub debts: Vec<Debt>,
    pair_last_created_tick: HashMap<(ActorId, ActorId), i64>,
}

impl DebtLedger {
    pub fn active_count(&self) -> usize {
        self.debts
            .iter()
            .filter(|debt| debt.status == DebtStatus::Active)
            .count()
    }

    pub fn active_count_for_debtor(&self, debtor: &str) -> usize {
        self.debts
            .iter()
            .filter(|debt| debt.status == DebtStatus::Active && debt.debtor == debtor)
            .count()
    }

    pub fn active_count_for_creditor(&self, creditor: &str) -> usize {
        self.debts
            .iter()
            .filter(|debt| debt.status == DebtStatus::Active && debt.creditor == creditor)
            .count()
    }

    pub fn active_amount_for_debtor(&self, debtor: &str) -> i32 {
        self.debts
            .iter()
            .filter(|debt| debt.status == DebtStatus::Active && debt.debtor == debtor)
            .map(|debt| debt.amount)
            .sum()
    }

    fn pair_is_on_cooldown(
        &self,
        debtor: &str,
        creditor: &str,
        tick: i64,
        cooldown_ticks: i64,
    ) -> bool {
        self.pair_last_created_tick
            .get(&(debtor.to_owned(), creditor.to_owned()))
            .is_some_and(|last_tick| tick - *last_tick < cooldown_ticks)
    }

    fn push_debt(&mut self, debt: Debt, tick: i64) {
        self.pair_last_created_tick
            .insert((debt.debtor.clone(), debt.creditor.clone()), tick);
        self.debts.push(debt);
    }

    pub fn settle_for_debtor(&mut self, debtor: &str, amount_paid: i32) -> i32 {
        let mut remaining = amount_paid.max(0);
        let mut cleared = 0;
        for debt in self.debts.iter_mut() {
            if debt.status == DebtStatus::Active && debt.debtor == debtor {
                if remaining <= 0 {
                    break;
                }
                if debt.amount <= remaining {
                    cleared += debt.amount;
                    remaining -= debt.amount;
                    debt.amount = 0;
                    debt.status = DebtStatus::Settled;
                } else {
                    debt.amount -= remaining;
                    cleared += remaining;
                    remaining = 0;
                }
            }
        }
        cleared
    }
}
pub static LIVE_DEBT_LEDGER: std::sync::LazyLock<std::sync::RwLock<DebtLedger>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(DebtLedger::default()));

pub fn active_debt_amount(debtor_id: &str) -> i32 {
    LIVE_DEBT_LEDGER
        .read()
        .map(|l| l.active_amount_for_debtor(debtor_id))
        .unwrap_or(0)
}

pub fn settle_debt_for_actor(debtor_id: &str, amount_paid: i32) -> i32 {
    let mut ledger = match LIVE_DEBT_LEDGER.write() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };
    ledger.settle_for_debtor(debtor_id, amount_paid)
}

pub fn try_credit_purchase(
    buyer_id: &str,
    seller_id: &str,
    item_id: &str,
    price: i32,
    buyer_cash: i32,
    trust_score: i32,
    now_tick: i64,
) -> CreditOutcome {
    let mut buyer = ActorCreditState {
        id: buyer_id.to_string(),
        cash: buyer_cash.max(0),
        trust: HashMap::new(),
    };
    let mut seller = ActorCreditState {
        id: seller_id.to_string(),
        cash: 0,
        trust: {
            let mut map = HashMap::new();
            map.insert(buyer_id.to_string(), trust_score);
            map
        },
    };
    let intent = TradeIntent {
        item_id: item_id.to_string(),
        price,
        urgency: 50,
    };
    let policy = DebtPolicy {
        min_trust: 15,
        min_urgency: 30,
        max_single_amount: 120,
        ..DebtPolicy::default()
    };
    let mut ledger = match LIVE_DEBT_LEDGER.write() {
        Ok(l) => l,
        Err(poisoned) => poisoned.into_inner(),
    };
    let outcome = resolve_trade_credit(
        &mut buyer,
        &mut seller,
        &intent,
        &mut ledger,
        &policy,
        now_tick,
    );
    match outcome {
        CreditOutcome::DebtCreated {
            debt_id,
            upfront_paid,
            deferred_amount,
        } => CreditOutcome::PurchasedOnCredit {
            cash_paid: upfront_paid,
            debt_created: deferred_amount,
            debt_id,
        },
        other => other,
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreditOutcome {
    Paid {
        amount: i32,
    },
    DebtCreated {
        debt_id: u64,
        upfront_paid: i32,
        deferred_amount: i32,
    },
    PurchasedOnCredit {
        cash_paid: i32,
        debt_created: i32,
        debt_id: u64,
    },
    Rejected {
        reason: CreditRejectReason,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreditRejectReason {
    InvalidTrade,
    SameActor,
    NoNeedForCredit,
    LowTrust,
    LowUrgency,
    AmountTooLarge,
    GlobalDebtCap,
    DebtorDebtCap,
    CreditorDebtCap,
    PairCooldown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarketEncounterOutcome {
    NoInteraction { reason: NoInteractionReason },
    Trade(CreditOutcome),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoInteractionReason {
    NeedOfferMismatch,
    OutOfStock,
    PriceTooHigh,
    LowUrgency,
}

pub fn resolve_autonomous_market_encounter(
    buyer: &mut ActorCreditState,
    seller: &mut ActorCreditState,
    need: &NeedSignal,
    offer: &mut OfferSignal,
    ledger: &mut DebtLedger,
    policy: &DebtPolicy,
    tick: i64,
) -> MarketEncounterOutcome {
    if need.item_id != offer.item_id {
        return MarketEncounterOutcome::NoInteraction {
            reason: NoInteractionReason::NeedOfferMismatch,
        };
    }

    if offer.stock <= 0 {
        return MarketEncounterOutcome::NoInteraction {
            reason: NoInteractionReason::OutOfStock,
        };
    }

    if offer.price > need.max_price {
        return MarketEncounterOutcome::NoInteraction {
            reason: NoInteractionReason::PriceTooHigh,
        };
    }

    if need.urgency < policy.min_urgency {
        return MarketEncounterOutcome::NoInteraction {
            reason: NoInteractionReason::LowUrgency,
        };
    }

    let intent = TradeIntent {
        item_id: offer.item_id.clone(),
        price: offer.price,
        urgency: need.urgency,
    };
    let outcome = resolve_trade_credit(buyer, seller, &intent, ledger, policy, tick);

    if matches!(
        outcome,
        CreditOutcome::Paid { .. } | CreditOutcome::DebtCreated { .. }
    ) {
        offer.stock -= 1;
    }

    MarketEncounterOutcome::Trade(outcome)
}

pub fn resolve_trade_credit(
    buyer: &mut ActorCreditState,
    seller: &mut ActorCreditState,
    intent: &TradeIntent,
    ledger: &mut DebtLedger,
    policy: &DebtPolicy,
    tick: i64,
) -> CreditOutcome {
    if intent.price <= 0 {
        return CreditOutcome::Rejected {
            reason: CreditRejectReason::InvalidTrade,
        };
    }

    if buyer.id == seller.id {
        return CreditOutcome::Rejected {
            reason: CreditRejectReason::SameActor,
        };
    }

    if buyer.cash >= intent.price {
        buyer.cash -= intent.price;
        seller.cash += intent.price;
        return CreditOutcome::Paid {
            amount: intent.price,
        };
    }

    let upfront_paid = buyer.cash.max(0);
    let deferred_amount = intent.price - upfront_paid;

    if deferred_amount <= 0 {
        return CreditOutcome::Rejected {
            reason: CreditRejectReason::NoNeedForCredit,
        };
    }

    if seller.trust_for(&buyer.id) < policy.min_trust {
        return CreditOutcome::Rejected {
            reason: CreditRejectReason::LowTrust,
        };
    }

    if intent.urgency < policy.min_urgency {
        return CreditOutcome::Rejected {
            reason: CreditRejectReason::LowUrgency,
        };
    }

    if deferred_amount > policy.max_single_amount {
        return CreditOutcome::Rejected {
            reason: CreditRejectReason::AmountTooLarge,
        };
    }

    if ledger.active_count() >= policy.max_active_debts_total {
        return CreditOutcome::Rejected {
            reason: CreditRejectReason::GlobalDebtCap,
        };
    }

    if ledger.active_count_for_debtor(&buyer.id) >= policy.max_active_per_debtor {
        return CreditOutcome::Rejected {
            reason: CreditRejectReason::DebtorDebtCap,
        };
    }

    if ledger.active_count_for_creditor(&seller.id) >= policy.max_active_per_creditor {
        return CreditOutcome::Rejected {
            reason: CreditRejectReason::CreditorDebtCap,
        };
    }

    if ledger.pair_is_on_cooldown(&buyer.id, &seller.id, tick, policy.pair_cooldown_ticks) {
        return CreditOutcome::Rejected {
            reason: CreditRejectReason::PairCooldown,
        };
    }

    let debt_id = ledger.debts.len() as u64 + 1;
    let debt = Debt {
        id: debt_id,
        debtor: buyer.id.clone(),
        creditor: seller.id.clone(),
        amount: deferred_amount,
        due_tick: tick + policy.due_after_ticks,
        source_item_id: intent.item_id.clone(),
        status: DebtStatus::Active,
    };

    buyer.cash -= upfront_paid;
    seller.cash += upfront_paid;
    ledger.push_debt(debt, tick);

    CreditOutcome::DebtCreated {
        debt_id,
        upfront_paid,
        deferred_amount,
    }
}

/// 依據場域物件化資訊產出賒欠或核銷帳目記敘，並推進場域事件包與劇本包。
pub fn format_venue_credit_or_debt_ledger_note_and_advance_pack(
    room_name: &str,
    item_or_reason_hint: &str,
    amount_mg: i32,
    is_settlement: bool,
) -> Option<String> {
    let room_trimmed = room_name.trim();
    let hint_trimmed = item_or_reason_hint.trim();
    if room_trimmed.is_empty() || amount_mg <= 0 {
        return None;
    }
    let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
        room_trimmed,
        hint_trimmed,
    );
    let venue = crate::grid::resolve_any_room_venue_object(room_trimmed, "")?;
    let zone_tag = if venue.has_counter_restricted_zone {
        "當值櫃檯帳冊"
    } else {
        "當值作業區帳冊"
    };
    let action_word = if is_settlement {
        "核銷清償"
    } else {
        "登載賒欠"
    };
    Some(format!(
        "【{}・{}】已由{}當面{}{}鎂往來帳目。",
        venue.venue_kind, zone_tag, venue.default_host_title, action_word, amount_mg
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(id: &str, cash: i32) -> ActorCreditState {
        ActorCreditState {
            id: id.to_owned(),
            cash,
            trust: HashMap::new(),
        }
    }

    fn seller_with_trust(id: &str, cash: i32, debtor: &str, trust: i32) -> ActorCreditState {
        let mut actor = actor(id, cash);
        actor.trust.insert(debtor.to_owned(), trust);
        actor
    }

    fn intent(price: i32) -> TradeIntent {
        TradeIntent {
            item_id: "grain".to_owned(),
            price,
            urgency: 80,
        }
    }

    fn need(item_id: &str, max_price: i32, urgency: i32) -> NeedSignal {
        NeedSignal {
            item_id: item_id.to_owned(),
            max_price,
            urgency,
        }
    }

    fn offer(item_id: &str, price: i32, stock: i32) -> OfferSignal {
        OfferSignal {
            item_id: item_id.to_owned(),
            price,
            stock,
        }
    }

    #[test]
    fn cash_payment_does_not_create_debt() {
        let mut buyer = actor("buyer", 12);
        let mut seller = seller_with_trust("seller", 0, "buyer", 100);
        let mut ledger = DebtLedger::default();

        let outcome = resolve_trade_credit(
            &mut buyer,
            &mut seller,
            &intent(10),
            &mut ledger,
            &DebtPolicy::default(),
            0,
        );

        assert_eq!(outcome, CreditOutcome::Paid { amount: 10 });
        assert_eq!(buyer.cash, 2);
        assert_eq!(seller.cash, 10);
        assert_eq!(ledger.active_count(), 0);
    }

    #[test]
    fn autonomous_market_encounter_can_create_debt_without_player_action() {
        let mut buyer = actor("buyer", 2);
        let mut seller = seller_with_trust("seller", 0, "buyer", 80);
        let mut offer = offer("grain", 10, 2);
        let mut ledger = DebtLedger::default();

        let outcome = resolve_autonomous_market_encounter(
            &mut buyer,
            &mut seller,
            &need("grain", 10, 90),
            &mut offer,
            &mut ledger,
            &DebtPolicy::default(),
            12,
        );

        assert_eq!(
            outcome,
            MarketEncounterOutcome::Trade(CreditOutcome::DebtCreated {
                debt_id: 1,
                upfront_paid: 2,
                deferred_amount: 8,
            })
        );
        assert_eq!(offer.stock, 1);
        assert_eq!(ledger.active_count(), 1);
    }

    #[test]
    fn autonomous_market_encounter_skips_when_need_and_offer_do_not_match() {
        let mut buyer = actor("buyer", 20);
        let mut seller = seller_with_trust("seller", 0, "buyer", 100);
        let mut offer = offer("iron", 10, 2);
        let mut ledger = DebtLedger::default();

        let outcome = resolve_autonomous_market_encounter(
            &mut buyer,
            &mut seller,
            &need("grain", 10, 90),
            &mut offer,
            &mut ledger,
            &DebtPolicy::default(),
            12,
        );

        assert_eq!(
            outcome,
            MarketEncounterOutcome::NoInteraction {
                reason: NoInteractionReason::NeedOfferMismatch,
            }
        );
        assert_eq!(offer.stock, 2);
        assert_eq!(ledger.active_count(), 0);
    }

    #[test]
    fn creates_debt_only_when_trade_cannot_settle_and_trust_allows() {
        let mut buyer = actor("buyer", 3);
        let mut seller = seller_with_trust("seller", 0, "buyer", 75);
        let mut ledger = DebtLedger::default();

        let outcome = resolve_trade_credit(
            &mut buyer,
            &mut seller,
            &intent(10),
            &mut ledger,
            &DebtPolicy::default(),
            5,
        );

        assert_eq!(
            outcome,
            CreditOutcome::DebtCreated {
                debt_id: 1,
                upfront_paid: 3,
                deferred_amount: 7,
            }
        );
        assert_eq!(buyer.cash, 0);
        assert_eq!(seller.cash, 3);
        assert_eq!(ledger.active_count(), 1);
        assert_eq!(ledger.active_amount_for_debtor("buyer"), 7);
        assert_eq!(ledger.debts[0].due_tick, 77);
    }

    #[test]
    fn low_trust_or_overlarge_amount_rejects_credit() {
        let mut buyer = actor("buyer", 0);
        let mut seller = seller_with_trust("seller", 0, "buyer", 10);
        let mut ledger = DebtLedger::default();

        let low_trust = resolve_trade_credit(
            &mut buyer,
            &mut seller,
            &intent(10),
            &mut ledger,
            &DebtPolicy::default(),
            0,
        );
        assert_eq!(
            low_trust,
            CreditOutcome::Rejected {
                reason: CreditRejectReason::LowTrust,
            }
        );

        seller.trust.insert("buyer".to_owned(), 100);
        let overlarge = resolve_trade_credit(
            &mut buyer,
            &mut seller,
            &intent(40),
            &mut ledger,
            &DebtPolicy::default(),
            0,
        );
        assert_eq!(
            overlarge,
            CreditOutcome::Rejected {
                reason: CreditRejectReason::AmountTooLarge,
            }
        );
        assert_eq!(ledger.active_count(), 0);
    }

    #[test]
    fn pair_cooldown_prevents_repeated_debt_edges() {
        let policy = DebtPolicy {
            max_active_per_debtor: 3,
            max_active_per_creditor: 3,
            pair_cooldown_ticks: 10,
            ..DebtPolicy::default()
        };
        let mut buyer = actor("buyer", 0);
        let mut seller = seller_with_trust("seller", 0, "buyer", 100);
        let mut ledger = DebtLedger::default();

        let first = resolve_trade_credit(
            &mut buyer,
            &mut seller,
            &intent(4),
            &mut ledger,
            &policy,
            20,
        );
        assert!(matches!(first, CreditOutcome::DebtCreated { .. }));

        let second = resolve_trade_credit(
            &mut buyer,
            &mut seller,
            &intent(4),
            &mut ledger,
            &policy,
            25,
        );
        assert_eq!(
            second,
            CreditOutcome::Rejected {
                reason: CreditRejectReason::PairCooldown,
            }
        );
        assert_eq!(ledger.active_count(), 1);
    }

    #[test]
    fn debt_caps_stop_the_world_from_becoming_a_debt_graph() {
        let policy = DebtPolicy {
            max_active_debts_total: 4,
            max_active_per_debtor: 1,
            max_active_per_creditor: 2,
            pair_cooldown_ticks: 100,
            ..DebtPolicy::default()
        };
        let mut ledger = DebtLedger::default();
        let mut created = 0;
        let mut rejected = 0;

        for i in 0..30 {
            let buyer_id = format!("buyer-{i}");
            let seller_id = format!("seller-{}", i % 3);
            let mut buyer = actor(&buyer_id, 0);
            let mut seller = seller_with_trust(&seller_id, 0, &buyer_id, 100);

            match resolve_trade_credit(&mut buyer, &mut seller, &intent(6), &mut ledger, &policy, i)
            {
                CreditOutcome::DebtCreated { .. } | CreditOutcome::PurchasedOnCredit { .. } => created += 1,
                CreditOutcome::Rejected { .. } => rejected += 1,
                CreditOutcome::Paid { .. } => {}
            }
        }

        assert_eq!(created, 4);
        assert_eq!(ledger.active_count(), 4);
        assert_eq!(ledger.active_count_for_creditor("seller-0"), 2);
        assert_eq!(ledger.active_count_for_creditor("seller-1"), 1);
        assert_eq!(ledger.active_count_for_creditor("seller-2"), 1);
        assert_eq!(rejected, 26);
    }

    #[test]
    fn test_live_debt_ledger_and_credit_purchase() {
        let debtor = "test_player_credit_1";
        let creditor = "test_vendor_credit_1";
        let outcome = try_credit_purchase(debtor, creditor, "mat_timber_log", 50, 10, 25, 100);
        assert!(matches!(
            outcome,
            CreditOutcome::PurchasedOnCredit {
                cash_paid: 10,
                debt_created: 40,
                ..
            }
        ));
        assert_eq!(active_debt_amount(debtor), 40);
        let cleared = settle_debt_for_actor(debtor, 25);
        assert_eq!(cleared, 25);
        assert_eq!(active_debt_amount(debtor), 15);
        let cleared_rest = settle_debt_for_actor(debtor, 30);
        assert_eq!(cleared_rest, 15);
        assert_eq!(active_debt_amount(debtor), 0);
    }

    #[test]
    fn test_format_venue_credit_or_debt_ledger_note_and_advance_pack() {
        let mill_debt = format_venue_credit_or_debt_ledger_note_and_advance_pack(
            "老堰頭・水輪石磨工坊",
            "麥粒 麵粉研磨費",
            35,
            false,
        );
        assert_eq!(
            mill_debt,
            Some("【水輪石磨工坊・當值櫃檯帳冊】已由當值石磨匠師當面登載賒欠35鎂往來帳目。".to_string())
        );

        let canteen_settle = format_venue_credit_or_debt_ledger_note_and_advance_pack(
            "三水大食堂",
            "合菜清償",
            60,
            true,
        );
        assert_eq!(
            canteen_settle,
            Some("【大食堂・當值櫃檯帳冊】已由食堂主廚當面核銷清償60鎂往來帳目。".to_string())
        );

        let non_venue = format_venue_credit_or_debt_ledger_note_and_advance_pack(
            "荒蕪礫石空地",
            "私人物品",
            20,
            false,
        );
        assert_eq!(non_venue, None);
    }
}
