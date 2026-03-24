// combat 模組 — 戰鬥判定與文字 log，對齊 Go combat/combat.go（Resolve、ResolveV2、CombatOpt）。

use rand::Rng;

/// 擴充選項：α/β/γ、地形、制伏留人。全為零或未設時以文件預設代入（與 Go `defaultOpt` 一致）。
#[derive(Clone, Debug, Default)]
pub struct CombatOpt {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    pub terrain: String,
    pub subdue: bool,
}

fn default_opt(opt: Option<&CombatOpt>) -> CombatOpt {
    let mut o = match opt {
        None => CombatOpt {
            alpha: 1.0,
            beta: 1.0,
            gamma: 0.0,
            terrain: String::new(),
            subdue: false,
        },
        Some(x) => x.clone(),
    };
    if o.alpha == 0.0 {
        o.alpha = 1.0;
    }
    if o.beta == 0.0 {
        o.beta = 1.0;
    }
    o
}

fn terrain_mult(terrain: &str) -> f64 {
    match terrain.trim().to_lowercase().as_str() {
        "lush" => 1.15,
        "chaos" => 1.08,
        "silent" | "grip" => 1.0,
        "" => 1.0,
        _ => 1.0,
    }
}

/// 第一階段傷害公式：D = max(1, round((1+Atk_Vit/10)*(1-Def_Vit/(Def_Vit+20)))).
pub fn damage_first_stage(atk_vit: i32, def_vit: i32) -> i32 {
    if def_vit <= 0 {
        return atk_vit;
    }
    let mit = def_vit as f64 / (def_vit as f64 + 20.0);
    let raw = (1.0 + atk_vit as f64 / 10.0) * (1.0 - mit);
    raw.round().max(1.0) as i32
}

fn damage_v2(atk_vit: i32, def_vit: i32, opt: Option<&CombatOpt>) -> i32 {
    let base = damage_first_stage(atk_vit, def_vit);
    let Some(o) = opt else {
        return base;
    };
    let d = default_opt(Some(o));
    let m = (base as f64) * d.alpha * terrain_mult(&d.terrain);
    m.round().max(1.0) as i32
}

fn phase_roll<R: Rng + ?Sized>(rng: &mut R, gamma: f64) -> (bool, bool) {
    if gamma <= 0.0 {
        return (false, false);
    }
    let r: f64 = rng.random();
    if r < gamma * 0.08 {
        return (false, true); // deflect
    }
    if r < gamma * 0.08 + gamma * 0.12 {
        return (true, false); // crit
    }
    (false, false)
}

fn apply_phase<R: Rng + ?Sized>(rng: &mut R, base_dmg: i32, gamma: f64) -> (i32, &'static str) {
    let (crit, deflect) = phase_roll(rng, gamma);
    if deflect {
        return (0, " [Conflict:Deflect]");
    }
    if crit {
        let d = ((base_dmg as f64) * 1.5).round().max(1.0) as i32;
        return (d, " [Conflict:Crit]");
    }
    (base_dmg, "")
}

/// 依雙方屬性判定勝負並產出文字 log；HP 初值 = Vit。
pub fn resolve(
    attacker_vit: i32,
    attacker_dex: i32,
    defender_vit: i32,
    defender_dex: i32,
) -> (String, String, i32, i32) {
    let mut a_hp = attacker_vit;
    let mut d_hp = defender_vit;
    const MAX_ROUNDS: i32 = 200;
    let mut rounds = 0;
    let mut log = String::from("戰鬥開始。");
    let attacker_first = attacker_dex >= defender_dex;
    if attacker_first {
        log.push_str(" 攻方先手。");
    } else {
        log.push_str(" 守方先手。");
    }
    while a_hp > 0 && d_hp > 0 && rounds < MAX_ROUNDS {
        rounds += 1;
        if attacker_first {
            let dmg = damage_first_stage(attacker_vit, defender_vit);
            d_hp -= dmg;
            log.push_str(" 攻方擊中守方，");
            if d_hp <= 0 {
                return (
                    "attacker".to_string(),
                    log + " 守方敗。",
                    a_hp,
                    d_hp,
                );
            }
            let dmg = damage_first_stage(defender_vit, attacker_vit);
            a_hp -= dmg;
            log.push_str(" 守方反擊。");
            if a_hp <= 0 {
                return (
                    "defender".to_string(),
                    log + " 攻方敗。",
                    a_hp,
                    d_hp,
                );
            }
        } else {
            let dmg = damage_first_stage(defender_vit, attacker_vit);
            a_hp -= dmg;
            log.push_str(" 守方擊中攻方，");
            if a_hp <= 0 {
                return (
                    "defender".to_string(),
                    log + " 攻方敗。",
                    a_hp,
                    d_hp,
                );
            }
            let dmg = damage_first_stage(attacker_vit, defender_vit);
            d_hp -= dmg;
            log.push_str(" 攻方反擊。");
            if d_hp <= 0 {
                return (
                    "attacker".to_string(),
                    log + " 守方敗。",
                    a_hp,
                    d_hp,
                );
            }
        }
    }
    if a_hp <= 0 && d_hp <= 0 {
        return (
            "defender".to_string(),
            log + " 平手。",
            a_hp,
            d_hp,
        );
    }
    if a_hp <= 0 {
        return (
            "defender".to_string(),
            log + " 攻方敗。",
            a_hp,
            d_hp,
        );
    }
    (
        "attacker".to_string(),
        log + " 守方敗。",
        a_hp,
        d_hp,
    )
}

/// 擴充版：CombatOpt、戰報標籤；`opt == None` 時傷害等同 `resolve`，僅 log 帶 §七 標籤。
pub fn resolve_v2(
    attacker_vit: i32,
    attacker_dex: i32,
    defender_vit: i32,
    defender_dex: i32,
    opt: Option<&CombatOpt>,
) -> (String, String, i32, i32) {
    let mut rng = rand::rng();
    let mut a_hp = attacker_vit;
    let mut d_hp = defender_vit;
    const MAX_ROUNDS: i32 = 200;
    let mut rounds = 0;
    let mut log = String::from("戰鬥開始。");
    let attacker_first = attacker_dex >= defender_dex;
    if attacker_first {
        log.push_str(" [Initiative] 攻方先手。");
    } else {
        log.push_str(" [Initiative] 守方先手。");
    }
    let o = default_opt(opt);
    let subdue = opt.is_some_and(|x| x.subdue);

    while a_hp > 0 && d_hp > 0 && rounds < MAX_ROUNDS {
        rounds += 1;
        if attacker_first {
            let base = damage_v2(attacker_vit, defender_vit, opt);
            let (dmg, tag) = apply_phase(&mut rng, base, o.gamma);
            d_hp -= dmg;
            log.push_str(&format!(
                " [Action:Skill] 攻方擊中守方，傷害 {dmg}。{tag} [Result] 守方剩餘 {d_hp}。"
            ));
            if d_hp <= 0 {
                if subdue {
                    d_hp = 1;
                    log.push_str(" 守方氣竭倒地。 [Result] 攻方留人（制伏）。");
                    return ("attacker".to_string(), log, a_hp, d_hp);
                }
                log.push_str(" 守方敗。 [Result] 攻方取得了勝利。");
                return ("attacker".to_string(), log, a_hp, d_hp);
            }
            let base = damage_v2(defender_vit, attacker_vit, opt);
            let (dmg, tag) = apply_phase(&mut rng, base, o.gamma);
            a_hp -= dmg;
            log.push_str(&format!(
                " 守方反擊。 [Action:Skill] 守方擊中攻方，傷害 {dmg}。{tag} [Result] 攻方剩餘 {a_hp}。"
            ));
            if a_hp <= 0 {
                if subdue {
                    a_hp = 1;
                    log.push_str(" 攻方不支倒地。 [Result] 守方留人（制伏）。");
                    return ("defender".to_string(), log, a_hp, d_hp);
                }
                log.push_str(" 攻方敗。 [Result] 守方取得了勝利。");
                return ("defender".to_string(), log, a_hp, d_hp);
            }
        } else {
            let base = damage_v2(defender_vit, attacker_vit, opt);
            let (dmg, tag) = apply_phase(&mut rng, base, o.gamma);
            a_hp -= dmg;
            log.push_str(&format!(
                " [Action:Skill] 守方擊中攻方，傷害 {dmg}。{tag} [Result] 攻方剩餘 {a_hp}。"
            ));
            if a_hp <= 0 {
                if subdue {
                    a_hp = 1;
                    log.push_str(" 攻方不支倒地。 [Result] 守方留人（制伏）。");
                    return ("defender".to_string(), log, a_hp, d_hp);
                }
                log.push_str(" 攻方敗。 [Result] 守方取得了勝利。");
                return ("defender".to_string(), log, a_hp, d_hp);
            }
            let base = damage_v2(attacker_vit, defender_vit, opt);
            let (dmg, tag) = apply_phase(&mut rng, base, o.gamma);
            d_hp -= dmg;
            log.push_str(&format!(
                " 攻方反擊。 [Action:Skill] 攻方擊中守方，傷害 {dmg}。{tag} [Result] 守方剩餘 {d_hp}。"
            ));
            if d_hp <= 0 {
                if subdue {
                    d_hp = 1;
                    log.push_str(" 守方氣竭倒地。 [Result] 攻方留人（制伏）。");
                    return ("attacker".to_string(), log, a_hp, d_hp);
                }
                log.push_str(" 守方敗。 [Result] 攻方取得了勝利。");
                return ("attacker".to_string(), log, a_hp, d_hp);
            }
        }
    }
    if a_hp <= 0 && d_hp <= 0 {
        log.push_str(" 平手。 [Result] 雙方敗下陣來。");
        return ("defender".to_string(), log, a_hp, d_hp);
    }
    if a_hp <= 0 {
        log.push_str(" 攻方敗。 [Result] 守方取得了勝利。");
        return ("defender".to_string(), log, a_hp, d_hp);
    }
    log.push_str(" 守方敗。 [Result] 攻方取得了勝利。");
    ("attacker".to_string(), log, a_hp, d_hp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_first_stage_matches_go_edge() {
        // Go: Round((1+Vit/10)*(1-Def/(Def+20))), max 1
        assert_eq!(damage_first_stage(10, 10), 1);
        assert_eq!(damage_first_stage(5, 5), 1);
        assert_eq!(damage_first_stage(15, 5), 2);
    }

    #[test]
    fn resolve_v2_nil_matches_resolve_outcome() {
        let a_vit = 8;
        let a_dex = 7;
        let d_vit = 4;
        let d_dex = 3;
        let (w1, _, a1, d1) = resolve(a_vit, a_dex, d_vit, d_dex);
        let (w2, log2, a2, d2) = resolve_v2(a_vit, a_dex, d_vit, d_dex, None);
        assert_eq!(w1, w2, "winner mismatch");
        assert_eq!(a1, a2, "attacker HP mismatch");
        assert_eq!(d1, d2, "defender HP mismatch");
        assert!(log2.contains("[Initiative]"));
        assert!(log2.contains("[Action:Skill]"));
        assert!(log2.contains("[Result]"));
    }

    #[test]
    fn resolve_v2_alpha_terrain_log_tags() {
        let opt = CombatOpt {
            alpha: 1.3,
            terrain: "lush".to_string(),
            ..Default::default()
        };
        let (_w, log, _a, _d) = resolve_v2(5, 5, 5, 5, Some(&opt));
        assert!(log.contains("傷害 "));
        assert!(log.contains("剩餘 "));
    }

    #[test]
    fn subdue_sets_loser_hp_to_one() {
        let opt = CombatOpt {
            alpha: 100.0,
            subdue: true,
            ..Default::default()
        };
        let (w, _log, a, d) = resolve_v2(50, 10, 5, 5, Some(&opt));
        assert_eq!(w, "attacker");
        assert!(a > 0);
        assert_eq!(d, 1);
    }
}
