/// 依住民當前處境（傷病、器損、抵達場所、壓力、時間刻度）判定觸發哪一個「決策意象」
pub fn resolve_decision_image(
    arrived_dest_name: &str,
    has_affliction: bool,
    vessel_worn: bool,
    stress: u8,
    life_step_tick: u64,
) -> DecisionImageKind {
    if has_affliction || arrived_dest_name.contains("診") || arrived_dest_name.contains("醫") {
        DecisionImageKind::HealOrTend
    } else if vessel_worn {
        DecisionImageKind::MendOrEquip
    } else if arrived_dest_name.contains("食堂")
        || arrived_dest_name.contains("廚")
        || arrived_dest_name.contains("飯")
        || arrived_dest_name.contains("麵")
    {
        DecisionImageKind::Eat
    } else if arrived_dest_name.contains("茶")
        || arrived_dest_name.contains("酒")
        || arrived_dest_name.contains("泉")
        || arrived_dest_name.contains("井")
    {
        DecisionImageKind::Drink
    } else if arrived_dest_name.contains("浴")
        || arrived_dest_name.contains("湯")
        || arrived_dest_name.contains("居")
        || arrived_dest_name.contains("宅")
        || arrived_dest_name.contains("旅")
        || arrived_dest_name.contains("宿")
        || stress >= 75
    {
        DecisionImageKind::SleepOrRest
    } else if arrived_dest_name.contains("橋")
        || arrived_dest_name.contains("廣場")
        || arrived_dest_name.contains("公議")
        || arrived_dest_name.contains("書閣")
    {
        DecisionImageKind::InteractOrUnwind
    } else if arrived_dest_name.is_empty() && life_step_tick.is_multiple_of(5) {
        DecisionImageKind::Eat
    } else {
        DecisionImageKind::WorkOrTrade
    }
}

/// 篩選某「決策意象」在當前鎂幣（有錢沒錢？有多少錢？）下所有符合條件的預擬「行為動作」選項：
/// - 沒錢（`mg <= 4`）時，僅回傳 2 個求生窄選項（匱乏的選擇少）；
/// - 有小錢（`mg: 5..=24`）時，回傳 3 個務實選項；
/// - 有餘裕（`mg >= 25`）時，回傳 5～6 個豐富選項（有餘裕的選擇多）。
pub fn eligible_actions_for_image(
    image: DecisionImageKind,
    magnesium: i32,
) -> Vec<&'static ActionOptionDef> {
    let mg = magnesium.max(0);
    PREDEFINED_ACTION_CATALOG
        .iter()
        .filter(|opt| {
            opt.image == image
                && mg >= opt.min_mg
                && opt.max_mg.is_none_or(|max| mg <= max)
                && mg >= opt.cost_mg
        })
        .collect()
}

/// 從符合條件的預擬行為動作選項池中，依住民性格基石、多元性格權重與直線時間刻度做出選擇
pub fn choose_action_from_eligible(
    eligible: &[&'static ActionOptionDef],
    cornerstone: Cornerstone,
    life_step_tick: u64,
    seed: u64,
) -> &'static ActionOptionDef {
    if eligible.is_empty() {
        return &PREDEFINED_ACTION_CATALOG[0];
    }
    if eligible.len() == 1 {
        return eligible[0];
    }
    // 計算該住民之多元性格權重（以當前 seed 及預設狀態派生次要傾向）
    let weights = compute_plural_personality_weights("", seed, 30, 10);
    // 以性格基石為主力核心（+85），同時融合多元副性格權重與時間刻度擾動，使副性格亦能真實影響選擇
    let mut best_opt = eligible[0];
    let mut best_score = i64::MIN;
    for (i, opt) in eligible.iter().enumerate() {
        let mut score: i64 = 50;
        if opt.preferred_cornerstone == Some(cornerstone) {
            // 每隔若干刻度允許副性格顯化探路，其餘時間以主要基石為重
            if (life_step_tick + seed).is_multiple_of(7) {
                score += 35;
            } else {
                score += 85;
            }
        }
        if let Some(c) = opt.preferred_cornerstone {
            score += weights.weight_for(c) as i64;
        }
        // 結合不可逆生命直線刻度與種子之確定性微擾
        let jitter = ((life_step_tick.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ seed.wrapping_add(i as u64)) % 23) as i64;
        score += jitter;

        if score > best_score {
            best_score = score;
            best_opt = opt;
        }
    }
    best_opt
}

/// 由實體 ID 與 soul_seed 計算穩定一致的非零種子
pub fn deterministic_resident_seed(entity_id: &str, soul_seed: Option<i64>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in entity_id.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    if let Some(s) = soul_seed
        && s != 0
    {
        h ^= s.unsigned_abs().wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    h.max(1)
}

const DEFAULT_SETTLEMENTS: &[&str] = &[
    "文曜", "半嶺鎮", "風舶角", "風舶嶼", "南哨", "大圳頭", "老堰頭", "北嶺腳", "霜林口",
    "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口", "水尾寮", "長風草甸", "古河巨木林",
    "赤石坡", "落日岬", "驚濤岩岸", "巨藻淺灣", "黑石灣", "西麓林道", "清泉溪谷",
    "翠冠巨木林", "北坡雲杉林", "淺澤泥口", "白茫洲", "貼山深潭", "青籠關", "西嶺礦谷",
    "東海峭壁", "雲腰雪林", "白塔雪巔",
];

/// 靜態聚落公庫（全島守恆循環經濟）：結餘儲蓄與公共派發之樞紐
pub static SETTLEMENT_TREASURIES: LazyLock<RwLock<HashMap<String, i32>>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for &s in DEFAULT_SETTLEMENTS {
        map.insert(s.to_string(), 2400);
    }
    RwLock::new(map)
});
static ECONOMY_NORMALIZED_ONCE: AtomicBool = AtomicBool::new(false);

/// 若尚未執行過全島閉環經濟初次規範化，於安全時機（未持有 Store 讀鎖時）取得寫鎖執行一次
pub fn ensure_circular_economy_normalized_if_needed() {
    if !ECONOMY_NORMALIZED_ONCE.load(Ordering::SeqCst)
        && let Some(arc) = crate::store::get_store()
        && let Some(mut s) = arc.try_write()
    {
        normalize_and_seed_circular_economy_once(&mut s);
    }
}

/// 計算單一住民在初次初始化時之寫實初始鎂幣（約 20% 匱乏 0..=4 鎂、約 35% 溫飽 5..=24 鎂、約 45% 有餘裕 25..=68 鎂）
fn compute_starting_wallet(seed: u64, role_hint: &str) -> i32 {
    let is_wealthy_role = role_hint.contains("主事")
        || role_hint.contains("店長")
        || role_hint.contains("會長")
        || role_hint.contains("大匠")
        || role_hint.contains("主理")
        || role_hint.contains("總教習")
        || role_hint.contains("東家")
        || role_hint.contains("教務長")
        || role_hint.contains("總站長")
        || role_hint.contains("大廚")
        || role_hint.contains("場主")
        || role_hint.contains("主任");
    let is_poor_role = role_hint.contains("學徒")
        || role_hint.contains("雜役")
        || role_hint.contains("腳夫")
        || role_hint.contains("苦力")
        || role_hint.contains("幫工")
        || role_hint.contains("散客");

    if is_wealthy_role {
        25 + ((seed % 44) as i32)
    } else if is_poor_role || (seed % 100) < 20 {
        (seed % 5) as i32
    } else if (seed % 100) < 55 {
        5 + ((seed % 20) as i32)
    } else {
        25 + ((seed % 44) as i32)
    }
}

/// 全島自由循環經濟單次初次規範化（修復歷史未質押暴衝鎂幣，並注入聚落公庫）
pub fn normalize_and_seed_circular_economy_once(store: &mut crate::store::Store) {
    if ECONOMY_NORMALIZED_ONCE.swap(true, Ordering::SeqCst) {
        return;
    }

    let has_hyper_inflation = store
        .entities
        .values()
        .any(|e| e.kind == "npc" && (e.magnesium > 300 || e.magnesium < 0));

    let treasuries_empty = {
        let guard = SETTLEMENT_TREASURIES.read().unwrap_or_else(|e| e.into_inner());
        guard.is_empty()
    };

    if has_hyper_inflation {
        for e in store.entities.values_mut() {
            if e.kind == "npc" {
                let seed = deterministic_resident_seed(&e.id, e.soul_seed);
                let slot = crate::store::wenyao_household_slot_for(&e.id);
                let role_hint = format!("{} {}", slot.brief_tag, e.display_title);
                let starting_wallet = compute_starting_wallet(seed, &role_hint);
                e.magnesium = starting_wallet;
                if let Some((before_tag, _)) = e.current_activity.split_once("（【") {
                    e.current_activity = before_tag.trim().to_string();
                }
                crate::pg::entity::upsert_async(e.clone());
            }
        }
    }

    if has_hyper_inflation || treasuries_empty {

        let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
        for &s in DEFAULT_SETTLEMENTS {
            treasuries.entry(s.to_string()).or_insert(2400);
        }
    }
}

/// 查詢指定聚落公庫之當前餘額（鎂幣）
pub fn settlement_treasury_balance(settlement: &str) -> i32 {
    let s = clean_settlement_name(settlement);
    let guard = SETTLEMENT_TREASURIES.read().unwrap_or_else(|e| e.into_inner());
    guard.get(s).copied().or_else(|| guard.get(settlement).copied()).unwrap_or(2400)
}

/// 將鎂幣存入指定聚落公庫（例如住民用膳消費、稅金），嚴格守恆
pub fn deposit_to_settlement_treasury(settlement: &str, amount: i32) -> i32 {
    if amount <= 0 {
        return settlement_treasury_balance(settlement);
    }
    let s = clean_settlement_name(settlement);
    let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
    let entry = treasuries.entry(s.to_string()).or_insert(2400);
    *entry = entry.saturating_add(amount);
    *entry
}

/// 計算全島閉環經濟之總守恆鎂幣量（∑ npc.magnesium + ∑ SETTLEMENT_TREASURIES + ∑ LIVE_STRANDED_CARGO_ORDERS escrow）
pub fn total_conserved_magnesium(store: &crate::store::Store) -> i64 {
    let now_unix = crate::game::now_unix();
    {
        let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
        let mut expired_refunds: Vec<(String, i32)> = Vec::new();
        for order in orders.values_mut() {
            if now_unix > order.expires_at_unix && order.escrow_bounty_mg > 0 {
                expired_refunds.push((order.dest_settlement.clone(), order.escrow_bounty_mg));
                order.escrow_bounty_mg = 0;
            }
        }
        if !expired_refunds.is_empty() {
            let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
            for (dest, refund) in expired_refunds {
                let s = clean_settlement_name(&dest);
                *treasuries.entry(s.to_string()).or_insert(2400) += refund;
            }
        }
    }
    let npc_total: i64 = store
        .entities
        .values()
        .filter(|e| e.kind == "npc" || e.kind == "player" || e.kind == "Player")
        .map(|e| i64::from(e.magnesium.max(0)))
        .sum();
    let treasuries_total: i64 = SETTLEMENT_TREASURIES
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .map(|&v| i64::from(v.max(0)))
        .sum();
    let escrow_total: i64 = LIVE_STRANDED_CARGO_ORDERS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .map(|o| i64::from(o.escrow_bounty_mg.max(0)))
        .sum();
    let bank_total: i64 = LIVE_PLAYER_BANK_LEDGER
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .map(|&v| i64::from(v.max(0)))
        .sum();
    npc_total + treasuries_total + escrow_total + bank_total
}

/// 判斷預擬行為選項所對應之生態物資分類
pub fn action_commodity_category(opt: &ActionOptionDef) -> &'static str {
    match opt.image {
        DecisionImageKind::Eat => "food",
        DecisionImageKind::Drink => "drink",
        DecisionImageKind::HealOrTend => "clinic",
        DecisionImageKind::MendOrEquip => "mend",
        DecisionImageKind::WorkOrTrade => match opt.code {
            "work_scavenge_wild_firewood_and_scraps" | "work_take_hard_dock_or_haul_odd_job" => "forage",
            "work_peddle_gathered_goods_at_market" => "peddle",
            "work_cross_settlement_scarcity_arbitrage" => "arbitrage",
            _ => "craft",
        },
        _ => "food",
    }
}

/// 依據聚落即時物資供需緊缺倍率，動態計算行為消費支出（零成本求生選項恆為 0 鎂，其餘至少 1 鎂）
pub fn dynamic_action_cost_mg(settlement: &str, opt: &ActionOptionDef) -> i32 {
    if opt.cost_mg <= 0 {
        return 0;
    }
    let (ratio, _, _) = crate::game::object_ecology::settlement_scarcity_multiplier_for_category(
        settlement,
        action_commodity_category(opt),
    );
    ((opt.cost_mg as f32 * ratio).round() as i32).max(1)
}

/// 依據聚落即時物資供需緊缺倍率，動態計算勞作或調撥所得工資（零報酬選項恆為 0 鎂，其餘至少 2 鎂）
pub fn dynamic_action_earn_mg(settlement: &str, opt: &ActionOptionDef) -> i32 {
    if opt.earn_mg <= 0 {
        return 0;
    }
    let (ratio, _, _) = crate::game::object_ecology::settlement_scarcity_multiplier_for_category(
        settlement,
        action_commodity_category(opt),
    );
    ((opt.earn_mg as f32 * ratio).round() as i32).max(2)
}

/// 從聚落公庫支付勞作／採集／交割工資（若公庫吃緊，則由在地商賈注資批發進貨，嚴格守恆）
pub fn pay_from_settlement_treasury(
    store: &mut crate::store::Store,
    worker_id: &str,
    settlement: &str,
    requested_mg: i32,
) -> i32 {
    if requested_mg <= 0 {
        return 0;
    }
    let s = clean_settlement_name(settlement);

    let current_bal = {
        let treasuries = SETTLEMENT_TREASURIES.read().unwrap_or_else(|e| e.into_inner());
        treasuries.get(s).copied().unwrap_or(0)
    };

    if current_bal < 600 || current_bal < requested_mg {
        let mut merchants: Vec<String> = store
            .entities
            .values()
            .filter(|e| {
                e.kind == "npc"
                    && e.id != worker_id
                    && e.magnesium > 45
                    && (e.display_title.contains(s) || e.current_activity.contains(s))
            })
            .map(|e| e.id.clone())
            .collect();

        if merchants.is_empty() {
            merchants = store
                .entities
                .values()
                .filter(|e| e.kind == "npc" && e.id != worker_id && e.magnesium > 55)
                .map(|e| e.id.clone())
                .collect();
        }

        merchants.truncate(3);

        for m_id in merchants {
            let (fee, m_title) = if let Some(m) = store.entities.get_mut(&m_id) {
                let restock_fee = ((m.magnesium - 40) / 3).clamp(2, 12);
                let actual_fee = restock_fee.min(m.magnesium.max(0));
                m.magnesium -= actual_fee;
                crate::pg::entity::upsert_async(m.clone());
                (actual_fee, m.display_title.clone())
            } else {
                (0, String::new())
            };

            if fee > 0 {
                {
                    let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
                    *treasuries.entry(s.to_string()).or_insert(0) += fee;
                }
                crate::game::object_ecology::supply_settlement_commodity_from_work(s, "restock", &m_title);
            }
        }
    }

    let actual_pay = {
        let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
        let bal = treasuries.entry(s.to_string()).or_insert(0);
        let take = requested_mg.min((*bal).max(0));
        *bal -= take;
        take
    };

    if actual_pay > 0 {
        if let Some(worker) = store.entities.get_mut(worker_id) {
            worker.magnesium = worker.magnesium.saturating_add(actual_pay);
            crate::pg::entity::upsert_async(worker.clone());
        } else {
            let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
            *treasuries.entry(s.to_string()).or_insert(0) += actual_pay;
            return 0;
        }
    }

    actual_pay
}

/// 玩家與聚落公庫雙向結算（嚴格符合 SFC 封閉經濟總鎂幣守恆）。
/// - delta_mg > 0: 自聚落公庫扣除相應款項並發放至玩家帳戶，回傳實際領取鎂幣（>= 0）。
/// - delta_mg < 0: 從玩家帳戶扣除款項並納入聚落公庫，回傳實際扣除之負數鎂幣（<= 0）。
pub fn settle_player_with_settlement_treasury(
    player_id: &str,
    settlement: &str,
    delta_mg: i32,
) -> i32 {
    let resolved_place = if settlement.starts_with("grid:") {
        crate::db::get_room_name(settlement).ok().unwrap_or_else(|| settlement.to_string())
    } else {
        settlement.to_string()
    };
    let Some(arc) = crate::store::get_store() else {
        return 0;
    };
    if delta_mg > 0 {
        let mut store = arc.write();
        pay_from_settlement_treasury(&mut store, player_id, &resolved_place, delta_mg)
    } else if delta_mg < 0 {
        let mut store = arc.write();
        let s = clean_settlement_name(&resolved_place);
        if let Some(ent) = store.entities.get_mut(player_id) {
            let actual_deducted = (-delta_mg).min(ent.magnesium.max(0));
            if actual_deducted > 0 {
                ent.magnesium -= actual_deducted;
                crate::pg::entity::upsert_async(ent.clone());
                let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
                *treasuries.entry(s.to_string()).or_insert(2400) += actual_deducted;
            }
            -actual_deducted
        } else {
            0
        }
    } else {
        0
    }
}

/// 住民日常生活消費支出（流向現場主事／同行／貧弱街坊，餘額納入聚落公庫，嚴格守恆）
pub fn spend_into_settlement_circulation(
    store: &mut crate::store::Store,
    payer_id: &str,
    settlement: &str,
    dest_name: &str,
    cost_mg: i32,
    peer_support: bool,
) -> i32 {
    if cost_mg <= 0 {
        return 0;
    }
    let actual_cost = match store.entities.get_mut(payer_id) {
        Some(payer) => {
            let payable = cost_mg.min(payer.magnesium.max(0));
            if payable <= 0 {
                return 0;
            }
            payer.magnesium -= payable;
            crate::pg::entity::upsert_async(payer.clone());
            payable
        }
        None => return 0,
    };

    let s = clean_settlement_name(settlement);

    if peer_support {
        let candidates: Vec<String> = store
            .entities
            .values()
            .filter(|e| e.kind == "npc" && e.id != payer_id && e.magnesium <= 8)
            .map(|e| e.id.clone())
            .collect();

        let (_to_recipient, to_treasury) = if !candidates.is_empty() {
            let idx = (deterministic_resident_seed(payer_id, None) as usize + actual_cost as usize) % candidates.len();
            let rid = candidates[idx].clone();
            let half = actual_cost / 2;
            if half > 0 {
                if let Some(rec) = store.entities.get_mut(&rid) {
                    rec.magnesium = rec.magnesium.saturating_add(half);
                    crate::pg::entity::upsert_async(rec.clone());
                }
            }
            (half, actual_cost - half)
        } else {
            (0, actual_cost)
        };

        if to_treasury > 0 {
            let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
            *treasuries.entry(s.to_string()).or_insert(0) += to_treasury;
        }
    } else {
        let dest_clean = dest_name.rsplit('・').next().unwrap_or(dest_name).trim();
        let candidates: Vec<String> = store
            .entities
            .values()
            .filter(|e| {
                e.kind == "npc"
                    && e.id != payer_id
                    && ((!dest_clean.is_empty()
                        && (e.current_activity.contains(dest_clean)
                            || e.display_title.contains(dest_clean)))
                        || (e.current_activity.contains(s) || e.display_title.contains(s)))
            })
            .map(|e| e.id.clone())
            .collect();

        let to_staff = actual_cost / 4;
        let to_treasury = actual_cost - to_staff;

        if !candidates.is_empty() && to_staff > 0 {
            let idx = (deterministic_resident_seed(payer_id, None) as usize + actual_cost as usize) % candidates.len();
            let rid = candidates[idx].clone();
            if let Some(rec) = store.entities.get_mut(&rid) {
                rec.magnesium = rec.magnesium.saturating_add(to_staff);
                crate::pg::entity::upsert_async(rec.clone());
            }
            if to_treasury > 0 {
                let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
                *treasuries.entry(s.to_string()).or_insert(0) += to_treasury;
            }
        } else {
            let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
            *treasuries.entry(s.to_string()).or_insert(0) += actual_cost;
        }
    }

    actual_cost
}

/// 執行一輪預擬動作之閉環經濟結算與體感生理需求調節
pub fn execute_conserved_action_settlement(
    store: &mut crate::store::Store,
    actor_id: &str,
    settlement: &str,
    dest_name: &str,
    action_code: &str,
    _now_unix: i64,
) -> i32 {
    let opt = lookup_action_option(action_code).unwrap_or(&PREDEFINED_ACTION_CATALOG[0]);
    let peer_support = matches!(
        action_code,
        "eat_treat_peers_at_table"
            | "heal_buy_extra_salve_and_cover_peer_bill"
            | "mend_gift_repaired_vessel_to_junior"
            | "social_bail_out_and_sponsor_struggling_peer"
            | "work_mentor_junior_and_delegate_routine"
    );

    let dyn_cost = dynamic_action_cost_mg(settlement, opt);
    let dyn_earn = dynamic_action_earn_mg(settlement, opt);

    let actor_title = store
        .entities
        .get(actor_id)
        .map(|e| e.display_title.clone())
        .unwrap_or_default();

    let spent = if dyn_cost > 0 {
        let s = spend_into_settlement_circulation(store, actor_id, settlement, dest_name, dyn_cost, peer_support);
        if s >= dyn_cost
            && matches!(
                opt.image,
                DecisionImageKind::Eat
                    | DecisionImageKind::Drink
                    | DecisionImageKind::HealOrTend
                    | DecisionImageKind::MendOrEquip
            )
        {
            crate::game::object_ecology::consume_settlement_commodity_for_action(
                settlement,
                action_commodity_category(opt),
                &actor_title,
            );
        }
        s
    } else {
        0
    };

    let earned = if dyn_earn > 0 && (dyn_cost == 0 || spent >= dyn_cost) {
        crate::game::object_ecology::supply_settlement_commodity_from_work(
            settlement,
            action_commodity_category(opt),
            &actor_title,
        );
        pay_from_settlement_treasury(store, actor_id, settlement, dyn_earn)
    } else {
        0
    };

    earned - spent
}

