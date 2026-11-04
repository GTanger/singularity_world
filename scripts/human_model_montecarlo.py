#!/usr/bin/env python3
"""
《奇點世界》「人版（2026-10-28）」個人之相與自然成相系統 — 蒙地卡羅數理校準與驗證腳本
固定隨機種子：20261028，樣本數 N = 10,000 個 soul_seed。
驗證項目：
1. 模組一：對數空間功率守恆 (alpha * beta) 與 3x5 運動鏈阻抗矩陣 R_e 帶界單純形投影 (總和 10,000 恆等守恆)。
2. 模組二：輕巧快招 / 中堅連招 / 沉猛重招 在 150 歲宗師狀態下的換招破綻窗分佈（確保「快招難拆、重招留隙、玉女克全真」成立）。
3. 模組三：5 階段（起承轉協合）Viterbi 最優套路湧現分佈（驗證「制度教法，個體成相」：同一本教材在 10,000 人身上自然分化出 >200 種個人套路）。
"""

from collections import Counter
import numpy as np


def run_verification(seed: int = 20261028, n_samples: int = 10000) -> None:
    rng = np.random.default_rng(seed)

    # 1. 模組一：對數功率守恆 (alpha, beta) 與 3x5 帶界單純形阻抗矩陣 R_e
    ln_alpha = rng.uniform(-np.log(2.0), np.log(2.0), size=n_samples)
    eps = rng.uniform(-0.15, 0.15, size=n_samples)
    ln_beta = np.clip(-ln_alpha + eps, -np.log(2.0), np.log(2.0))
    alpha = np.exp(ln_alpha)
    beta = np.exp(ln_beta)

    raw_dir = rng.gamma(shape=18.0, scale=1.0, size=(n_samples, 15))
    raw_dir /= raw_dir.sum(axis=1, keepdims=True)
    r_mat = raw_dir * 10000.0
    for _ in range(10):
        r_mat = np.clip(r_mat, 300.0, 1200.0)
        deficit = 10000.0 - r_mat.sum(axis=1, keepdims=True)
        free = ((r_mat > 300.0) & (r_mat < 1200.0)).astype(float)
        r_mat += deficit * (free / np.maximum(free.sum(axis=1, keepdims=True), 1.0))

    r_3x5 = r_mat.reshape(n_samples, 3, 5)
    r_bar = 10000.0 / 15.0
    tau_r = 140.0

    print("=== [模組一] 先天雙層守恆驗證 ===")
    print(f"R_e 總和最大代數誤差: {np.max(np.abs(r_mat.sum(axis=1) - 10000.0)):.8f}")
    print(f"R_e 範圍: min={r_mat.min():.1f}, max={r_mat.max():.1f}, std={r_mat.std():.1f}")
    boundary_rate = ((r_mat <= 300.01) | (r_mat >= 1199.99)).mean() * 100.0
    print(f"R_e 邊界截斷率 ([300, 1200]): {boundary_rate:.2f}%")
    print(
        f"alpha * beta 功率乘積: mean={(alpha * beta).mean():.3f}, "
        f"min={(alpha * beta).min():.3f}, max={(alpha * beta).max():.3f}"
    )

    # 2. 模組二：多量級碎招破綻窗分佈驗證
    s_cells = 0.25 + 1.50 / (1.0 + np.exp((r_3x5 - r_bar) / tau_r))
    s_best = np.sort(s_cells[:, :, 2], axis=1)[:, -1]
    c_master = 0.88 * s_best * 2.0
    denom = 1.0 + 0.55 * np.sqrt(beta) * c_master

    print("\n=== [模組二] 150 歲宗師換招破綻窗 (Ticks) 分佈 ===")
    for label, base_ticks in [
        ("輕巧快招 (原始空檔 4 Ticks)", 4.0),
        ("中堅連招 (原始空檔 7 Ticks)", 7.0),
        ("沉猛重招 (原始空檔 10 Ticks)", 10.0),
    ]:
        gap_raw = base_ticks / denom
        gap_ticks = np.ceil(np.maximum(1.0, gap_raw)).astype(int)
        dist = {int(k): round(float((gap_ticks == k).mean() * 100.0), 1) for k in sorted(set(gap_ticks))}
        print(f"  {label}: {dist}%")

    # 3. 模組三：5 階段（起承轉協合）Viterbi 同法不同相湧現驗證
    cand_weights = np.array([
        [0.80, 0.10, 0.10],  # 0: 重體勢
        [0.50, 0.40, 0.10],  # 1: 沉氣勢
        [0.10, 0.80, 0.10],  # 2: 純氣勢 (教材標準示範)
        [0.10, 0.40, 0.50],  # 3: 流轉勢
        [0.10, 0.10, 0.80],  # 4: 靈敏勢
    ])
    k_mat = np.exp(-0.15 * (np.arange(5)[:, None] - np.arange(5)[None, :]) ** 2)

    chains_5 = []
    for i in range(n_samples):
        s_mat = np.zeros((5, 5))
        h_mat = np.ones((5, 5)) * 1.20
        h_mat[:, 2] += 0.35 * 0.40  # 教材標準動作引導加成
        for v in range(5):
            r_cand = cand_weights @ r_3x5[i, :, v]
            s_mat[v] = 0.25 + 1.50 / (1.0 + np.exp((r_cand - r_bar) / tau_r))
            habit_bonus = 0.65 * (1.0 - np.exp(-(s_mat[v] ** 2) / 1.2))
            h_mat[v] += habit_bonus
        dp = np.log(s_mat[0] * h_mat[0])
        parent = np.zeros((5, 5), dtype=int)
        for v in range(1, 5):
            scores = dp[:, None] + np.log(k_mat * s_mat[v][None, :] * h_mat[v][None, :])
            parent[v] = np.argmax(scores, axis=0)
            dp = np.max(scores, axis=0)
        best_last = int(np.argmax(dp))
        path = [0] * 5
        path[4] = best_last
        for v in range(4, 0, -1):
            path[v - 1] = int(parent[v, path[v]])
        chains_5.append(tuple(path))

    c5 = Counter(chains_5)
    print("\n=== [模組三] 5 階段（起承轉協合）同法不同相湧現分佈 ===")
    print(f"單一教材在 {n_samples} 名角色身上湧現之相異套路總數: {len(c5)} 種")
    print(f"完全照搬教材原式 (2,2,2,2,2) 比例: {c5[(2, 2, 2, 2, 2)] / n_samples * 100.0:.2f}%")
    print(f"因個人骨相代償而走出自創變體比例: {(1.0 - c5[(2, 2, 2, 2, 2)] / n_samples) * 100.0:.2f}%")
    print("前 8 名套路分佈 (%):", [(k, round(cnt / n_samples * 100.0, 2)) for k, cnt in c5.most_common(8)])


if __name__ == "__main__":
    run_verification()
