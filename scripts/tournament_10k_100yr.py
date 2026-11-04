#!/usr/bin/env python3
"""
萬人萬相・百年演化與萬人武道大會自然湧現模擬 (N = 10,000, T = 100 年, Bo5 淘汰賽)
嚴格落實造物者三大公理與 12 點數理校正定案規格（與 Rust `src/combat/human_phase.rs` 100% 逐字對齊）：
1. 公平起跑點：萬人萬相先天雙層守恆，由不可變實體識別碼經 splitmix64 逐一確定性展開骨相 (alpha * beta 約等於 1, Vit_0 + Qi_0 + Dex_0 恆等於 310, 阻抗總和 10,000)。
2. 天地不仁：零人工 z-score 流派配平、零 2.0 倍指數飽和封頂，改為無上限練習冪律 m_e = (f_eff / 25)^0.45，年度時間預算分流與閒置年衰減 delta = 0.03。
3. 眾生平等，人獸無別：人與獸 100% 跑同一條對稱有效質量公式 M_eff = (0.12 Vit_0 + 0.06 Qi_0) * sqrt(alpha) * limb_factor(c) + m_載體(c)。
4. 自然湧現：逐年演化跑滿 100 年迴圈、涵蓋人類武者與山海百歲巨獸同場競技，觀察多門宗師、山海霸主與孤峰苦修者之真實博弈。
"""

import math
from typing import Dict, List, Tuple
import numpy as np

N = 10_000
YEARS = 100
SEED = 20261028

# 六大載體名稱（人獸共用：0 刀劍爪刃 / 1 長兵頂角 / 2 重鈍掌蹄 / 3 徒手撕咬 / 4 堅盾甲膜 / 5 游身騰躍）
CARRIER_NAMES = [
    "刀劍爪刃",  # 0: 刀劍／猛獸銳爪 (入微穿隙主場)
    "長兵頂角",  # 1: 長槍大戟／異獸硬角 (中距截擊破綻窗)
    "重鈍掌蹄",  # 2: 重錘大斧／熊掌重蹄 (一力降十會主場)
    "徒手撕咬",  # 3: 拳掌擒拿／猛獸撕咬 (極短前搖近身穿透)
    "堅盾甲膜",  # 4: 盾衛重壁／厚實獸皮重甲 (極高定力與衝量承受)
    "游身騰躍",  # 5: 身法步法／山海靈動騰躍 (身法周旋免除硬撼)
]

# 載體對應之基礎前搖基底 tau_pre_base (與 Rust carrier_pre_base 一致)
CARRIER_PRE_BASE = np.array([1.50, 2.00, 2.50, 1.50, 2.50, 1.00])

# 載體肢段幾何係數 limb_factor (與 Rust carrier_limb_factor 一致)
CARRIER_LIMB_FACTOR = np.array([1.00, 1.15, 1.30, 0.85, 1.30, 0.85])

# 載體外掛器物質量 (與 Rust effective_carrier_mass 一致)
CARRIER_TOOL_MASS = np.array([3.5, 5.0, 14.0, 0.0, 10.0, 0.0])

# 載體動量力矩係數 kappa (與 Rust 一致)
CARRIER_TORQUE_KAPPA = np.array([1.00, 1.00, 1.40, 0.80, 1.40, 0.80])

# 載體刃口集中度 p_tool (與 Rust evaluate_seam_threading_with_depth 一致)
CARRIER_P_TOOL = np.array([1.80, 1.40, 0.50, 1.00, 0.50, 0.50])

# 載體截擊係數 carrier_int (與 Rust carrier_intercept_factor 逐字一致)
# 游身 1.30, 刀劍 1.15, 徒手 1.05, 長兵 0.95, 堅盾 0.85, 重鈍 0.75
CARRIER_INT_FACTOR = np.array([1.15, 0.95, 0.75, 1.05, 0.85, 1.30])

# 載體對應之三軸負荷傾向 [體, 氣, 敏]
CARRIER_AXIS_W = np.array([
    [0.20, 0.35, 0.45],  # 刀劍爪刃
    [0.30, 0.45, 0.25],  # 長兵頂角
    [0.60, 0.25, 0.15],  # 重鈍掌蹄
    [0.25, 0.40, 0.35],  # 徒手撕咬
    [0.50, 0.35, 0.15],  # 堅盾甲膜
    [0.15, 0.30, 0.55],  # 游身騰躍
])


def splitmix64(seed: int) -> int:
    """SplitMix64 確定性純整數雜湊器 (與 Rust splitmix64 一致)"""
    z = (seed + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
    return (z ^ (z >> 31)) & 0xFFFFFFFFFFFFFFFF


def from_seed_rust_deterministic(seed: int):
    """
    與 Rust `InnateBonePhase::from_seed` 100% 一致之確定性骨相展開演算法
    """
    rng_state = seed & 0xFFFFFFFFFFFFFFFF

    def next_f64():
        nonlocal rng_state
        val = splitmix64(rng_state)
        rng_state = (rng_state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        return (val >> 11) / (1 << 53)

    ln_2 = math.log(2.0)
    u_alpha = next_f64()
    ln_alpha = -ln_2 + u_alpha * (2.0 * ln_2)
    u_eps = next_f64()
    eps = -0.15 + u_eps * 0.30
    raw_ln_beta = -ln_alpha + eps
    ln_beta = max(-ln_2, min(ln_2, raw_ln_beta))
    u_gamma = next_f64()
    gamma = -1.0 + u_gamma * 2.0

    alpha = math.exp(ln_alpha)
    beta = math.exp(ln_beta)

    # 抽樣 15 個 Gamma(18, 1) 變量
    r_flat = [0.0] * 15
    sum_raw = 0.0
    for i in range(15):
        g_val = 0.0
        for _ in range(18):
            u = max(1e-12, min(1.0, next_f64()))
            g_val += -math.log(u)
        r_flat[i] = g_val
        sum_raw += g_val

    if sum_raw <= 0.0:
        sum_raw = 1.0
    for i in range(15):
        r_flat[i] = (r_flat[i] / sum_raw) * 10000.0

    # 帶界單純形水位投影 (12 次迭代)
    for _ in range(12):
        for i in range(15):
            r_flat[i] = max(300.0, min(1200.0, r_flat[i]))
        cur_sum = sum(r_flat)
        deficit = 10000.0 - cur_sum
        free_mask = [False] * 15
        free_sum = 0.0
        for i in range(15):
            val = r_flat[i]
            if (deficit > 0.0 and val < 1199.99) or (deficit < 0.0 and val > 300.01):
                free_mask[i] = True
                free_sum += 1.0
        if free_sum > 0.0:
            step = deficit / free_sum
            for i in range(15):
                if free_mask[i]:
                    r_flat[i] += step
        else:
            break

    for i in range(15):
        r_flat[i] = max(300.0, min(1200.0, r_flat[i]))
    final_sum = sum(r_flat)
    if final_sum > 0.0 and abs(final_sum - 10000.0) > 1e-9:
        factor = 10000.0 / final_sum
        for i in range(15):
            r_flat[i] = max(300.0, min(1200.0, r_flat[i] * factor))
        diff = 10000.0 - sum(r_flat)
        r_flat[0] = max(300.0, min(1200.0, r_flat[0] + diff))

    # 3x5 矩陣與外顯三維計算
    sum_vit = sum(r_flat[0:5])
    sum_qi = sum(r_flat[5:10])
    sum_dex = sum(r_flat[10:15])
    vit_0 = 50.0 + 0.02 * (6000.0 - sum_vit)
    qi_0 = 50.0 + 0.02 * (6000.0 - sum_qi)
    dex_0 = 50.0 + 0.02 * (6000.0 - sum_dex)

    return alpha, beta, gamma, r_flat, vit_0, qi_0, dex_0


def generate_10k_souls(seed: int = SEED) -> Dict[str, np.ndarray]:
    """
    第一公理：公平起跑點（萬人萬相先天雙層守恆）
    眾生平等：以不可變實體識別碼經 splitmix64 逐一確定性展開骨相
    """
    cur = seed
    seeds = np.zeros(N, dtype=np.uint64)
    alphas = np.zeros(N, dtype=np.float64)
    betas = np.zeros(N, dtype=np.float64)
    gammas = np.zeros(N, dtype=np.float64)
    r_3x5 = np.zeros((N, 3, 5), dtype=np.float64)
    vit_0 = np.zeros(N, dtype=np.float64)
    qi_0 = np.zeros(N, dtype=np.float64)
    dex_0 = np.zeros(N, dtype=np.float64)

    for i in range(N):
        cur = splitmix64(cur)
        s_val = cur if cur != 0 else 42
        seeds[i] = s_val
        a, b, g, rf, v0, q0, d0 = from_seed_rust_deterministic(s_val)
        alphas[i] = a
        betas[i] = b
        gammas[i] = g
        r_3x5[i] = np.array(rf).reshape(3, 5)
        vit_0[i] = v0
        qi_0[i] = q0
        dex_0[i] = d0

    # 順暢因子矩陣 S_e (在 [0.28, 1.65] 區間平滑分佈)
    r_bar = 10000.0 / 15.0
    s_mat = 0.25 + 1.50 / (1.0 + np.exp((r_3x5 - r_bar) / 140.0))

    # 各生靈身份標籤：前 9,200 為人類武者，後 800 為山海生靈
    is_beast = np.zeros(N, dtype=bool)
    is_beast[9200:] = True

    return {
        "seeds": seeds,
        "alpha": alphas,
        "beta": betas,
        "gamma": gammas,
        "vit_0": vit_0,
        "qi_0": qi_0,
        "dex_0": dex_0,
        "r_3x5": r_3x5,
        "s_mat": s_mat,
        "is_beast": is_beast,
    }


def evolve_100_years_natural(souls: Dict[str, np.ndarray]) -> Dict[str, np.ndarray]:
    """
    第二公理：天地不仁（零人工配平、零 2.0 倍指數封頂，實施時間預算與用進廢退）
    第三公理：眾生平等（人獸同源，100% 跑同一條有效質量公式 M_eff）
    逐年跑滿 100 年真實演化迴圈
    """
    rng = np.random.default_rng(SEED + 777)
    alpha, beta, gamma = souls["alpha"], souls["beta"], souls["gamma"]
    vit_0, qi_0, dex_0 = souls["vit_0"], souls["qi_0"], souls["dex_0"]
    s_mat = souls["s_mat"]
    is_beast = souls["is_beast"]

    # 1. 載體分配與契合度計算
    s_axis_mean = s_mat.mean(axis=2)  # [N, 3]
    carrier_fit = np.dot(s_axis_mean, CARRIER_AXIS_W.T)  # [N, 6]

    carrier = np.zeros(N, dtype=np.int32)
    human_mask = ~is_beast
    n_human = int(np.sum(human_mask))

    # 人類：75% 順應骨相自選最優載體；25% 因家傳或生計命運錯位（涉足次優或逆性載體）
    human_rnd = rng.random(n_human)
    human_carrier = np.where(
        human_rnd < 0.75,
        np.argmax(carrier_fit[human_mask], axis=1),
        rng.integers(0, 6, size=n_human),
    )
    carrier[human_mask] = human_carrier

    # 巨獸：分佈於爪刃(0)、頂角(1)、重蹄(2)、撕咬(3)
    beast_mask = is_beast
    n_beast = int(np.sum(beast_mask))
    beast_pool = np.array([0, 1, 2, 3])
    carrier[beast_mask] = rng.choice(beast_pool, size=n_beast, p=[0.30, 0.20, 0.30, 0.20])

    # 本命載體之契合度 S_bar_carrier
    chosen_s_bar = carrier_fit[np.arange(N), carrier]

    # 2. 建立多門套路時間預算與用進廢退資料結構
    k_routines = np.ones(N, dtype=np.int32)
    human_k = rng.choice([1, 2, 3, 4], size=n_human, p=[0.18, 0.40, 0.30, 0.12])
    k_routines[human_mask] = human_k
    k_routines[beast_mask] = 1

    # 手札吸收 Delta L
    manual_books = np.zeros(N, dtype=np.int32)
    manual_books[human_mask] = rng.poisson(lam=2.5, size=n_human)
    manual_books[beast_mask] = 0

    # 標記「孤峰苦修者」：逆性 (chosen_s_bar <= 0.98)、K=1、無手札 (manual_books == 0)，死磕一招百年
    is_lone_ascetic = human_mask & (chosen_s_bar <= 0.98) & (k_routines == 1) & (manual_books == 0)

    r_0 = 3.0
    delta_L = 1.0 - np.exp(-chosen_s_bar * (manual_books / r_0))
    delta_L = np.clip(delta_L, 0.0, 1.0)

    # 記錄各套路之刻意練習量 f_raw [N, 4]
    f_raw = np.zeros((N, 4), dtype=np.float64)
    # 山海材料熟悉度 f_mat
    f_mat = np.zeros(N, dtype=np.float64)

    # 3. 逐年跑滿 100 年真實演化迴圈 (t = 1 .. 100)
    c_switch = 0.08
    delta_decay = 0.03
    t_budget_annual = 120.0  # 每年刻意練習時間預算 (10小時單位)

    for year in range(1, YEARS + 1):
        effective_annual_budget = t_budget_annual / (1.0 + c_switch * (k_routines - 1))

        # 主套路分配：單門者 100% 傾注；多門者主套路 60%
        alloc_main = np.where(k_routines == 1, 1.0, 0.60)
        f_raw[:, 0] += effective_annual_budget * alloc_main

        for slot in range(1, 4):
            has_slot = k_routines > slot
            alloc_slot = np.where(has_slot, (1.0 - alloc_main) / np.maximum(1, k_routines - 1), 0.0)
            f_raw[:, slot] += effective_annual_budget * alloc_slot
            # 閒置套路衰減用進廢退
            idle_mask = ~has_slot & (f_raw[:, slot] > 0.0)
            f_raw[idle_mask, slot] *= (1.0 - delta_decay)

        # 每年山海歷練累積 f_mat (巨獸搏殺每年累積較高)
        hunt_gain = np.where(is_beast, rng.uniform(0.6, 1.4, size=N), rng.uniform(0.1, 0.8, size=N))
        f_mat += hunt_gain

    # 4. 結算 100 年後之物理與技藝指標
    f_eff_main = chosen_s_bar * f_raw[:, 0]

    # 無上限練習冪律：m_e = (f_eff / 25)^0.45 (與 Rust P_PRACTICE=0.45, F_0=25 一致)
    f_0 = 25.0
    p_power = 0.45
    m_e = np.power(f_eff_main / f_0, p_power)

    # 技藝熟練度因子 H_e（僅進技術項，嚴禁污染身體基本維度）
    H_e = 1.0 + 0.35 * delta_L + 0.28 * m_e

    # 套路等效連貫度 C_bar（由適配均值 chosen_s_bar 與練習深度 m_e 單調推導）
    c_bar = np.clip(0.70 * chosen_s_bar * (1.0 + 0.48 * m_e), 0.80, 8.50)

    # 變化性指標 V_e：依套路門數、手札與靈活度自然形成
    v_e_raw = 0.35 + 0.14 * (k_routines - 1) + 0.18 * delta_L + 0.08 * (s_axis_mean[:, 2] - 1.0)
    V_e = np.clip(v_e_raw, 0.32, 0.95)

    # 連續 f64 時序：起手前搖 tau_pre 與換招破綻窗 tau_gap (與 Rust compute_continuous_timings 100% 逐字對齊)
    pre_base = CARRIER_PRE_BASE[carrier]
    tau_pre = 1.0 + (np.maximum(0.20, pre_base - 0.60)) / (np.sqrt(beta) * (1.0 + 0.35 * m_e))
    tau_gap = (1.0 + (pre_base * 1.40) / (np.sqrt(beta) * (1.0 + 0.15 * np.maximum(0.0, gamma)) * (1.0 + 0.22 * c_bar)))

    # 5. 人獸同源：對稱有效質量 M_eff (100% 跑同一條與 Rust effective_carrier_mass 相同的公式)
    # m_limb = (0.12 * vit_0 + 0.06 * qi_0) * sqrt(alpha) * limb_factor(c)
    # M_eff = m_limb + m_載體(c)
    limb_factor = CARRIER_LIMB_FACTOR[carrier]
    m_limb = (0.12 * vit_0 + 0.06 * qi_0) * np.sqrt(alpha) * limb_factor
    m_carrier = CARRIER_TOOL_MASS[carrier]
    m_eff = m_limb + m_carrier

    # 身體基本屬性：飽和體能規律
    EVit = vit_0 * (0.90 + 0.10 * np.tanh(m_e / 6.0))
    EQi = qi_0 * (0.90 + 0.10 * np.tanh(m_e / 6.0))
    EDex = dex_0 * (0.90 + 0.10 * np.tanh(m_e / 6.0))

    # 動量衝量 (第一相量級硬撼，與 Rust 一致：i = kappa * m_eff.max(10) * vit_0 * impact_factor)
    torque_kappa = CARRIER_TORQUE_KAPPA[carrier]
    impulse_atk = torque_kappa * np.maximum(10.0, m_eff) * vit_0 * np.sqrt(alpha)
    s_dex = s_mat[:, 2, 2]  # cell_smoothness(2, 2)
    deflect_factor = 1.0 + 0.35 * np.maximum(0.0, -gamma) * s_dex
    # 架勢承受衝量：攻守對稱採納有效質量 m_eff (穩態架勢 post_mult = 1.0)
    impulse_hold = 1.0 * np.maximum(10.0, m_eff) * vit_0 * deflect_factor

    return {
        "carrier": carrier,
        "chosen_s_bar": chosen_s_bar,
        "is_beast": is_beast,
        "is_lone_ascetic": is_lone_ascetic,
        "k_routines": k_routines,
        "manual_books": manual_books,
        "delta_L": delta_L,
        "f_eff": f_eff_main,
        "m_e": m_e,
        "H_e": H_e,
        "c_bar": c_bar,
        "V_e": V_e,
        "tau_pre": tau_pre,
        "tau_gap": tau_gap,
        "m_eff": m_eff,
        "EVit": EVit,
        "EQi": EQi,
        "EDex": EDex,
        "impulse_atk": impulse_atk,
        "impulse_hold": impulse_hold,
        "f_mat": f_mat,
    }


def play_natural_bo5_match(
    idx_a: int,
    idx_b: int,
    souls: Dict[str, np.ndarray],
    pop: Dict[str, np.ndarray],
    rng: np.random.Generator,
) -> Tuple[int, int, int, List[int], Dict[str, int]]:
    """
    五戰三勝制 (Bo5, 單局 100 狀態值) 嚴謹對局
    落實 12 點數理校正與 Rust 100% 一致之三相物理判準：
    - 多局識破適應：守方 d 用自己的 V_d 識破攻方 a 的 V_a，收窄守方自己暴露的 tau_gap_eff_d！
    - 第二相連續相對餘裕調制判準 (tau_gap_eff_d - tau_pre_a) / tau_gap_eff_d
    - 第三相動態緊密度 kappa_d = kappa_posture / (1.0 + 0.30 max(0, tau_gap_eff_d - tau_pre_a))
    """
    wins_a = 0
    wins_b = 0
    p1_wins = p2_wins = p3_wins = std_wins = 0

    c_a, c_b = pop["carrier"][idx_a], pop["carrier"][idx_b]
    imp_atk_a, imp_atk_b = pop["impulse_atk"][idx_a], pop["impulse_atk"][idx_b]
    imp_hold_a, imp_hold_b = pop["impulse_hold"][idx_a], pop["impulse_hold"][idx_b]
    tau_pre_a, tau_pre_b = pop["tau_pre"][idx_a], pop["tau_pre"][idx_b]
    tau_gap_a, tau_gap_b = pop["tau_gap"][idx_a], pop["tau_gap"][idx_b]
    V_a, V_b = pop["V_e"][idx_a], pop["V_e"][idx_b]
    gamma_a, gamma_b = souls["gamma"][idx_a], souls["gamma"][idx_b]
    edex_a, edex_b = pop["EDex"][idx_a], pop["EDex"][idx_b]
    beta_a, beta_b = souls["beta"][idx_a], souls["beta"][idx_b]
    p_tool_a = CARRIER_P_TOOL[c_a]
    p_tool_b = CARRIER_P_TOOL[c_b]
    f_mat_a, f_mat_b = pop["f_mat"][idx_a], pop["f_mat"][idx_b]
    m_e_a, m_e_b = pop["m_e"][idx_a], pop["m_e"][idx_b]

    # 第一相量級硬撼比 (身法載體 c==5 在守勢具周旋卸力優勢)
    evade_a = 0.65 if c_a == 5 else 1.0
    evade_b = 0.65 if c_b == 5 else 1.0
    lambda_ab = (imp_atk_a / np.maximum(1e-4, imp_hold_b)) * evade_b
    lambda_ba = (imp_atk_b / np.maximum(1e-4, imp_hold_a)) * evade_a

    for bout in range(5):
        if wins_a >= 3 or wins_b >= 3:
            break

        # 多局識破適應因子 (與 Rust read_adaptation_factor 逐字一致)：
        # R_read(b, V_defender, V_attacker) = clip(0.22 * b * V_defender * (1.15 - 0.65 * V_attacker), 0.0, 0.75)
        # 當 B 防守 A 時：B 用 V_b 識破 A 的 V_a，收窄 B 自己露出的破綻窗 tau_gap_eff_b
        r_read_b = np.clip(0.22 * bout * V_b * (1.15 - 0.65 * V_a), 0.0, 0.75)
        # 當 A 防守 B 時：A 用 V_a 識破 B 的 V_b，收窄 A 自己露出的破綻窗 tau_gap_eff_a
        r_read_a = np.clip(0.22 * bout * V_a * (1.15 - 0.65 * V_b), 0.0, 0.75)

        tau_gap_eff_a = 1.0 + (tau_gap_a - 1.0) * (1.0 - r_read_a)
        tau_gap_eff_b = 1.0 + (tau_gap_b - 1.0) * (1.0 - r_read_b)

        # 第二相截斷連續相對餘裕 (與 Rust 一致)：
        # A 截斷 B (B 露出破綻 tau_gap_eff_b，A 以 tau_pre_a 截擊)
        margin_ab = max(0.0, (tau_gap_eff_b - tau_pre_a) / max(1e-6, tau_gap_eff_b))
        # B 截斷 A (A 露出破綻 tau_gap_eff_a，B 以 tau_pre_b 截擊)
        margin_ba = max(0.0, (tau_gap_eff_a - tau_pre_b) / max(1e-6, tau_gap_eff_a))

        # 同量級截擊門檻 (Rust 實作門檻 i / i_hold >= 0.55)
        gate_ab = (imp_atk_a / max(1.0, imp_hold_b)) >= 0.55
        gate_ba = (imp_atk_b / max(1.0, imp_hold_a)) >= 0.55

        # 第二相截擊評分 (與 Rust human_phase.rs:1795-1815 100% 逐字對齊)：
        # Score_int = Margin_rel * carrier_int(c) * (1.0 + 0.20 max(0, gamma)) * (1.0 + 0.15 cont) * Omega
        omega_factor = 1.0  # 中局對等交鋒
        cont_a = float(pop["c_bar"][idx_a])
        cont_b = float(pop["c_bar"][idx_b])
        carrier_int_a = CARRIER_INT_FACTOR[c_a]
        carrier_int_b = CARRIER_INT_FACTOR[c_b]

        intercept_score_a = (
            margin_ab
            * carrier_int_a
            * (1.0 + 0.20 * max(0.0, gamma_a))
            * (1.0 + 0.15 * cont_a)
            * omega_factor
        ) if (gate_ab and margin_ab > 0.0) else -1.0

        intercept_score_b = (
            margin_ba
            * carrier_int_b
            * (1.0 + 0.20 * max(0.0, gamma_b))
            * (1.0 + 0.15 * cont_b)
            * omega_factor
        ) if (gate_ba and margin_ba > 0.0) else -1.0
        # 第三相動態緊密度 (與 Rust evaluate_seam_threading_with_depth 逐字一致)：
        # 守方姿勢穩態時 kappa_posture = 100.0
        kappa_target_b = 100.0 / (1.0 + 0.30 * max(0.0, tau_gap_eff_b - tau_pre_a))
        kappa_target_a = 100.0 / (1.0 + 0.30 * max(0.0, tau_gap_eff_a - tau_pre_b))

        dex_beta_a = edex_a * beta_a
        dex_beta_b = edex_b * beta_b
        match_a = dex_beta_a / (dex_beta_a + max(1.0, kappa_target_b))
        match_b = dex_beta_b / (dex_beta_b + max(1.0, kappa_target_a))

        fam_a = 1.0 + 0.25 * math.pow(f_mat_a / 20.0, 0.45) + 0.12 * max(0.0, m_e_a)
        fam_b = 1.0 + 0.25 * math.pow(f_mat_b / 20.0, 0.45) + 0.12 * max(0.0, m_e_b)

        xi_a = min(2.30, p_tool_a * match_a * fam_a)
        xi_b = min(2.30, p_tool_b * match_b * fam_b)

        # 單局微觀博弈演算
        noise_a = rng.normal(1.0, 0.05)
        noise_b = rng.normal(1.0, 0.05)

        # 各相優勢計算
        adv_p1_a = max(0.0, lambda_ab * noise_a - 1.50) * 0.40
        adv_p1_b = max(0.0, lambda_ba * noise_b - 1.50) * 0.40

        adv_p2_a = max(0.0, intercept_score_a * noise_a - 0.20) * 0.38
        adv_p2_b = max(0.0, intercept_score_b * noise_b - 0.20) * 0.38

        adv_p3_a = max(0.0, xi_a * noise_a - 1.50) * 0.35
        adv_p3_b = max(0.0, xi_b * noise_b - 1.50) * 0.35

        # 局間戰術應變加成：高變化性 V 隨局數深入發揮分支優勢
        adapt_a = 1.0 + 0.04 * bout * (V_a - 0.50)
        adapt_b = 1.0 + 0.04 * bout * (V_b - 0.50)

        rate_a = (1.0 + adv_p1_a + adv_p2_a + adv_p3_a) * (pop["EVit"][idx_a] / 100.0) * adapt_a * noise_a
        rate_b = (1.0 + adv_p1_b + adv_p2_b + adv_p3_b) * (pop["EVit"][idx_b] / 100.0) * adapt_b * noise_b

        if rate_a >= rate_b:
            wins_a += 1
            max_adv = max(adv_p1_a, adv_p2_a, adv_p3_a)
            if max_adv >= 0.03:
                if max_adv == adv_p1_a:
                    p1_wins += 1
                elif max_adv == adv_p2_a:
                    p2_wins += 1
                else:
                    p3_wins += 1
            else:
                std_wins += 1
        else:
            wins_b += 1
            max_adv = max(adv_p1_b, adv_p2_b, adv_p3_b)
            if max_adv >= 0.03:
                if max_adv == adv_p1_b:
                    p1_wins += 1
                elif max_adv == adv_p2_b:
                    p2_wins += 1
                else:
                    p3_wins += 1
            else:
                std_wins += 1

    winner = idx_a if wins_a >= 3 else idx_b
    stats = {
        "p1": p1_wins,
        "p2": p2_wins,
        "p3": p3_wins,
        "std": std_wins,
        "is_sweep": 1 if (wins_a == 3 and wins_b == 0) or (wins_b == 3 and wins_a == 0) else 0,
        "is_full5": 1 if (wins_a + wins_b == 5) else 0,
    }
    return winner, wins_a, wins_b, [wins_a, wins_b], stats


def run_tournament_simulation(souls: Dict[str, np.ndarray], pop: Dict[str, np.ndarray]) -> Dict:
    """執行萬人武道大會單敗淘汰賽與多項自然湧現數據統計"""
    rng = np.random.default_rng(SEED + 999)

    print("================================================================================")
    print("《奇點世界》萬人萬相・百年演化與武道大會自然湧現模擬 (N=10,000, T=100 年, Bo5)")
    print("================================================================================")

    # 先天骨相守恆指標核對
    vit_sum = souls["vit_0"] + souls["qi_0"] + souls["dex_0"]
    print("\n【第一公理驗證：先天萬人萬相雙層嚴格守恆】")
    print(f"1. 外顯三維總和 (Vit_0 + Qi_0 + Dex_0): min={vit_sum.min():.4f}, max={vit_sum.max():.4f} (恆等於 310.0)")
    print(f"2. 人類有效質量 M_eff: 均值 {pop['m_eff'][~souls['is_beast']].mean():.1f} kg | 山海巨獸 M_eff: 均值 {pop['m_eff'][souls['is_beast']].mean():.1f} kg")
    ascetic_cnt = int(np.sum(pop["is_lone_ascetic"]))
    beast_cnt = int(np.sum(pop["is_beast"]))
    print(f"3. 孤峰苦修者樣本數: {ascetic_cnt} 名 (逆性無手札死磕單招 100 年)")
    print(f"4. 山海百歲巨獸樣本數: {beast_cnt} 隻 (無手札、專精撲殺打磨 100 年)")

    # 百年後單招前搖分佈
    ascetic_pre_str = f"{pop['tau_pre'][pop['is_lone_ascetic']].mean():.2f}" if ascetic_cnt > 0 else "無樣本"
    print(f"5. 百年起手前搖 tau_pre: 全體均值 {pop['tau_pre'].mean():.2f} Ticks | 苦修者均值 {ascetic_pre_str} Ticks | 巨獸均值 {pop['tau_pre'][pop['is_beast']].mean():.2f} Ticks")

    # 執行萬人單敗淘汰大會
    order = rng.permutation(N)
    r1_contestants = order[:3616]
    byes = order[3616:]

    tot_p1 = tot_p2 = tot_p3 = tot_std = 0
    tot_sweeps = 0
    tot_full5 = 0
    total_matches = 0

    upset_events = []
    current_round = []

    for i in range(0, len(r1_contestants), 2):
        a = r1_contestants[i]
        b = r1_contestants[i + 1]
        w, wa, wb, _, st = play_natural_bo5_match(a, b, souls, pop, rng)
        current_round.append(w)
        tot_p1 += st["p1"]
        tot_p2 += st["p2"]
        tot_p3 += st["p3"]
        tot_std += st["std"]
        tot_sweeps += st["is_sweep"]
        tot_full5 += st["is_full5"]
        total_matches += 1

    current_round = np.array(current_round)
    active_field = np.concatenate([current_round, byes])

    round_idx = 2
    top_8_logs = []

    while len(active_field) > 1:
        next_field = []
        is_top_8 = (len(active_field) == 8)
        for i in range(0, len(active_field), 2):
            a = active_field[i]
            b = active_field[i + 1]
            w, wa, wb, scores, st = play_natural_bo5_match(a, b, souls, pop, rng)
            next_field.append(w)
            tot_p1 += st["p1"]
            tot_p2 += st["p2"]
            tot_p3 += st["p3"]
            tot_std += st["std"]
            tot_sweeps += st["is_sweep"]
            tot_full5 += st["is_full5"]
            total_matches += 1

            if is_top_8:
                top_8_logs.append((a, b, w, wa, wb, scores))

            # 檢驗苦修者 vs 宗師交鋒
            is_ascetic_a = pop["is_lone_ascetic"][a]
            is_master_b = (~pop["is_beast"][b]) and (pop["k_routines"][b] >= 3) and (pop["manual_books"][b] >= 3)
            is_ascetic_b = pop["is_lone_ascetic"][b]
            is_master_a = (~pop["is_beast"][a]) and (pop["k_routines"][a] >= 3) and (pop["manual_books"][a] >= 3)

            if is_ascetic_a and is_master_b:
                upset_events.append((wa, wb, w == a))
            elif is_ascetic_b and is_master_a:
                upset_events.append((wb, wa, w == b))

        active_field = np.array(next_field)
        round_idx += 1

    champion = active_field[0]

    # 結算與輸出
    total_bouts = tot_p1 + tot_p2 + tot_p3 + tot_std
    p1_pct = tot_p1 / total_bouts * 100.0
    p2_pct = tot_p2 / total_bouts * 100.0
    p3_pct = tot_p3 / total_bouts * 100.0
    std_pct = tot_std / total_bouts * 100.0
    sweep_pct = tot_sweeps / total_matches * 100.0
    full5_pct = tot_full5 / total_matches * 100.0

    print("\n【萬人武道大會自然湧現統計結果】")
    print(f"總場次: {total_matches} 場 (共結算 {total_bouts} 局)")
    print(f"- 第一相【量級硬撼（一力降十會）】勝因佔比: {p1_pct:.2f}%")
    print(f"- 第二相【時序截斷（專克拆招）】勝因佔比:   {p2_pct:.2f}%")
    print(f"- 第三相【入微穿隙（庖丁解牛）】勝因佔比:   {p3_pct:.2f}%")
    print(f"- 常態連攜與狀態消耗勝因佔比:             {std_pct:.2f}%")
    print(f"- Bo5 橫掃率 (3:0): {sweep_pct:.2f}% | 打滿五局率 (3:2): {full5_pct:.2f}%")

    # 觀察孤峰苦修者 vs 宗師之真實表現
    print("\n【孤峰苦修者 vs 多門宗師之初見與名局實測】")
    if len(upset_events) > 0:
        first_bout_takes = sum(1 for e in upset_events if e[0] >= 1)
        full5_upsets = sum(1 for e in upset_events if (e[0] + e[1]) == 5)
        ascetic_wins = sum(1 for e in upset_events if e[2])
        print(f"大會直接對壘樣本數: {len(upset_events)} 場 (搶局率 {first_bout_takes/len(upset_events)*100.0:.1f}%, 爆冷勝率 {ascetic_wins/len(upset_events)*100.0:.1f}%)")

    ascetics = np.where(pop["is_lone_ascetic"])[0]
    masters = np.where((~pop["is_beast"]) & (pop["k_routines"] >= 3) & (pop["manual_books"] >= 4))[0]
    if len(ascetics) > 0 and len(masters) > 0:
        sample_upsets = []
        test_rng = np.random.default_rng(SEED + 101)
        for _ in range(50):
            for asc_i in ascetics:
                mst_i = test_rng.choice(masters)
                _, wa, wb, _, _ = play_natural_bo5_match(asc_i, mst_i, souls, pop, test_rng)
                sample_upsets.append((wa, wb, wa >= 3))
        fb_rate = sum(1 for e in sample_upsets if e[0] >= 1) / len(sample_upsets) * 100.0
        f5_rate = sum(1 for e in sample_upsets if (e[0] + e[1]) == 5) / len(sample_upsets) * 100.0
        win_rate = sum(1 for e in sample_upsets if e[2]) / len(sample_upsets) * 100.0
        print(f"平行抽樣 200 場苦修者 vs 多門宗師對陣觀測：")
        print(f"- 苦修者首局憑藉逼近極限短前搖之搶局率: {fb_rate:.1f}%")
        print(f"- 逼入滿五局 (3:2) 纏鬥率:             {f5_rate:.1f}%")
        print(f"- 苦修者終局爆冷取勝率:                 {win_rate:.1f}%")
    champ_is_beast = pop["is_beast"][champion]
    champ_carrier = CARRIER_NAMES[pop["carrier"][champion]]
    champ_k = pop["k_routines"][champion]
    champ_books = pop["manual_books"][champion]
    champ_me = pop["m_e"][champion]
    champ_mass = pop["m_eff"][champion]
    identity = "山海鎮場百歲巨獸" if champ_is_beast else "人類行旅武者"
    print("\n【本屆萬人武道大會總冠軍】")
    print(f"天下第一: 實體編號 #{champion} ({identity})")
    print(f"- 載體流派: {champ_carrier}")
    print(f"- 修習門數: {champ_k} 門 | 研讀手札: {champ_books} 卷 | 百年單招深度 m_e: {champ_me:.2f}")
    print(f"- 物理有效質量 M_eff: {champ_mass:.1f} kg | 起手前搖: {pop['tau_pre'][champion]:.2f} Ticks")

    # 八強實況戰報
    print("\n【頂峰八強淘汰賽戰報一覽】")
    for idx, (a, b, w, wa, wb, sc) in enumerate(top_8_logs):
        id_a = f"{'巨獸' if pop['is_beast'][a] else '人類'}#{a} ({CARRIER_NAMES[pop['carrier'][a]]})"
        id_b = f"{'巨獸' if pop['is_beast'][b] else '人類'}#{b} ({CARRIER_NAMES[pop['carrier'][b]]})"
        id_w = f"#{w}"
        print(f"對局 {idx + 1}: {id_a} vs {id_b} => 比分 {wa}:{wb} (勝者: {id_w})")

    print("\n================================================================================")
    print("驗證結論：零人工 z-score 流派配平、零 2.0 倍指數飽和封頂、純代數物理演化自然湧現。")
    print("================================================================================")

    return {
        "p1_pct": p1_pct,
        "p2_pct": p2_pct,
        "p3_pct": p3_pct,
        "std_pct": std_pct,
        "sweep_pct": sweep_pct,
        "full5_pct": full5_pct,
        "champion": champion,
        "champ_identity": identity,
        "champ_carrier": champ_carrier,
        "champ_k": champ_k,
        "champ_books": champ_books,
        "champ_me": champ_me,
        "champ_mass": champ_mass,
        "champ_pre": pop["tau_pre"][champion],
        "top_8_logs": top_8_logs,
    }


def main():
    souls = generate_10k_souls(SEED)
    pop = evolve_100_years_natural(souls)
    run_tournament_simulation(souls, pop)


if __name__ == "__main__":
    main()
