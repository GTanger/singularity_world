//! 系統一單 Token Logit 評分器（對接 RX 9060 XT Vulkan0 llama-decision.service：Qwen3-4B-Instruct-2507-Q8_0）。
//! 依據 NPC 的 SoulSeed 性格軸線、未完義務、雙方恩怨度與動態情境拍板四大立場。

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpcStanceChoice {
    Accept,  // A: 同意／正面回應／傾囊相授
    Refuse,  // B: 拒絕／防備／冷淡送客
    Deflect, // C: 敷衍／轉移話題／只談日常
    Bargain, // D: 提出條件交換／索取報酬／請託幫忙
}

#[derive(Debug, Clone)]
pub struct NpcDecisionInput<'a> {
    pub npc_name: &'a str,
    pub boldness: f64,
    pub sensitivity: f64,
    pub orderliness: f64,
    pub familiarity_score: i32,
    pub balance_score: i32,
    pub open_obligations: &'a [String],
    pub current_activity: &'a str,
    pub action_kind: &'a str,
    pub player_utterance: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcStanceDecision {
    pub choice: NpcStanceChoice,
    pub confidence: f64,
    pub prompt_directive: String,
    pub latency_ms: u64,
}

const DECISION_LOGIT_TEMP: f64 = 2.5;

pub fn build_decision_prompt(input: &NpcDecisionInput<'_>) -> String {
    let obligations = if input.open_obligations.is_empty() {
        "無".to_string()
    } else {
        format!("{}（急需他人協助完成此未完義務）", input.open_obligations.join("、"))
    };
    let balance_desc = if input.balance_score < 0 {
        format!("{}（雙方交惡有仇）", input.balance_score)
    } else if input.balance_score > 0 {
        format!("{}（雙方交好互信）", input.balance_score)
    } else {
        "0（關係平淡無恩怨）".to_string()
    };
    format!(
        "<|im_start|>system\n\
你是角色反應決策器。請根據角色的性格、未完義務、雙方關係與來者行動，從以下四個選項中選出最合理的單一選項字母（只輸出 A、B、C 或 D）：\n\
- 若雙方恩怨值為負（有嫌隙或仇怨），或陌生人要求無本賒借、勒索、違規放行，必須選 B（拒絕防備）。\n\
- 若角色有未完義務在身，面對一般搭話或請託，應優先選 D（提出交換條件或請對方先幫忙完成未完義務）或 C（簡短敷衍）。\n\
- 僅在熟人合理請託、危急人命求救、或無未完義務且無嫌隙時，才選 A（同意回應）。\n\
選項定義：\n\
A: 同意請求、正面回應或出手相助\n\
B: 拒絕請求、保持戒備防範或回絕無理要求\n\
C: 轉移話題、簡短敷衍或閃避爭端\n\
D: 提出對等利益交換或請對方先幫忙完成未完義務<|im_end|>\n\
<|im_start|>user\n\
[當前情境] 角色：{npc_name}（大膽{boldness:.2}, 敏感{sensitivity:.2}, 秩序{orderliness:.2}）。目前動態：{current_activity}。未完義務：{obligations}。對來者關係：熟識度={familiarity_score}, 恩怨值={balance_desc}。來者行動({action_kind})：「{player_utterance}」<|im_end|>\n\
<|im_start|>assistant\n",
        npc_name = input.npc_name,
        boldness = input.boldness,
        sensitivity = input.sensitivity,
        orderliness = input.orderliness,
        current_activity = input.current_activity,
        obligations = obligations,
        familiarity_score = input.familiarity_score,
        balance_desc = balance_desc,
        action_kind = input.action_kind,
        player_utterance = input.player_utterance,
    )
}

fn extract_token_probs(resp_val: &serde_json::Value) -> (f64, f64, f64, f64) {
    let mut prob_a = 0.0;
    let mut prob_b = 0.0;
    let mut prob_c = 0.0;
    let mut prob_d = 0.0;

    fn add_token_prob(
        pa: &mut f64,
        pb: &mut f64,
        pc: &mut f64,
        pd: &mut f64,
        token: &str,
        logprob: f64,
    ) {
        let trimmed = token.trim();
        let p = (logprob / DECISION_LOGIT_TEMP).exp();
        if trimmed.eq_ignore_ascii_case("A") {
            *pa += p;
        } else if trimmed.eq_ignore_ascii_case("B") {
            *pb += p;
        } else if trimmed.eq_ignore_ascii_case("C") {
            *pc += p;
        } else if trimmed.eq_ignore_ascii_case("D") {
            *pd += p;
        }
    }

    let choices = resp_val.get("choices").and_then(|c| c.as_array());
    if let Some(choices) = choices
        && let Some(choice) = choices.first()
    {
        // 1. 嘗試解析 choices[0].logprobs.content[0].top_logprobs
        if let Some(content_top) = choice
            .pointer("/logprobs/content/0/top_logprobs")
            .and_then(|v| v.as_array())
        {
            for item in content_top {
                if let Some(token) = item.get("token").and_then(|t| t.as_str())
                    && let Some(logprob) = item.get("logprob").and_then(|lp| lp.as_f64())
                {
                    add_token_prob(&mut prob_a, &mut prob_b, &mut prob_c, &mut prob_d, token, logprob);
                } else if let Some(map) = item.as_object() {
                    for (k, v) in map {
                        if let Some(lp) = v.as_f64() {
                            add_token_prob(&mut prob_a, &mut prob_b, &mut prob_c, &mut prob_d, k, lp);
                        }
                    }
                }
            }
        }

        // 2. 嘗試解析 choices[0].logprobs.top_logprobs[0]
        if prob_a + prob_b + prob_c + prob_d == 0.0
            && let Some(top_logprobs) = choice
                .pointer("/logprobs/top_logprobs/0")
                .or_else(|| choice.pointer("/logprobs/top_logprobs"))
        {
            if let Some(map) = top_logprobs.as_object() {
                for (k, v) in map {
                    if let Some(lp) = v.as_f64() {
                        add_token_prob(&mut prob_a, &mut prob_b, &mut prob_c, &mut prob_d, k, lp);
                    }
                }
            } else if let Some(arr) = top_logprobs.as_array() {
                for item in arr {
                    if let Some(token) = item.get("token").and_then(|t| t.as_str())
                        && let Some(logprob) = item.get("logprob").and_then(|lp| lp.as_f64())
                    {
                        add_token_prob(&mut prob_a, &mut prob_b, &mut prob_c, &mut prob_d, token, logprob);
                    } else if let Some(map) = item.as_object() {
                        for (k, v) in map {
                            if let Some(lp) = v.as_f64() {
                                add_token_prob(&mut prob_a, &mut prob_b, &mut prob_c, &mut prob_d, k, lp);
                            }
                        }
                    }
                }
            }
        }

        // 3. Fallback 回 choices[0].text
        if prob_a + prob_b + prob_c + prob_d == 0.0
            && let Some(text) = choice.get("text").and_then(|t| t.as_str())
        {
            let t = text.trim();
            if t.starts_with('A') || t.starts_with('a') {
                prob_a = 1.0;
            } else if t.starts_with('B') || t.starts_with('b') {
                prob_b = 1.0;
            } else if t.starts_with('C') || t.starts_with('c') {
                prob_c = 1.0;
            } else if t.starts_with('D') || t.starts_with('d') {
                prob_d = 1.0;
            }
        }
    }

    (prob_a, prob_b, prob_c, prob_d)
}

pub fn score_npc_interaction_stance(input: &NpcDecisionInput<'_>) -> NpcStanceDecision {
    let start_time = Instant::now();
    let prompt = build_decision_prompt(input);

    let base_url = std::env::var("DECISION_API_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11436".to_string());
    let base_url = base_url.trim().trim_end_matches('/');
    let endpoint = format!("{base_url}/v1/completions");

    let body = json!({
        "id_slot": 0,
        "prompt": prompt,
        "n_predict": 1,
        "temperature": 0.0,
        "n_probs": 10,
        "cache_prompt": true,
    });

    static HTTP_CLIENT: std::sync::LazyLock<Option<reqwest::blocking::Client>> =
        std::sync::LazyLock::new(|| {
            reqwest::blocking::Client::builder()
                .timeout(Duration::from_millis(1500))
                .build()
                .ok()
        });

    let mut raw_probs = (0.0, 0.0, 0.0, 0.0);
    if let Some(client) = HTTP_CLIENT.as_ref()
        && let Ok(resp) = client.post(&endpoint).json(&body).send()
        && resp.status().is_success()
        && let Ok(val) = resp.json::<serde_json::Value>()
    {
        raw_probs = extract_token_probs(&val);
    }

    let (mut prob_a, mut prob_b, mut prob_c, mut prob_d) = raw_probs;
    if prob_a + prob_b + prob_c + prob_d <= 0.0 {
        // LLM 失敗或無單一候選命中時，由先驗分佈平權做基底
        prob_a = 1.0;
        prob_b = 1.0;
        prob_c = 1.0;
        prob_d = 1.0;
    }

    // 先驗加權計算
    let prior_a = (0.6 + 0.5 * input.boldness)
        * if input.balance_score >= 2 {
            1.4
        } else if input.balance_score < 0 {
            0.5
        } else {
            1.0
        };

    let prior_b = (0.6 + 0.6 * input.sensitivity)
        * if input.balance_score < 0 {
            1.6
        } else {
            0.9
        };

    let prior_c = 0.6 + 0.5 * input.orderliness;

    let prior_d = if !input.open_obligations.is_empty() {
        1.5 + 0.4 * input.orderliness
    } else {
        0.7
    };

    let weight_a = prob_a * prior_a;
    let weight_b = prob_b * prior_b;
    let weight_c = prob_c * prior_c;
    let weight_d = prob_d * prior_d;

    let mut best_choice = NpcStanceChoice::Accept;
    let mut best_weight = weight_a;

    if weight_b > best_weight {
        best_choice = NpcStanceChoice::Refuse;
        best_weight = weight_b;
    }
    if weight_c > best_weight {
        best_choice = NpcStanceChoice::Deflect;
        best_weight = weight_c;
    }
    if weight_d > best_weight {
        best_choice = NpcStanceChoice::Bargain;
        best_weight = weight_d;
    }

    let total_weight = weight_a + weight_b + weight_c + weight_d;
    let confidence = if total_weight > 0.0 {
        best_weight / total_weight
    } else {
        0.25
    };

    let prompt_directive = match best_choice {
        NpcStanceChoice::Accept => {
            "【已拍板決策：A 坦誠正面回應】你已決定正面回應對方；台詞限 35 字內，直接切入重點。".to_string()
        }
        NpcStanceChoice::Refuse => {
            "【已拍板決策：B 拒絕並防備】你已決定拒絕對方並保持距離；台詞限 35 字內，禁止在台詞中答應對方請求。".to_string()
        }
        NpcStanceChoice::Deflect => {
            "【已拍板決策：C 簡短敷衍】你正忙於手頭事務，決定簡短敷衍對方；台詞限 30 字內。".to_string()
        }
        NpcStanceChoice::Bargain => {
            let ob_desc = if input.open_obligations.is_empty() {
                "手頭雜務".to_string()
            } else {
                input.open_obligations.join("、")
            };
            format!("【已拍板決策：D 條件交換】你手頭有未完義務（{ob_desc}），決定提出交換條件或請對方搭把手；台詞限 35 字內。")
        }
    };

    let latency_ms = start_time.elapsed().as_millis() as u64;

    NpcStanceDecision {
        choice: best_choice,
        confidence,
        prompt_directive,
        latency_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_prior_favors_bargain_when_obligations_exist() {
        let obs = vec!["repair_tool".to_string()];
        let input = NpcDecisionInput {
            npc_name: "老張",
            boldness: 0.2,
            sensitivity: 0.2,
            orderliness: 0.9,
            familiarity_score: 5,
            balance_score: 0,
            open_obligations: &obs,
            current_activity: "修補漁網",
            action_kind: "Talk",
            player_utterance: "你好",
        };
        let dec = score_npc_interaction_stance(&input);
        assert_eq!(dec.choice, NpcStanceChoice::Bargain);
        assert!(dec.prompt_directive.contains("repair_tool"));
    }

    #[test]
    fn deterministic_prior_favors_refuse_when_grudge_and_sensitive() {
        let input = NpcDecisionInput {
            npc_name: "阿勝",
            boldness: 0.1,
            sensitivity: 0.9,
            orderliness: 0.1,
            familiarity_score: 1,
            balance_score: -5,
            open_obligations: &[],
            current_activity: "避雨中",
            action_kind: "Borrow",
            player_utterance: "借點錢",
        };
        let dec = score_npc_interaction_stance(&input);
        assert_eq!(dec.choice, NpcStanceChoice::Refuse);
        assert!(dec.prompt_directive.contains("拒絕並防備"));
    }

    #[test]
    fn parses_top_logprobs_correctly() {
        let sample = serde_json::json!({
            "choices": [{
                "text": "A",
                "logprobs": {
                    "content": [{
                        "top_logprobs": [
                            { "token": "A", "logprob": -0.1 },
                            { "token": "B", "logprob": -2.5 }
                        ]
                    }]
                }
            }]
        });
        let (pa, pb, pc, pd) = extract_token_probs(&sample);
        assert!(pa > pb);
        assert_eq!(pc, 0.0);
        assert_eq!(pd, 0.0);
    }
}
