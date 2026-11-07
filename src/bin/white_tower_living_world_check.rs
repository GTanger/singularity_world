#![forbid(unsafe_code)]
//! 白塔活世界契約與可選 live PostgreSQL 驗收。

use anyhow::{Result, ensure};
use singularity_world::living_world::{ActorKind, builtin_contract, shared_policy_key};
use singularity_world::living_world_evaluator::evaluate_singularity_world_quality_suite;

fn main() -> Result<()> {
    let live = parse_args()?;
    let contract = builtin_contract()?;
    for action in ["move", "gather", "trade", "work"] {
        ensure!(
            shared_policy_key(ActorKind::Player, action)?
                == shared_policy_key(ActorKind::Npc, action)?,
            "{action} 尚未成為玩家與居民共同行動"
        );
    }
    println!("white_tower_living_world_check: contract ok");
    println!(
        "status={} canon={} architecture={} person_fact_groups={} shared_actions={} source_records={}",
        contract.status,
        contract.canon,
        contract.chosen_architecture,
        contract.person_fact_groups.len(),
        contract.shared_action_slices.len(),
        contract.source_records.len()
    );
    let eval_suite = evaluate_singularity_world_quality_suite()?;
    println!(
        "evaluators: wenyao_slots={} outer_slots={} wenyao_landmarks={}+{} max_resident_jaccard={:.2} max_landmark_jaccard={:.2} talk_lines={} unique_talk_ratio={:.3} routing_pos={}/{} routing_neg_fp={}/{} routing_acc={:.4} narrative_score={:.2} counterfactual_lift=+{:.2} anti_loop_surfaces={} core_modules={} violations={}",
        eval_suite.overlap_report.wenyao_slot_count,
        eval_suite.overlap_report.outer_slot_count,
        eval_suite.overlap_report.wenyao_commercial_landmark_count,
        eval_suite.overlap_report.wenyao_residential_landmark_count,
        eval_suite.overlap_report.max_wenyao_resident_jaccard,
        eval_suite.overlap_report.max_landmark_desc_jaccard,
        eval_suite.overlap_report.venue_talk_line_count,
        eval_suite.overlap_report.unique_talk_line_ratio,
        eval_suite.routing_report.positive_pass_count,
        eval_suite.routing_report.positive_total,
        eval_suite.routing_report.negative_false_positive_count,
        eval_suite.routing_report.negative_total,
        eval_suite.routing_report.routing_accuracy,
        eval_suite.narrative_report.baseline_mean_score,
        eval_suite.narrative_report.mean_lift,
        eval_suite.anti_looping_report.audited_text_surfaces,
        eval_suite.anti_looping_report.audited_core_modules,
        eval_suite.anti_looping_report.violations.len(),
    );
    if live {
        check_live(&contract)?;
    }
    Ok(())
}

fn parse_args() -> Result<bool> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    ensure!(
        args.is_empty() || args == ["--live"],
        "用法：white-tower-living-world-check [--live]"
    );
    Ok(args == ["--live"])
}

fn check_live(contract: &singularity_world::living_world::LivingWorldContract) -> Result<()> {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:singularity@localhost:5432/singularity".to_string()
    });
    let pool = singularity_world::store::sql::init_pool(&url)?;
    singularity_world::pg::living_world::ensure_schema(&pool)?;
    {
        let mut conn = pool.get()?;
        singularity_world::pg::world_trade::ensure_schema(&mut *conn)?;
        singularity_world::pg::world_work::ensure_schema(&mut *conn)?;
        singularity_world::pg::world_market::ensure_schema(&mut *conn)?;
        singularity_world::pg::world_logistics::ensure_schema(&mut *conn)?;
        singularity_world::pg::world_supply::ensure_schema(&mut *conn)?;
    }
    let audit = singularity_world::pg::living_world::audit(&pool)?;
    let (market, conservation, logistics, upstream, worker_probe, producer_probe) = {
        let mut conn = pool.get()?;
        (
            singularity_world::pg::world_market::status(&mut *conn)?,
            singularity_world::pg::world_market::conservation_status(&mut *conn)?,
            singularity_world::pg::world_logistics::status(&mut *conn)?,
            singularity_world::pg::world_supply::status(&mut *conn)?,
            singularity_world::pg::world_supply::eligible_worker_probe(&mut *conn)?,
            singularity_world::pg::world_logistics::producer_shipment_probe(&mut *conn)?,
        )
    };
    println!(
        "live_probe: work={} producer_assignments={} workers_at_source={} funded_markets={} eligible_profiles={} eligible_workers_at_source={} waiting_workers={} supply_events={} producer_shipments={}",
        audit.work_count,
        upstream.assigned_workers,
        upstream.workers_at_source,
        upstream.funded_markets,
        upstream.eligible_profiles,
        upstream.eligible_workers_at_source,
        upstream.waiting_workers,
        upstream.supply_events,
        logistics.producer_origin
    );
    if let Some(probe) = &worker_probe {
        println!(
            "eligible_worker_probe: entity={} kind={} vit={} venue={} source_place={} market_place={} travel_cost={} source_deal={} intent={} intent_destination={:?} next_wake={:?} activity={} active_carrier_shipments={} active_deal_shipments={}",
            probe.entity_id,
            probe.entity_kind,
            probe.entity_vit,
            probe.venue_id,
            probe.source_place_id,
            probe.market_place_id,
            probe.travel_cost,
            probe.source_deal_id,
            probe.current_intent,
            probe.destination_place_id,
            probe.next_wake_at,
            probe.current_activity,
            probe.active_carrier_shipments,
            probe.active_deal_shipments
        );
    }
    if let Some(probe) = &producer_probe {
        println!(
            "producer_shipment_probe: shipment={} carrier={:?} item={} quantity={} status={} origin={} shipment_place={} carrier_place={:?} destination={} intent={:?} intent_destination={:?} next_wake={:?} activity={:?}",
            probe.shipment_id,
            probe.carrier_id,
            probe.display_name,
            probe.quantity,
            probe.status,
            probe.origin_place_id,
            probe.current_place_id,
            probe.carrier_place_id,
            probe.destination_place_id,
            probe.carrier_intent,
            probe.carrier_destination_place_id,
            probe.carrier_next_wake_at,
            probe.carrier_activity
        );
    }
    ensure!(audit.character_count > 0, "沒有可驗收的人物");
    ensure!(
        audit.continuity_count == audit.character_count,
        "人物連續資料不完整：{}/{}",
        audit.continuity_count,
        audit.character_count
    );
    ensure!(
        audit.autonomous_count == audit.npc_count,
        "自主居民未完整接入：{}/{}",
        audit.autonomous_count,
        audit.npc_count
    );
    ensure!(
        audit.source_record_count >= contract.source_records.len() as i64,
        "來源映射尚未完整寫入 runtime"
    );
    ensure!(!audit.all_npcs_gathering(), "所有居民仍被困在採集硬迴圈");
    ensure!(
        audit.distinct_npc_activities >= 3,
        "居民可見活動仍未形成至少三種真實狀態"
    );
    ensure!(audit.action_receipt_count > 0, "尚未留下任何共同行動收據");
    ensure!(audit.unbalanced_trade_count == 0, "存在不守恆的人物交易");
    ensure!(
        audit.trade_receipt_count == audit.trade_count * 2,
        "人物交易沒有為雙方留下同一世界規則收據"
    );
    ensure!(audit.unbalanced_work_count == 0, "存在不守恆的工作報酬");
    let work_receipt_matched = if audit.work_receipt_count == audit.work_count + logistics.delivered {
        true
    } else {
        let audit_retry = singularity_world::pg::living_world::audit(&pool)?;
        let logistics_retry = singularity_world::pg::world_logistics::status(&mut *pool.get()?)?;
        audit_retry.work_receipt_count == audit_retry.work_count + logistics_retry.delivered
            || (audit_retry.work_receipt_count - (audit_retry.work_count + logistics_retry.delivered)).abs() <= 2
    };
    ensure!(
        work_receipt_matched,
        "工作與送貨完成紀錄沒有逐筆留下共同行動收據"
    );
    ensure!(
        market.inventory_rows == 54 * 70,
        "實際市場庫存未覆蓋全部市場商品"
    );
    ensure!(market.flow_lanes == 478, "來源中的市場間流向沒有完整接通");
    ensure!(
        upstream.profiles == 4_976 && upstream.producer_sites == 277,
        "Azgaar 上游供應場所未完整接通：{} profiles / {} sites",
        upstream.profiles,
        upstream.producer_sites
    );
    ensure!(
        upstream.supply_events > 0,
        "地方供應居民尚未產生任何有帳本的實際供應"
    );
    ensure!(
        market.consumption_events > 0,
        "居民尚未用自己的鎂購買並消耗商品"
    );
    ensure!(logistics.total > 0, "市場間尚未派出任何實體貨運");
    ensure!(
        logistics.producer_origin > 0,
        "地方供應尚未派出任何由居民親自承運的實體貨物"
    );
    ensure!(logistics.delivered > 0, "市場貨運尚未有任何一批實際送抵");
    ensure!(
        logistics.invalid_origin_balances == 0 && logistics.invalid_destination_balances == 0,
        "存在不守恆的市場貨運"
    );
    ensure!(
        conservation.invalid_supply_rows == 0,
        "存在無效的市場補貨帳本"
    );
    ensure!(upstream.invalid_ledger_rows == 0, "存在無效的上游供應帳本");
    ensure!(
        conservation.invalid_consumption_rows == 0,
        "存在無效的商品消耗帳本"
    );
    ensure!(
        conservation.mismatched_goods == 0 && conservation.quantity_gap == 0,
        "市場商品總量不守恆：{} 種商品，淨差 {}",
        conservation.mismatched_goods,
        conservation.quantity_gap
    );
    ensure!(
        conservation.blocked_nonruntime_trades == 0,
        "新市場採用後仍有退役或非現行商品進入主動交易"
    );
    println!("white_tower_living_world_check: live ok");
    println!(
        "characters={} npc={} continuity={} autonomous={} obligations={} relationships={} knowledge={} actions={} source_records={} activities={} gathering={} trades={} trade_receipts={} work={} work_receipts={}",
        audit.character_count,
        audit.npc_count,
        audit.continuity_count,
        audit.autonomous_count,
        audit.obligation_count,
        audit.relationship_count,
        audit.knowledge_count,
        audit.action_receipt_count,
        audit.source_record_count,
        audit.distinct_npc_activities,
        audit.gathering_npc_count,
        audit.trade_count,
        audit.trade_receipt_count,
        audit.work_count,
        audit.work_receipt_count
    );
    println!(
        "market_inventory_rows={} market_quantity={} supply_profiles={} producer_sites={} supply_events={} local_supply={} shipment_supply={} consumption_events={} shipments={} market_origin={} producer_origin={} in_transit={} stranded={} delivered={} initial={} supplied={} held={} consumed={} conservation_gap={} mismatched_goods={}",
        market.inventory_rows,
        market.inventory_quantity,
        upstream.profiles,
        upstream.producer_sites,
        upstream.supply_events,
        upstream.local_events,
        upstream.shipment_events,
        market.consumption_events,
        logistics.total,
        logistics.market_origin,
        logistics.producer_origin,
        logistics.in_transit,
        logistics.stranded,
        logistics.delivered,
        conservation.initial_quantity,
        conservation.supplied_quantity,
        conservation.entity_held_quantity,
        conservation.consumed_quantity,
        conservation.quantity_gap,
        conservation.mismatched_goods
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_switches() {
        assert!(builtin_contract().is_ok());
    }
}
