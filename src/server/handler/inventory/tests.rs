#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_residents_receive_vocational_word_casket_and_inscribed_vessel() {
        let targets = [
            "秦水生",
            "陳萬福",
            "岑守岳",
            "顧海平",
            crate::store::OUTER_HOUSEHOLD_SLOTS[0].name(),
        ];

        for pid in targets {
            let casket = get_player_word_casket(pid);
            assert!(
                casket.total_count() >= 24,
                "住民 {pid} 之隨身詞匣應被初始化並具備基礎百工詞元"
            );

            // 解析住民對應之本職器物
            let clue = if pid == "秦水生" {
                "水輪石磨匠|老堰頭・水輪石磨工坊|秦水生".to_string()
            } else if let Some(slot) = crate::store::WENYAO_HOUSEHOLD_SLOTS
                .iter()
                .chain(crate::store::OUTER_HOUSEHOLD_SLOTS.iter())
                .find(|s| s.name() == pid)
            {
                format!("{}|{}|{}", slot.brief_tag, slot.work_name, slot.name())
            } else {
                let mut h: u64 = 0xcbf2_9ce4_8422_2325;
                for b in pid.as_bytes() {
                    h ^= u64::from(*b);
                    h = h.wrapping_mul(0x0100_0000_01b3);
                }
                let target_settlement = crate::store::resident_macro_settlement(pid, "").unwrap_or("文曜");
                let matching: Vec<&'static crate::store::WenyaoHouseholdSlot> = if target_settlement == "文曜" {
                    crate::store::WENYAO_HOUSEHOLD_SLOTS.iter().collect()
                } else {
                    crate::store::OUTER_HOUSEHOLD_SLOTS
                        .iter()
                        .filter(|s| s.home_name.starts_with(target_settlement))
                        .collect()
                };
                let slot = matching[(h >> 8) as usize % matching.len()];
                format!("{}|{}|{}", slot.brief_tag, slot.work_name, pid)
            };
            let vessel = crate::npc::causal_core::resolve_vocational_vessel(&clue);
            assert!(
                !casket.inscribed_words_of(vessel.item_id).is_empty(),
                "住民 {pid} 之本職器物 {} 應被銘刻專屬字印",
                vessel.item_id
            );
        }
    }
}
