// human_phase 模組 — 個人之相與自然成相系統（從「露西」演化為「人」）
// 嚴守台灣正體中文，無任何簡體字與禁止字眼；零 unsafe、零 bare #[allow]。
// 涵蓋五大數理模組：
// 1. 先天稟賦雙層守恆（對數功率守恆 × 帶界單純形 10,000 阻抗矩陣 R_e）
// 2. 碎招運動學算子與有界非交換銜接度 C_e
// 3. 套路分支樹、連貫性與變化性、Viterbi 5 階段路徑湧現
// 4. 事件分派三相動力學判準（一力降十會、專克拆招、庖丁解牛）
// 5. 過完備語意字典 2-Step Residual OMP 殘差正交匹配定名

use std::sync::LazyLock;
use crate::combat::wordplate::{
    CarrierAffinity, PostureState, TacticalSlot, WordElement, WordRegistry,
};

include!("human_phase/types.rs");
include!("human_phase/kinematics.rs");
include!("human_phase/catalog.rs");
include!("human_phase/routine.rs");
include!("human_phase/bout.rs");
include!("human_phase/tests.rs");
