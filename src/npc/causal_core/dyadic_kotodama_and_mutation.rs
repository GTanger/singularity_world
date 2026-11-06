/// 雙人相遇言出法隨計畫（承載雙向選擇、實物交割、關係鉤子與後續行動推進）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DyadicKotodamaPlan {
    pub plan_code: String,
    pub mg_delta_a: i32,
    pub clear_obligation_a: Option<String>,
    pub open_obligation_a: Option<(String, String)>,
    pub clear_obligation_b: Option<String>,
    pub open_obligation_b: Option<(String, String)>,
    pub hook_a: Option<(String, String, i8)>,
    pub hook_b: Option<(String, String, i8)>,
    pub next_plan_step_a: Option<String>,
    pub next_plan_step_b: Option<String>,
    pub surprise_a: Option<String>,
    pub surprise_b: Option<String>,
    pub keep_thread_open: bool,
    pub topic_override: String,
    pub prompt_directive: String,
}

/// 依雙方真實生計狀態、未完義務、多元性格與人際關係，判定言出法隨雙人實物事件
pub fn plan_dyadic_kotodama_encounter(
    a_id: &str,
    a_name: &str,
    a_role: &str,
    a_mg: i32,
    a_ob: Option<(String, String)>,
    b_id: &str,
    b_name: &str,
    b_role: &str,
    b_mg: i32,
    b_ob: Option<(String, String)>,
    room_name: &str,
    fam: i32,
    sen: i32,
    _now_unix: i64,
) -> Option<DyadicKotodamaPlan> {
    let a_seed = deterministic_resident_seed(a_id, None);
    let b_seed = deterministic_resident_seed(b_id, None);
    let s_name = clean_settlement_name(room_name);

    let (a_reps, a_hooks, a_stress) = {
        if let Ok(guard) = LIVE_NPC_CAUSAL_STATES.read() {
            if let Some(st) = guard.get(a_id) {
                (st.same_task_repetitions, st.hooks.clone(), st.stress)
            } else {
                (1, Vec::new(), 35)
            }
        } else {
            (1, Vec::new(), 35)
        }
    };
    let a_weights = compute_plural_personality_weights_with_experience(
        a_role,
        s_name,
        a_seed,
        a_stress,
        a_mg,
        a_reps,
        &a_hooks,
        Some(PsychSituationSlot::RelationalEncounter),
    );

    let (b_reps, b_hooks, b_stress) = {
        if let Ok(guard) = LIVE_NPC_CAUSAL_STATES.read() {
            if let Some(st) = guard.get(b_id) {
                (st.same_task_repetitions, st.hooks.clone(), st.stress)
            } else {
                (1, Vec::new(), 20)
            }
        } else {
            (1, Vec::new(), 20)
        }
    };
    let b_weights = compute_plural_personality_weights_with_experience(
        b_role,
        s_name,
        b_seed,
        b_stress,
        b_mg,
        b_reps,
        &b_hooks,
        Some(PsychSituationSlot::RelationalEncounter),
    );
    // (a) 雙方既有借貸債務關係 (Existing debt between A and B)
    let existing_hook_a = find_relational_hook_for_target(a_id, b_id, b_name);
    let existing_hook_b = find_relational_hook_for_target(b_id, a_id, a_name);

    let all_obs_a = crate::store::get_all_active_npc_obligations(a_id);
    let all_obs_b = crate::store::get_all_active_npc_obligations(b_id);

    let has_debt_obligation_a = all_obs_a.iter().any(|(k, target)| k == "repay_debt" && (target == b_id || target == b_name))
        || matches!(a_ob.as_ref(), Some((k, target)) if k == "repay_debt" && (target == b_id || target == b_name));
    let has_debt_hook_a = existing_hook_a.as_ref().is_some_and(|h| h.kind_label == "立契借貸" || h.kind_label == "展期欠債");

    let has_debt_obligation_b = all_obs_b.iter().any(|(k, target)| k == "repay_debt" && (target == a_id || target == a_name))
        || matches!(b_ob.as_ref(), Some((k, target)) if k == "repay_debt" && (target == a_id || target == a_name));
    let has_debt_hook_b = existing_hook_b.as_ref().is_some_and(|h| h.kind_label == "立契借貸" || h.kind_label == "展期欠債");

    if has_debt_obligation_a || has_debt_hook_a {
        if a_mg >= 4 {
            // 清償債務：A 支付 4 鎂幣歸還 B
            return Some(DyadicKotodamaPlan {
                plan_code: "settle_debt".to_string(),
                mg_delta_a: -4,
                clear_obligation_a: Some("repay_debt".to_string()),
                open_obligation_a: None,
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    "守諾償清".to_string(),
                    format!("於{room_name}當面如數結清積欠{b_name}之四枚鎂幣"),
                    10,
                )),
                hook_b: Some((
                    "守諾償清".to_string(),
                    format!("於{room_name}如數收訖{a_name}償還之四枚鎂幣"),
                    10,
                )),
                next_plan_step_a: Some(format!("與{b_name}結清舊欠，如釋重負，重新打點生計")),
                next_plan_step_b: None,
                surprise_a: Some(format!("當面償清積欠{b_name}之欠款，心中一塊大石落地")),
                surprise_b: Some(format!("收訖{a_name}如期償還之四枚鎂幣，對其誠信頗感慰藉")),
                keep_thread_open: false,
                topic_override: "結清舊欠：當面結算歸還四枚鎂幣".to_string(),
                prompt_directive: format!(
                    "【已拍板言出法隨：當面結清欠款】{a_name}如數掏出四枚鎂幣償清對{b_name}的借款，雙方帳目兩清、如釋重負；請在對話中體現交割與守諾。"
                ),
            });
        } else {
            // 無力還款：依 B 之多元性格權重評估寬限或逼討
            let b_benevolence = b_weights.care + b_weights.honor + (a_weights.honor + a_weights.order) / 4 + fam / 2 + sen * 2;
            let b_strictness = b_weights.profit + b_weights.pride;

            let (hook_b_label, hook_b_summary, hook_b_delta, prompt_text) = if b_benevolence >= b_strictness {
                (
                    "寬限債期".to_string(),
                    format!("於{room_name}體諒{a_name}手頭拮据，寬限數日容其緩頰籌款"),
                    1,
                    format!("【已拍板言出法隨：舊欠展期寬限】{a_name}手頭匱乏難以償還舊欠，向{b_name}告急求告；{b_name}權衡之下體諒其難處，允諾展期再籌；請在對話中體現求告與寬限。"),
                )
            } else {
                (
                    "追討舊欠".to_string(),
                    format!("於{room_name}嚴詞督促{a_name}儘速設法清償舊欠"),
                    -2,
                    format!("【已拍板言出法隨：嚴詞追討舊欠】{a_name}囊空如洗無力還款，{b_name}言辭緊逼、責其不可背信，限其儘速籌措；請在對話中體現催逼與窘迫。"),
                )
            };

            return Some(DyadicKotodamaPlan {
                plan_code: "default_extend_debt".to_string(),
                mg_delta_a: 0,
                clear_obligation_a: None,
                open_obligation_a: None,
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    "展期欠債".to_string(),
                    format!("於{room_name}向{b_name}告貸求寬限，舊欠展期再籌"),
                    if b_benevolence >= b_strictness { 1 } else { -4 },
                )),
                hook_b: Some((
                    hook_b_label,
                    hook_b_summary,
                    hook_b_delta,
                )),
                next_plan_step_a: Some(format!("設法籌措鎂幣償還{b_name}之欠款")),
                next_plan_step_b: None,
                surprise_a: Some(format!("因舊欠無力償還而受制於{b_name}，心急如焚籌款")),
                surprise_b: Some(format!("查覺{a_name}依舊手頭羞澀，記掛未還舊帳")),
                keep_thread_open: true,
                topic_override: "催討舊欠：就尚未償清之欠款交涉緩急".to_string(),
                prompt_directive: prompt_text,
            });
        }
    } else if has_debt_obligation_b || has_debt_hook_b {
        if b_mg >= 4 {
            // 清償債務：B 支付 4 鎂幣歸還 A (A 收入 +4，B 扣減 4)
            return Some(DyadicKotodamaPlan {
                plan_code: "settle_debt".to_string(),
                mg_delta_a: 4,
                clear_obligation_a: None,
                open_obligation_a: None,
                clear_obligation_b: Some("repay_debt".to_string()),
                open_obligation_b: None,
                hook_a: Some((
                    "守諾償清".to_string(),
                    format!("於{room_name}如數收訖{b_name}償還之四枚鎂幣"),
                    10,
                )),
                hook_b: Some((
                    "守諾償清".to_string(),
                    format!("於{room_name}當面如數結清積欠{a_name}之四枚鎂幣"),
                    10,
                )),
                next_plan_step_a: None,
                next_plan_step_b: Some(format!("與{a_name}結清舊欠，如釋重負，重新打點生計")),
                surprise_a: Some(format!("收訖{b_name}如期償還之四枚鎂幣，對其誠信頗感慰藉")),
                surprise_b: Some(format!("當面償清積欠{a_name}之欠款，心中一塊大石落地")),
                keep_thread_open: false,
                topic_override: "結清舊欠：當面結算歸還四枚鎂幣".to_string(),
                prompt_directive: format!(
                    "【已拍板言出法隨：當面結清欠款】{b_name}如數掏出四枚鎂幣償清對{a_name}的借款，雙方帳目兩清、如釋重負；請在對話中體現交割與守諾。"
                ),
            });
        } else {
            // B 無力還款：依 A 之多元性格權重評估寬限或逼討
            let a_benevolence = a_weights.care + a_weights.honor + (b_weights.honor + b_weights.order) / 4 + fam / 2 + sen * 2;
            let a_strictness = a_weights.profit + a_weights.pride;

            let (hook_a_label, hook_a_summary, hook_a_delta, prompt_text) = if a_benevolence >= a_strictness {
                (
                    "寬限債期".to_string(),
                    format!("於{room_name}體諒{b_name}手頭拮据，寬限數日容其緩頰籌款"),
                    1,
                    format!("【已拍板言出法隨：舊欠展期寬限】{b_name}手頭匱乏難以償還舊欠，向{a_name}告急求告；{a_name}權衡之下體諒其難處，允諾展期再籌；請在對話中體現求告與寬限。"),
                )
            } else {
                (
                    "追討舊欠".to_string(),
                    format!("於{room_name}嚴詞督促{b_name}儘速設法清償舊欠"),
                    -2,
                    format!("【已拍板言出法隨：嚴詞追討舊欠】{b_name}囊空如洗無力還款，{a_name}言辭緊逼、責其不可背信，限其儘速籌措；請在對話中體現催逼與窘迫。"),
                )
            };

            return Some(DyadicKotodamaPlan {
                plan_code: "default_extend_debt".to_string(),
                mg_delta_a: 0,
                clear_obligation_a: None,
                open_obligation_a: None,
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    hook_a_label,
                    hook_a_summary,
                    hook_a_delta,
                )),
                hook_b: Some((
                    "展期欠債".to_string(),
                    format!("於{room_name}向{a_name}告貸求寬限，舊欠展期再籌"),
                    if a_benevolence >= a_strictness { 1 } else { -4 },
                )),
                next_plan_step_a: None,
                next_plan_step_b: Some(format!("設法籌措鎂幣償還{a_name}之欠款")),
                surprise_a: Some(format!("查覺{b_name}依舊手頭羞澀，記掛未還舊帳")),
                surprise_b: Some(format!("因舊欠無力償還而受制於{a_name}，心急如焚籌款")),
                keep_thread_open: true,
                topic_override: "催討舊欠：就尚未償清之欠款交涉緩急".to_string(),
                prompt_directive: prompt_text,
            });
        }
    }

    // (b) 急迫匱乏 vs 餘裕 (Urgent scarcity vs surplus)
    if a_mg <= 5 && b_mg >= 15 && a_ob.is_none() {
        let benevolence = b_weights.care + b_weights.honor + (a_weights.honor + a_weights.craft) / 3 + fam + sen * 3;
        let self_interest = b_weights.profit + b_weights.pride;

        if benevolence >= self_interest {
            let loan_mg = 4;
            let loan_word = "四";
            return Some(DyadicKotodamaPlan {
                plan_code: "lend_magnesium".to_string(),
                mg_delta_a: loan_mg,
                clear_obligation_a: None,
                open_obligation_a: Some(("repay_debt".to_string(), b_id.to_string())),
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    "立契借貸".to_string(),
                    format!("於{room_name}蒙{b_name}慷慨解囊借與{loan_word}枚鎂幣應急"),
                    8,
                )),
                hook_b: Some((
                    "解囊借貸".to_string(),
                    format!("於{room_name}借與{a_name}{loan_word}枚鎂幣周轉，立下還款口約"),
                    6,
                )),
                next_plan_step_a: Some(format!("持向{b_name}借得之{loan_word}枚鎂幣採買急需並籌款歸還")),
                next_plan_step_b: None,
                surprise_a: Some(format!("在窘迫之際獲{b_name}借貸{loan_word}枚鎂幣，解燃眉之急")),
                surprise_b: Some(format!("念及街坊情分借出{loan_word}枚鎂幣予{a_name}，囑其度過難關後歸還")),
                keep_thread_open: true,
                topic_override: format!("借貸應急：借與{loan_word}枚鎂幣周轉，立下還款口約"),
                prompt_directive: format!(
                    "【已拍板言出法隨：慷慨借貸】{a_name}生計窘迫見底，{b_name}深明其難，當面掏出{loan_word}枚鎂幣借與周轉；{a_name}銘感在心立約日後必償；請在對話中體現解囊借貸與感激立契。"
                ),
            });
        } else {
            return Some(DyadicKotodamaPlan {
                plan_code: "refuse_loan".to_string(),
                mg_delta_a: 0,
                clear_obligation_a: None,
                open_obligation_a: None,
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    "求借遭拒".to_string(),
                    format!("於{room_name}向{b_name}告貸無果，遭其拒絕"),
                    -3,
                )),
                hook_b: Some((
                    "婉拒借貸".to_string(),
                    format!("於{room_name}以手頭緊俏為由婉拒{a_name}告貸求助"),
                    -1,
                )),
                next_plan_step_a: Some("借貸無門，轉赴外圍自力採集變現".to_string()),
                next_plan_step_b: None,
                surprise_a: Some(format!("向{b_name}求借周轉遭拒，自尊受挫且生計更加緊迫")),
                surprise_b: Some(format!("婉拒了{a_name}借錢周轉的請求，各人自掃門前雪")),
                keep_thread_open: false,
                topic_override: "告貸遭拒：無奈求助卻遭閉門羹".to_string(),
                prompt_directive: format!(
                    "【已拍板言出法隨：婉拒借貸】{a_name}手頭窘迫試圖向{b_name}告借周轉，{b_name}算盤謹慎或各有難處婉言拒絕；請在對話中體現求借無門之尷尬與生計壓力。"
                ),
            });
        }
    }

    // (c) 差事義務轉包託付 (Task obligation subcontracting)
    if let Some((kind, subj)) = &a_ob {
        if (kind == "repair_tool" || kind == "stranded_cargo") && a_mg >= 4 && b_ob.is_none() {
            if b_weights.craft + b_weights.profit + b_weights.honor >= 110 && sen >= -10 {
                return Some(DyadicKotodamaPlan {
                    plan_code: "subcontract_obligation".to_string(),
                    mg_delta_a: -2,
                    clear_obligation_a: Some(kind.clone()),
                    open_obligation_a: None,
                    clear_obligation_b: None,
                    open_obligation_b: Some((kind.clone(), subj.clone())),
                    hook_a: Some((
                        "出資託付".to_string(),
                        format!("於{room_name}出資兩枚鎂幣託付{b_name}代辦{subj}"),
                        5,
                    )),
                    hook_b: Some((
                        "受託代辦".to_string(),
                        format!("於{room_name}收受{a_name}兩枚鎂幣定金，接下代辦{subj}之差事"),
                        6,
                    )),
                    next_plan_step_a: Some(format!("已付酬兩枚鎂幣託付{b_name}代辦{subj}，轉身操持主業")),
                    next_plan_step_b: Some(format!("前往代辦受託之差事：{subj}")),
                    surprise_a: Some(format!("成功將手頭待辦之{subj}託付予{b_name}，解了燃眉之急")),
                    surprise_b: Some(format!("受{a_name}之託承攬{subj}，賺得兩枚鎂幣酬勞")),
                    keep_thread_open: true,
                    topic_override: format!("差事託付：出資兩枚鎂幣託付代辦{subj}"),
                    prompt_directive: format!(
                        "【已拍板言出法隨：出資託付差事】{a_name}手頭分身乏術，出資兩枚鎂幣當面委託{b_name}代為奔走處理「{subj}」；{b_name}欣然接下差事與訂金；請在對話中落實此樁委託與交割。"
                    ),
                });
            }
        }
    }

    // (d) 場域事件包（EventPack）與劇本包（ScriptPack）雙人當值協作演繹
    if a_ob.is_none() && b_ob.is_none() && (_now_unix.unsigned_abs() + a_id.len() as u64 + b_id.len() as u64) % 6 == 0 {
        if let Some((ev_pack, sc_pack)) =
            crate::game::peninsula_scripts::active_dyadic_event_and_script_pack_for_room(
                room_name,
                Some(a_name),
                Some(b_name),
                _now_unix,
            )
        {
            let raw_obj = ev_pack.key_objects.split('、').next().unwrap_or("手頭器物").trim();
            let clean_single_obj = if let Some(rest) = raw_obj.strip_prefix("WT-") {
                rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ')
            } else {
                raw_obj
            };
            let single_obj = if clean_single_obj.is_empty() { "手頭器物" } else { clean_single_obj };
            return Some(DyadicKotodamaPlan {
                plan_code: "co_enact_script_pack".to_string(),
                mg_delta_a: 0,
                clear_obligation_a: None,
                open_obligation_a: None,
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    "劇本協作".to_string(),
                    format!("於{room_name}與{b_name}協力演繹「{}」", ev_pack.event_title),
                    4,
                )),
                hook_b: Some((
                    "劇本協作".to_string(),
                    format!("於{room_name}同{a_name}協力完成「{}」", ev_pack.event_title),
                    4,
                )),
                next_plan_step_a: Some(format!(
                    "推進場域劇本「{}」：{}",
                    ev_pack.event_title, sc_pack.player_actions
                )),
                next_plan_step_b: Some(format!("配合{a_name}完成「{}」", ev_pack.event_title)),
                surprise_a: Some(format!(
                    "與{b_name}同場料理急務「{}」，{}",
                    ev_pack.event_title, sc_pack.outcome_and_world_change
                )),
                surprise_b: Some(format!("協助{a_name}料理現場差事「{}」", ev_pack.event_title)),
                keep_thread_open: false,
                topic_override: format!("{a_name}與{b_name}協作{}：{}", ev_pack.event_title, sc_pack.player_actions),
                prompt_directive: format!(
                    "【已拍板言出法隨：雙人協作差事「{}」】場域緣由：{}；現場備用器物：{}；協作行動：{}；預期成果：{}。請在對話中自然體現兩人就手頭器物與差事當面配合交接，不得提及任何後台系統名詞或企劃設定詞。",
                    ev_pack.event_title,
                    ev_pack.trigger_and_causal_chain,
                    single_obj,
                    sc_pack.player_actions,
                    sc_pack.outcome_and_world_change
                ),
            });
        }
    }

    None
}

/// 提交並落實雙人言出法隨計畫：鎂幣嚴格守恆轉移、義務狀態流轉、人情鉤子更新、NPC因果大腦推進
pub fn commit_dyadic_kotodama_plan(
    a_id: &str,
    a_name: &str,
    b_id: &str,
    b_name: &str,
    plan: &DyadicKotodamaPlan,
    now_unix: i64,
) {
    // 1. 貨幣守恆交割 (delta_a + delta_b == 0)
    if plan.mg_delta_a != 0 {
        let _ = crate::db::add_magnesium(a_id, plan.mg_delta_a);
        let _ = crate::db::add_magnesium(b_id, -plan.mg_delta_a);
    }

    // 2. 義務狀態流轉
    if let Some(kind) = &plan.clear_obligation_a {
        crate::store::clear_active_npc_obligation(a_id, kind);
    }
    if let Some((kind, subj)) = &plan.open_obligation_a {
        crate::store::open_active_npc_obligation(a_id, kind, subj);
    }
    if let Some(kind) = &plan.clear_obligation_b {
        crate::store::clear_active_npc_obligation(b_id, kind);
    }
    if let Some((kind, subj)) = &plan.open_obligation_b {
        crate::store::open_active_npc_obligation(b_id, kind, subj);
    }

    // 3. 人情／恩怨關係鉤子更新
    if let Some((kind_label, summary, delta)) = &plan.hook_a {
        upsert_relational_hook(a_id, b_id, b_name, kind_label, summary.clone(), *delta, now_unix);
    }
    if let Some((kind_label, summary, delta)) = &plan.hook_b {
        upsert_relational_hook(b_id, a_id, a_name, kind_label, summary.clone(), *delta, now_unix);
    }

    // 4. 更新因果大腦之計畫步驟與意外衝擊
    let mut states = LIVE_NPC_CAUSAL_STATES.write().unwrap_or_else(|e| e.into_inner());
    if plan.next_plan_step_a.is_some() || plan.surprise_a.is_some() {
        if let Some(st) = states.get_mut(a_id) {
            if let Some(step) = &plan.next_plan_step_a {
                st.plan_steps[0] = step.clone();
            }
            if let Some(sur) = &plan.surprise_a {
                st.last_surprise_delta = Some(sur.clone());
            }
        }
    }
    if plan.next_plan_step_b.is_some() || plan.surprise_b.is_some() {
        if let Some(st) = states.get_mut(b_id) {
            if let Some(step) = &plan.next_plan_step_b {
                st.plan_steps[0] = step.clone();
            }
            if let Some(sur) = &plan.surprise_b {
                st.last_surprise_delta = Some(sur.clone());
            }
        }
    }
    drop(states);

    // 5. 若為雙人同場演繹事件包與劇本包，同步注入世界漣漪事實
    if plan.plan_code == "co_enact_script_pack" {
        let fact = WorldRippleFact {
            fact_id: format!("script_pack_{}_{}_{}", a_id, b_id, now_unix),
            kind: RippleFactKind::ApexSlainOrRescued,
            subject_zone_or_settlement: clean_settlement_name(&plan.topic_override).to_string(),
            item_or_actor: format!("{a_name}與{b_name}"),
            order_id: None,
            severity: 0.75,
            origin_tick: now_unix,
            hop_count: 0,
            max_hops: 3,
            prose_summary: plan.topic_override.clone(),
        };
        inject_ripple_fact_into_npc(a_id, fact.clone(), now_unix);
        inject_ripple_fact_into_npc(b_id, fact, now_unix);
    }
}

/// 依據玩家之實際作為（義舉／解圍／交易／交鋒）即時改變住民因果狀態與人情鉤子
pub fn mutate_npc_from_player_action(
    npc_id: &str,
    player_id: &str,
    player_name: &str,
    settlement: &str,
    action_kind: &str,
    item_or_detail: &str,
    mg_delta: i32,
    now_unix: i64,
) {
    let s = clean_settlement_name(settlement);
    match action_kind {
        "trade" | "buy" | "sell" => {
            let lbl = if mg_delta.abs() >= 15 { "大宗交割" } else { "商貿往來" };
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                lbl,
                format!("與義士【{}】在{}交割「{}」，銀貨兩訖", player_name, s, item_or_detail),
                if mg_delta.abs() >= 15 { 4 } else { 2 },
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = st.stress.saturating_sub(4);
            }
            let fact = WorldRippleFact {
                fact_id: format!("deed_{}_{}", player_id, now_unix),
                kind: RippleFactKind::PlayerDeed,
                subject_zone_or_settlement: s.to_string(),
                item_or_actor: player_name.to_string(),
                order_id: None,
                severity: 0.65,
                origin_tick: now_unix,
                hop_count: 0,
                max_hops: 2,
                prose_summary: format!(
                    "義士【{}】在{}與商戶交割「{}」，市面行情隨之一振",
                    player_name, s, item_or_detail
                ),
            };
            inject_ripple_fact_into_npc(npc_id, fact, now_unix);
        }
        "rescue" | "help" | "rescue_cargo" => {
            let lbl = if action_kind == "rescue_cargo" { "山道解圍" } else { "救命之恩" };
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                lbl,
                format!("蒙義士【{}】在{}出手解危「{}」，此恩銘記在心", player_name, s, item_or_detail),
                6,
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = st.stress.saturating_sub(12);
            }
            let fact = WorldRippleFact {
                fact_id: format!("rescue_{}_{}", player_id, now_unix),
                kind: RippleFactKind::PlayerDeed,
                subject_zone_or_settlement: s.to_string(),
                item_or_actor: player_name.to_string(),
                order_id: None,
                severity: 0.85,
                origin_tick: now_unix,
                hop_count: 0,
                max_hops: 3,
                prose_summary: format!(
                    "義士【{}】在{}仗義馳援「{}」，街坊鄰里無不交口稱讚",
                    player_name, s, item_or_detail
                ),
            };
            inject_ripple_fact_into_npc(npc_id, fact, now_unix);
        }
        "return_sharp" => {
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                "守諾歸還",
                format!("義士【{}】守約歸還「{}」且保養得宜，令人感佩", player_name, item_or_detail),
                4,
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = st.stress.saturating_sub(6);
            }
        }
        "return_damaged" => {
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                "毀器芥蒂",
                format!("借予【{}】的「{}」歸還時有所損耗，心頭頗為疼惜", player_name, item_or_detail),
                4,
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = (st.stress + 8).min(100);
            }
        }
        "borrow_item" => {
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                "通融借器",
                format!("曾將隨身「{}」通融借予【{}】應急使用", item_or_detail, player_name),
                2,
                now_unix,
            );
        }
        "combat_subdue" => {
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                "切磋落敗",
                format!("在{}與【{}】交手切磋略遜一籌，心下暗自敬服", s, player_name),
                4,
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = (st.stress + 6).min(100);
            }
        }
        "combat_slay" | "harm" | "provoke" | "attack" => {
            let lbl = if action_kind == "combat_slay" { "結下樑子" } else { "交鋒結怨" };
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                lbl,
                format!("在{}遭【{}】以「{}」相逼，往後需多加提防", s, player_name, item_or_detail),
                8,
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = (st.stress + 15).min(100);
            }
            let fact = WorldRippleFact {
                fact_id: format!("clash_{}_{}", player_id, now_unix),
                kind: RippleFactKind::PlayerDeed,
                subject_zone_or_settlement: s.to_string(),
                item_or_actor: player_name.to_string(),
                order_id: None,
                severity: 0.75,
                origin_tick: now_unix,
                hop_count: 0,
                max_hops: 2,
                prose_summary: format!(
                    "【{}】在{}與人爭執交鋒「{}」，市井間議論紛紛",
                    player_name, s, item_or_detail
                ),
            };
            inject_ripple_fact_into_npc(npc_id, fact, now_unix);
        }
        _ => {
            if !player_name.trim().is_empty()
                && !item_or_detail.trim().is_empty()
                && !item_or_detail.starts_with("grid:")
                && find_relational_hook_for_target(npc_id, player_id, player_name).is_none()
            {
                upsert_relational_hook(
                    npc_id,
                    player_id,
                    player_name,
                    "面會結識",
                    format!("曾於{}與【{}】打過照面", s, player_name),
                    1,
                    now_unix,
                );
            }
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                if mg_delta > 0 {
                    st.stress = st.stress.saturating_sub(2);
                } else if mg_delta < 0 {
                    st.stress = (st.stress + 2).min(100);
                }
            }
        }
    }
}

/// 將滯留貨單之懸賞金（鎂幣）撥入解圍者帳戶，確保全島封閉經濟守恆
pub fn credit_magnesium_bounty(store: &mut crate::store::Store, entity_id: &str, bounty_mg: i32) {
    if bounty_mg <= 0 {
        return;
    }
    if store.entities.contains_key(entity_id) {
        let _ = store.update_entity(entity_id, |ent| {
            ent.magnesium = ent.magnesium.saturating_add(bounty_mg);
        });
    } else {
        let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
        *treasuries.entry("文曜".to_string()).or_insert(2400) += bounty_mg;
    }
}

/// 玩家離線自主行事準則（Offline Autonomy Directive）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfflineAutonomyDirective {
    InnRestAndHeal,         // code: "inn_rest", label: "客房歇息養傷"
    WorkshopCraftAndAssist, // code: "workshop_craft", label: "駐坊百工代工"
    FieldTendAndForage,     // code: "field_tend", label: "照料田壟與林緣巡採"
    RoadEscortAndPatrol,    // code: "road_escort", label: "古道隨行護運"
}

impl OfflineAutonomyDirective {
    pub fn from_code(code: &str) -> Self {
        match code {
            "workshop_craft" => Self::WorkshopCraftAndAssist,
            "field_tend" => Self::FieldTendAndForage,
            "road_escort" => Self::RoadEscortAndPatrol,
            _ => Self::InnRestAndHeal,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::InnRestAndHeal => "inn_rest",
            Self::WorkshopCraftAndAssist => "workshop_craft",
            Self::FieldTendAndForage => "field_tend",
            Self::RoadEscortAndPatrol => "road_escort",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::InnRestAndHeal => "客房歇息養傷",
            Self::WorkshopCraftAndAssist => "駐坊百工代工",
            Self::FieldTendAndForage => "照料田壟與林緣巡採",
            Self::RoadEscortAndPatrol => "古道隨行護運",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Self::InnRestAndHeal => "於客房靜心歇息、調勻氣血與養護體魄，舒緩舊傷疾痛",
            Self::WorkshopCraftAndAssist => "駐留工坊協助匠人推磨校準器物，按行規賺取津貼並熟習百工",
            Self::FieldTendAndForage => "巡行聚落田壟與近郊林緣，照看作物生長並採集應季野生草木",
            Self::RoadEscortAndPatrol => "隨同商隊或獵戶漫步古道，護衛行旅安寧並結識四方旅友",
        }
    }
}

pub static PLAYER_OFFLINE_DIRECTIVES: LazyLock<RwLock<HashMap<String, OfflineAutonomyDirective>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static PLAYER_OFFLINE_RECAPS: LazyLock<RwLock<HashMap<String, Vec<(String, String, String)>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static PLAYER_LAST_OFFLINE_STEP: LazyLock<RwLock<HashMap<String, i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_player_offline_directive(player_id: &str) -> OfflineAutonomyDirective {
    if let Some(dir) = PLAYER_OFFLINE_DIRECTIVES
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(player_id)
        .copied()
    {
        return dir;
    }

    if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        if let Some(e) = s.entities.get(player_id) {
            let combined = format!("{} {}", e.display_title, e.current_activity);
            let matches_any = |keywords: &[&str]| keywords.iter().any(|k| combined.contains(k));
            if matches_any(&["工", "匠", "鍛", "磨", "木", "窯", "織", "鐵"]) {
                return OfflineAutonomyDirective::WorkshopCraftAndAssist;
            }
            if matches_any(&["農", "田", "採", "茶", "林", "漁", "藥", "圃"]) {
                return OfflineAutonomyDirective::FieldTendAndForage;
            }
            if matches_any(&["商", "獵", "衛", "巡", "行", "渡", "哨"]) {
                return OfflineAutonomyDirective::RoadEscortAndPatrol;
            }
        }
    }

    OfflineAutonomyDirective::InnRestAndHeal
}

pub fn set_player_offline_directive(player_id: &str, directive: OfflineAutonomyDirective) {
    let mut map = PLAYER_OFFLINE_DIRECTIVES
        .write()
        .unwrap_or_else(|e| e.into_inner());
    map.insert(player_id.to_string(), directive);
}

