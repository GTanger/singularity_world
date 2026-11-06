
// ==========================================
// 碎招知識庫建立
// ==========================================

static GLOBAL_MOTION_CATALOG: LazyLock<Vec<ElementaryMotionDef>> =
    LazyLock::new(build_elementary_motion_catalog);

fn build_elementary_motion_catalog() -> Vec<ElementaryMotionDef> {
    let mut catalog = Vec::with_capacity(180);

    let carriers = [
        CarrierAffinity::Unarmed,
        CarrierAffinity::Blade,
        CarrierAffinity::Pole,
        CarrierAffinity::Blunt,
        CarrierAffinity::Shield,
        CarrierAffinity::Footwork,
    ];

    // 為每種載體在 5 個階段各提供 3 個候選碎招（martial motions, craft_domain: None）
    for carrier in carriers {
        for stage in 0..5 {
            for variant in 0..3 {
                let name = match (carrier, stage, variant) {
                    // 徒手拳掌
                    (CarrierAffinity::Unarmed, 0, 0) => "沉腰蹲步",
                    (CarrierAffinity::Unarmed, 0, 1) => "虛步亮掌",
                    (CarrierAffinity::Unarmed, 0, 2) => "提膝側身",
                    (CarrierAffinity::Unarmed, 1, 0) => "蹬地躍起",
                    (CarrierAffinity::Unarmed, 1, 1) => "探馬前衝",
                    (CarrierAffinity::Unarmed, 1, 2) => "擰腰推肘",
                    (CarrierAffinity::Unarmed, 2, 0) => "貼身上勾拳",
                    (CarrierAffinity::Unarmed, 2, 1) => "順水推舟",
                    (CarrierAffinity::Unarmed, 2, 2) => "雲手引化",
                    (CarrierAffinity::Unarmed, 3, 0) => "旋身纏抱",
                    (CarrierAffinity::Unarmed, 3, 1) => "錯骨分筋",
                    (CarrierAffinity::Unarmed, 3, 2) => "封門閉氣",
                    (CarrierAffinity::Unarmed, 4, 0) => "雙撞掌擊面",
                    (CarrierAffinity::Unarmed, 4, 1) => "截脈透骨打",
                    (CarrierAffinity::Unarmed, 4, 2) => "崩山重貫頂",

                    // 刀劍鋒芒
                    (CarrierAffinity::Blade, 0, 0) => "踏步斜探",
                    (CarrierAffinity::Blade, 0, 1) => "流光抽刃",
                    (CarrierAffinity::Blade, 0, 2) => "沉腕藏鋒",
                    (CarrierAffinity::Blade, 1, 0) => "順紋上劈",
                    (CarrierAffinity::Blade, 1, 1) => "連環點刺",
                    (CarrierAffinity::Blade, 1, 2) => "貼骨挑筋",
                    (CarrierAffinity::Blade, 2, 0) => "旋刃卸骨",
                    (CarrierAffinity::Blade, 2, 1) => "游刃截擊",
                    (CarrierAffinity::Blade, 2, 2) => "順勢收鋒切入",
                    (CarrierAffinity::Blade, 3, 0) => "纏頭裹腦",
                    (CarrierAffinity::Blade, 3, 1) => "分水穿肋",
                    (CarrierAffinity::Blade, 3, 2) => "翻腕截脈",
                    (CarrierAffinity::Blade, 4, 0) => "斷浪直斬",
                    (CarrierAffinity::Blade, 4, 1) => "驚鴻一瞥刺",
                    (CarrierAffinity::Blade, 4, 2) => "反刃撩雲截",

                    // 長槍大戟
                    (CarrierAffinity::Pole, 0, 0) => "靈蛇探梢",
                    (CarrierAffinity::Pole, 0, 1) => "低樁埋桿",
                    (CarrierAffinity::Pole, 0, 2) => "滑步流端",
                    (CarrierAffinity::Pole, 1, 0) => "順桿遊撥",
                    (CarrierAffinity::Pole, 1, 1) => "如電連刺",
                    (CarrierAffinity::Pole, 1, 2) => "抖腕崩顫",
                    (CarrierAffinity::Pole, 2, 0) => "穿花點穴",
                    (CarrierAffinity::Pole, 2, 1) => "順勢挑浪",
                    (CarrierAffinity::Pole, 2, 2) => "壓桿入懷",
                    (CarrierAffinity::Pole, 3, 0) => "回馬封喉",
                    (CarrierAffinity::Pole, 3, 1) => "攔拿扎胸",
                    (CarrierAffinity::Pole, 3, 2) => "橫掃千軍",
                    (CarrierAffinity::Pole, 4, 0) => "崩槍裂石",
                    (CarrierAffinity::Pole, 4, 1) => "白虹貫日刺",
                    (CarrierAffinity::Pole, 4, 2) => "沉桿鎖喉打",

                    // 重錘鈍器
                    (CarrierAffinity::Blunt, 0, 0) => "沉腰踏步",
                    (CarrierAffinity::Blunt, 0, 1) => "提柄旋身",
                    (CarrierAffinity::Blunt, 0, 2) => "順風掄臂",
                    (CarrierAffinity::Blunt, 1, 0) => "橫掄壓頂",
                    (CarrierAffinity::Blunt, 1, 1) => "沉柄推撞",
                    (CarrierAffinity::Blunt, 1, 2) => "斜掄掃擊",
                    (CarrierAffinity::Blunt, 2, 0) => "借轉崩錘",
                    (CarrierAffinity::Blunt, 2, 1) => "流勢推砸",
                    (CarrierAffinity::Blunt, 2, 2) => "沉底撩撞",
                    (CarrierAffinity::Blunt, 3, 0) => "旋風蕩寇",
                    (CarrierAffinity::Blunt, 3, 1) => "背身掛錘",
                    (CarrierAffinity::Blunt, 3, 2) => "頓地引震",
                    (CarrierAffinity::Blunt, 4, 0) => "傾力重砸",
                    (CarrierAffinity::Blunt, 4, 1) => "震臂崩擊",
                    (CarrierAffinity::Blunt, 4, 2) => "連環翻砸打",

                    // 堅盾重禦
                    (CarrierAffinity::Shield, 0, 0) => "紮步架盾",
                    (CarrierAffinity::Shield, 0, 1) => "斜身合盾",
                    (CarrierAffinity::Shield, 0, 2) => "沉肘立壁",
                    (CarrierAffinity::Shield, 1, 0) => "頂盾前推",
                    (CarrierAffinity::Shield, 1, 1) => "旋盾卸力",
                    (CarrierAffinity::Shield, 1, 2) => "撞門破勢",
                    (CarrierAffinity::Shield, 2, 0) => "盾角削面",
                    (CarrierAffinity::Shield, 2, 1) => "扣盾纏臂",
                    (CarrierAffinity::Shield, 2, 2) => "側身夾擊",
                    (CarrierAffinity::Shield, 3, 0) => "護體反震",
                    (CarrierAffinity::Shield, 3, 1) => "鐵門閉鎖",
                    (CarrierAffinity::Shield, 3, 2) => "滑步引流",
                    (CarrierAffinity::Shield, 4, 0) => "撼山沉擊",
                    (CarrierAffinity::Shield, 4, 1) => "猛推震跌",
                    (CarrierAffinity::Shield, 4, 2) => "碎骨重撞",

                    // 步法身形
                    (CarrierAffinity::Footwork, 0, 0) => "點水起落",
                    (CarrierAffinity::Footwork, 0, 1) => "側行遊弋",
                    (CarrierAffinity::Footwork, 0, 2) => "含胸蓄勢",
                    (CarrierAffinity::Footwork, 1, 0) => "疾步繞身",
                    (CarrierAffinity::Footwork, 1, 1) => "如影隨形",
                    (CarrierAffinity::Footwork, 1, 2) => "踏雪無痕",
                    (CarrierAffinity::Footwork, 2, 0) => "回首折向",
                    (CarrierAffinity::Footwork, 2, 1) => "移形換位",
                    (CarrierAffinity::Footwork, 2, 2) => "穿林脫身",
                    (CarrierAffinity::Footwork, 3, 0) => "欺身突進",
                    (CarrierAffinity::Footwork, 3, 1) => "游離分影",
                    (CarrierAffinity::Footwork, 3, 2) => "轉角牽引",
                    (CarrierAffinity::Footwork, 4, 0) => "疾閃穿背",
                    (CarrierAffinity::Footwork, 4, 1) => "追魂奪魄步",
                    (CarrierAffinity::Footwork, 4, 2) => "踏空定勢",

                    _ => "碎招流形",
                };

                // 設定物理姿態特徵
                let z_base = match stage {
                    0 => -0.5,
                    1 => 0.0,
                    2 => 0.4,
                    3 => 0.1,
                    _ => 0.6,
                };
                let p_in = [
                    z_base + (variant as f64) * 0.1,
                    -0.2 + (variant as f64) * 0.2,
                    0.1 * (stage as f64),
                    (variant as f64) * 0.5,
                ];
                let p_out = [
                    z_base + 0.2 + (variant as f64) * 0.1,
                    0.2 + (variant as f64) * 0.2,
                    0.2 * (stage as f64),
                    (variant as f64) * 0.5 + 0.3,
                ];

                let w_3d = match variant {
                    0 => [0.60, 0.20, 0.20], // 偏體
                    1 => [0.20, 0.60, 0.20], // 偏氣
                    _ => [0.20, 0.20, 0.60], // 偏敏
                };

                let base_torque = match carrier {
                    CarrierAffinity::Blunt | CarrierAffinity::Shield => 1.4,
                    CarrierAffinity::Blade | CarrierAffinity::Pole => 1.0,
                    _ => 0.85,
                };
                let torque = match variant {
                    0 => base_torque * 1.35,
                    1 => base_torque * 1.00,
                    _ => base_torque * 0.80,
                };

                let base_edge = match carrier {
                    CarrierAffinity::Blade => 1.8,
                    CarrierAffinity::Pole => 1.4,
                    CarrierAffinity::Unarmed => 0.95,
                    _ => 0.45,
                };
                let edge = match variant {
                    0 => base_edge * 0.85,
                    1 => base_edge * 1.05,
                    _ => base_edge * 1.30,
                };

                let timing = match variant {
                    0 => (3, 4, 3),
                    1 => (2, 3, 2),
                    _ => (1, 2, 1),
                };

                let counter_fit = match variant {
                    0 => [0.85, 0.25, 0.25], // 克剛猛硬架
                    1 => [0.25, 0.85, 0.25], // 克後撤閃避
                    _ => [0.25, 0.25, 0.85], // 克貼身搶攻
                };

                let mut feat = [0.05f64; 16];
                feat[stage] += 0.40;
                feat[variant + 5] += 0.35;

                catalog.push(ElementaryMotionDef {
                    name: name.to_string(),
                    carrier,
                    craft_domain: None,
                    stage,
                    p_in,
                    p_out,
                    w_3d,
                    torque_kappa: torque,
                    edge_focus_p: edge,
                    timing,
                    counter_fit,
                    feature_16d: feat,
                });
            }
        }
    }

    // 百工專用碎招（採獵 harvest、鍛鑄 forge、庖廚 culinary、行旅 wayfare）
    // 4 個領域 × 5 個階段 × 3 種變體（偏體、偏氣、偏敏）= 60 個百工碎招
    let craft_domains = [
        ("harvest", 0usize, CarrierAffinity::Blade),
        ("forge", 1usize, CarrierAffinity::Blunt),
        ("culinary", 2usize, CarrierAffinity::Blade),
        ("wayfare", 3usize, CarrierAffinity::Footwork),
    ];

    for &(domain, c_idx, default_carrier) in &craft_domains {
        for stage in 0..5 {
            for variant in 0..3 {
                let (name, p_in, p_out, w_3d, torque, edge, c_fit) = match (domain, stage, variant) {
                    // harvest（採獵）
                    ("harvest", 0, 0) => ("巡山識痕", [-0.3, 0.0, 0.2, 0.0], [-0.2, 0.1, 0.2, 0.2], [0.6, 0.2, 0.2], 1.1, 1.2, [0.7, 0.2, 0.3]),
                    ("harvest", 0, 1) => ("聆風辨獸", [-0.2, 0.1, 0.1, 0.1], [-0.1, 0.2, 0.2, 0.3], [0.2, 0.6, 0.2], 0.9, 1.3, [0.3, 0.7, 0.2]),
                    ("harvest", 0, 2) => ("撥草伺伏", [-0.3, -0.1, 0.3, 0.2], [-0.2, 0.0, 0.3, 0.3], [0.2, 0.2, 0.6], 0.8, 1.4, [0.2, 0.3, 0.8]),

                    ("harvest", 1, 0) => ("探跡尋隙", [-0.2, 0.1, 0.2, 0.2], [-0.1, 0.2, 0.3, 0.4], [0.5, 0.3, 0.2], 1.2, 1.3, [0.8, 0.2, 0.2]),
                    ("harvest", 1, 1) => ("踏雪追蹤", [-0.1, 0.2, 0.2, 0.3], [0.0, 0.3, 0.2, 0.5], [0.2, 0.5, 0.3], 0.9, 1.4, [0.2, 0.7, 0.3]),
                    ("harvest", 1, 2) => ("伺機掩近", [-0.2, 0.0, 0.3, 0.3], [-0.1, 0.1, 0.4, 0.5], [0.2, 0.2, 0.6], 0.9, 1.5, [0.3, 0.2, 0.8]),

                    ("harvest", 2, 0) => ("順紋剝解", [-0.1, 0.2, 0.3, 0.4], [0.1, 0.3, 0.1, 0.6], [0.5, 0.2, 0.3], 1.3, 1.5, [0.7, 0.3, 0.2]),
                    ("harvest", 2, 1) => ("尋筋走刃", [0.0, 0.3, 0.2, 0.5], [0.1, 0.4, 0.2, 0.7], [0.2, 0.5, 0.3], 1.0, 1.6, [0.2, 0.8, 0.2]),
                    ("harvest", 2, 2) => ("巧取逆鱗", [-0.1, 0.1, 0.4, 0.5], [0.0, 0.2, 0.3, 0.7], [0.2, 0.3, 0.5], 0.9, 1.7, [0.2, 0.3, 0.8]),

                    ("harvest", 3, 0) => ("剔骨離筋", [0.1, 0.3, 0.1, 0.6], [0.2, 0.2, 0.2, 0.8], [0.6, 0.2, 0.2], 1.4, 1.6, [0.8, 0.2, 0.2]),
                    ("harvest", 3, 1) => ("抽絲剝繭", [0.1, 0.4, 0.2, 0.7], [0.2, 0.3, 0.2, 0.9], [0.2, 0.6, 0.2], 0.9, 1.6, [0.3, 0.7, 0.2]),
                    ("harvest", 3, 2) => ("入理剖膜", [0.0, 0.2, 0.3, 0.7], [0.1, 0.3, 0.2, 0.8], [0.2, 0.2, 0.6], 0.9, 1.8, [0.2, 0.3, 0.8]),

                    ("harvest", 4, 0) => ("收皮納寶", [0.2, 0.2, 0.2, 0.8], [0.0, 0.0, 0.0, 0.0], [0.5, 0.3, 0.2], 1.2, 1.2, [0.7, 0.3, 0.2]),
                    ("harvest", 4, 1) => ("整束歸囊", [0.2, 0.3, 0.2, 0.9], [0.0, 0.0, 0.0, 0.0], [0.2, 0.5, 0.3], 0.9, 1.3, [0.2, 0.8, 0.2]),
                    ("harvest", 4, 2) => ("輕裝收穫", [0.1, 0.3, 0.2, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.2, 0.6], 0.8, 1.4, [0.2, 0.2, 0.8]),

                    // forge（鍛鑄）
                    ("forge", 0, 0) => ("開爐看火", [-0.4, 0.1, 0.0, 0.1], [-0.3, 0.2, 0.1, 0.3], [0.6, 0.2, 0.2], 1.3, 0.8, [0.8, 0.2, 0.2]),
                    ("forge", 0, 1) => ("推橐引風", [-0.3, 0.2, 0.1, 0.2], [-0.2, 0.3, 0.2, 0.3], [0.2, 0.6, 0.2], 1.1, 0.8, [0.3, 0.7, 0.2]),
                    ("forge", 0, 2) => ("觀色候溫", [-0.3, 0.1, 0.2, 0.1], [-0.2, 0.2, 0.2, 0.2], [0.2, 0.3, 0.5], 1.0, 0.9, [0.2, 0.3, 0.8]),

                    ("forge", 1, 0) => ("引鐵入砧", [-0.3, 0.2, 0.1, 0.3], [-0.2, 0.3, 0.2, 0.5], [0.6, 0.2, 0.2], 1.4, 0.9, [0.8, 0.2, 0.2]),
                    ("forge", 1, 1) => ("平鉗定形", [-0.2, 0.3, 0.2, 0.4], [-0.1, 0.3, 0.2, 0.5], [0.2, 0.6, 0.2], 1.2, 0.9, [0.2, 0.7, 0.3]),
                    ("forge", 1, 2) => ("輕點引路", [-0.2, 0.2, 0.3, 0.4], [-0.1, 0.2, 0.3, 0.5], [0.2, 0.3, 0.5], 1.0, 1.1, [0.2, 0.3, 0.8]),

                    ("forge", 2, 0) => ("翻轉受力", [-0.2, 0.3, 0.2, 0.5], [0.0, 0.4, 0.2, 0.7], [0.5, 0.3, 0.2], 1.5, 1.0, [0.8, 0.2, 0.2]),
                    ("forge", 2, 1) => ("旋砧借勢", [-0.1, 0.4, 0.2, 0.6], [0.0, 0.4, 0.3, 0.7], [0.2, 0.5, 0.3], 1.2, 1.0, [0.2, 0.8, 0.2]),
                    ("forge", 2, 2) => ("順紋輕敲", [-0.1, 0.3, 0.3, 0.5], [0.0, 0.3, 0.3, 0.7], [0.2, 0.3, 0.5], 1.1, 1.2, [0.3, 0.2, 0.8]),

                    ("forge", 3, 0) => ("重錘排渣", [0.0, 0.4, 0.2, 0.7], [0.2, 0.2, 0.3, 0.9], [0.7, 0.2, 0.1], 1.6, 1.0, [0.9, 0.2, 0.1]),
                    ("forge", 3, 1) => ("連環疊打", [0.0, 0.4, 0.3, 0.7], [0.1, 0.3, 0.3, 0.8], [0.2, 0.6, 0.2], 1.3, 1.1, [0.3, 0.7, 0.2]),
                    ("forge", 3, 2) => ("密錘封孔", [0.0, 0.3, 0.3, 0.8], [0.1, 0.2, 0.2, 0.9], [0.2, 0.2, 0.6], 1.1, 1.3, [0.2, 0.3, 0.8]),

                    ("forge", 4, 0) => ("淬水成鋒", [0.2, 0.2, 0.3, 0.9], [0.0, 0.0, 0.0, 0.0], [0.5, 0.3, 0.2], 1.3, 1.4, [0.7, 0.3, 0.2]),
                    ("forge", 4, 1) => ("浴火定性", [0.1, 0.3, 0.3, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.6, 0.2], 1.1, 1.3, [0.2, 0.8, 0.2]),
                    ("forge", 4, 2) => ("冷鍛凝刃", [0.1, 0.2, 0.2, 0.9], [0.0, 0.0, 0.0, 0.0], [0.2, 0.2, 0.6], 1.0, 1.5, [0.2, 0.3, 0.8]),

                    // culinary（庖廚）
                    ("culinary", 0, 0) => ("辨氣識味", [-0.2, 0.0, 0.1, 0.1], [-0.1, 0.1, 0.1, 0.2], [0.5, 0.3, 0.2], 0.9, 1.2, [0.7, 0.2, 0.3]),
                    ("culinary", 0, 1) => ("調和五味", [-0.2, 0.1, 0.1, 0.1], [-0.1, 0.2, 0.1, 0.3], [0.2, 0.6, 0.2], 0.8, 1.2, [0.3, 0.7, 0.2]),
                    ("culinary", 0, 2) => ("按刀量材", [-0.2, 0.0, 0.2, 0.2], [-0.1, 0.1, 0.2, 0.3], [0.2, 0.2, 0.6], 0.8, 1.4, [0.2, 0.3, 0.8]),

                    ("culinary", 1, 0) => ("順刀片肉", [-0.1, 0.1, 0.1, 0.2], [0.0, 0.2, 0.2, 0.4], [0.5, 0.2, 0.3], 1.0, 1.5, [0.8, 0.2, 0.2]),
                    ("culinary", 1, 1) => ("走刃分肌", [-0.1, 0.2, 0.1, 0.3], [0.0, 0.3, 0.2, 0.4], [0.2, 0.5, 0.3], 0.9, 1.6, [0.2, 0.8, 0.2]),
                    ("culinary", 1, 2) => ("巧斬軟骨", [-0.1, 0.1, 0.2, 0.3], [0.0, 0.2, 0.3, 0.5], [0.2, 0.2, 0.6], 0.9, 1.7, [0.2, 0.3, 0.8]),

                    ("culinary", 2, 0) => ("候火調鼎", [0.0, 0.2, 0.2, 0.4], [0.1, 0.3, 0.1, 0.6], [0.5, 0.3, 0.2], 1.1, 1.2, [0.8, 0.2, 0.2]),
                    ("culinary", 2, 1) => ("掀蓋順氣", [0.0, 0.3, 0.2, 0.4], [0.1, 0.4, 0.2, 0.6], [0.2, 0.6, 0.2], 0.8, 1.2, [0.2, 0.8, 0.2]),
                    ("culinary", 2, 2) => ("旋杓勻熱", [0.0, 0.2, 0.3, 0.5], [0.1, 0.3, 0.2, 0.6], [0.2, 0.2, 0.6], 0.8, 1.3, [0.3, 0.2, 0.8]),

                    ("culinary", 3, 0) => ("起杓收汁", [0.1, 0.3, 0.1, 0.6], [0.2, 0.2, 0.2, 0.8], [0.6, 0.2, 0.2], 1.2, 1.2, [0.8, 0.2, 0.2]),
                    ("culinary", 3, 1) => ("文火漫煨", [0.1, 0.4, 0.2, 0.6], [0.2, 0.3, 0.2, 0.8], [0.2, 0.6, 0.2], 0.8, 1.2, [0.2, 0.8, 0.2]),
                    ("culinary", 3, 2) => ("引香點綴", [0.1, 0.3, 0.2, 0.7], [0.2, 0.2, 0.3, 0.8], [0.2, 0.2, 0.6], 0.8, 1.4, [0.2, 0.3, 0.8]),

                    ("culinary", 4, 0) => ("合味定鼎", [0.2, 0.2, 0.2, 0.8], [0.0, 0.0, 0.0, 0.0], [0.5, 0.3, 0.2], 1.0, 1.3, [0.7, 0.3, 0.2]),
                    ("culinary", 4, 1) => ("盛盤封香", [0.2, 0.3, 0.2, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.6, 0.2], 0.8, 1.3, [0.2, 0.8, 0.2]),
                    ("culinary", 4, 2) => ("剔骨呈鮮", [0.2, 0.2, 0.3, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.2, 0.6], 0.8, 1.5, [0.2, 0.2, 0.8]),

                    // wayfare（行旅）
                    ("wayfare", 0, 0) => ("觀天辨向", [-0.2, 0.0, 0.3, 0.0], [-0.1, 0.1, 0.4, 0.2], [0.5, 0.3, 0.2], 0.9, 0.8, [0.7, 0.3, 0.2]),
                    ("wayfare", 0, 1) => ("束裝提氣", [-0.2, 0.1, 0.2, 0.1], [-0.1, 0.2, 0.3, 0.2], [0.2, 0.6, 0.2], 0.8, 0.8, [0.3, 0.7, 0.2]),
                    ("wayfare", 0, 2) => ("探風定步", [-0.2, 0.0, 0.3, 0.1], [-0.1, 0.1, 0.4, 0.3], [0.2, 0.2, 0.6], 0.7, 0.9, [0.2, 0.3, 0.8]),

                    ("wayfare", 1, 0) => ("踏草尋徑", [-0.1, 0.1, 0.4, 0.2], [0.0, 0.2, 0.3, 0.4], [0.5, 0.2, 0.3], 1.0, 0.8, [0.8, 0.2, 0.2]),
                    ("wayfare", 1, 1) => ("涉水探石", [-0.1, 0.2, 0.3, 0.2], [0.0, 0.3, 0.3, 0.4], [0.2, 0.5, 0.3], 0.8, 0.9, [0.2, 0.8, 0.2]),
                    ("wayfare", 1, 2) => ("循嶺攀行", [-0.1, 0.1, 0.4, 0.3], [0.0, 0.2, 0.4, 0.5], [0.2, 0.2, 0.6], 0.8, 0.9, [0.2, 0.3, 0.8]),

                    ("wayfare", 2, 0) => ("側身避瘴", [0.0, 0.2, 0.3, 0.4], [0.1, 0.3, 0.2, 0.6], [0.5, 0.3, 0.2], 1.0, 0.9, [0.7, 0.3, 0.2]),
                    ("wayfare", 2, 1) => ("借藤越澗", [0.0, 0.3, 0.3, 0.4], [0.1, 0.4, 0.3, 0.6], [0.2, 0.5, 0.3], 0.9, 0.9, [0.2, 0.8, 0.2]),
                    ("wayfare", 2, 2) => ("旋步卸滑", [0.0, 0.2, 0.4, 0.5], [0.1, 0.3, 0.3, 0.7], [0.2, 0.2, 0.6], 0.8, 1.0, [0.2, 0.2, 0.8]),

                    ("wayfare", 3, 0) => ("勻息長驅", [0.1, 0.3, 0.2, 0.6], [0.2, 0.2, 0.3, 0.8], [0.6, 0.2, 0.2], 1.1, 0.8, [0.8, 0.2, 0.2]),
                    ("wayfare", 3, 1) => ("順風借勢", [0.1, 0.4, 0.3, 0.6], [0.2, 0.3, 0.3, 0.8], [0.2, 0.6, 0.2], 0.8, 0.9, [0.2, 0.8, 0.2]),
                    ("wayfare", 3, 2) => ("換步輕行", [0.1, 0.3, 0.3, 0.7], [0.2, 0.2, 0.4, 0.8], [0.2, 0.2, 0.6], 0.7, 1.0, [0.2, 0.3, 0.8]),

                    ("wayfare", 4, 0) => ("安營定址", [0.2, 0.2, 0.3, 0.8], [0.0, 0.0, 0.0, 0.0], [0.6, 0.2, 0.2], 1.1, 0.8, [0.7, 0.3, 0.2]),
                    ("wayfare", 4, 1) => ("歸途斂息", [0.2, 0.3, 0.3, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.6, 0.2], 0.8, 0.9, [0.2, 0.8, 0.2]),
                    ("wayfare", 4, 2) => ("登高極目", [0.2, 0.2, 0.4, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.2, 0.6], 0.7, 1.0, [0.2, 0.2, 0.8]),

                    _ => ("行旅步法", [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.33, 0.33, 0.34], 1.0, 1.0, [0.33, 0.33, 0.34]),
                };

                let mut feat = [0.03f64; 16];
                feat[12 + c_idx] = 0.80;
                feat[stage] += 0.30;
                feat[variant + 5] += 0.20;

                catalog.push(ElementaryMotionDef {
                    name: name.to_string(),
                    carrier: default_carrier,
                    craft_domain: Some(domain),
                    stage,
                    p_in,
                    p_out,
                    w_3d,
                    torque_kappa: torque,
                    edge_focus_p: edge,
                    timing: (2, 3, 2),
                    counter_fit: c_fit,
                    feature_16d: feat,
                });
            }
        }
    }

    catalog
}

// ==========================================
// 模組三：Viterbi 5 階段最優路徑湧現
// ==========================================

fn find_viterbi_5stages_for_slot(
    bone: &InnateBonePhase,
    slot_opt: Option<TacticalSlot>,
    candidates_by_stage: &[Vec<&ElementaryMotionDef>; 5],
    learned_delta: f64,
    familiarity_count: u32,
) -> (Vec<ElementaryMotionDef>, Vec<usize>, f64) {
    // 動態分配 dp 表，完全支援任意候選數量，杜絕固定陣列越界
    let mut dp = Vec::with_capacity(5);
    for s in 0..5 {
        dp.push(vec![(0.0f64, 0usize); candidates_by_stage[s].len()]);
    }

    // 階段權重與戰術槽偏好加成評估
    let slot_bonus = |slot: TacticalSlot, m: &ElementaryMotionDef| -> f64 {
        match slot {
            TacticalSlot::Opening => {
                let speed_bonus = (3.0 - (m.timing.0 as f64).min(3.0)) * 0.35;
                let agi_bonus = m.w_3d[2] * 0.75;
                let stage_fit = if m.stage <= 1 { m.w_3d[2] * 0.40 } else { m.w_3d[1] * 0.25 };
                1.0 + speed_bonus + agi_bonus + stage_fit
            }
            TacticalSlot::Midgame => {
                let qi_bonus = m.w_3d[1] * 0.95;
                let flow_bonus = if m.timing.0 == 2 { 0.35 } else { 0.0 };
                1.0 + qi_bonus + flow_bonus
            }
            TacticalSlot::Finisher => {
                let torque_b = (m.torque_kappa - 0.8).max(0.0) * 0.85;
                let vit_b = m.w_3d[0] * 1.45;
                let stage_fit = if m.stage >= 2 { m.w_3d[0] * 0.65 } else { 0.0 };
                1.0 + torque_b + vit_b + stage_fit
            }
            TacticalSlot::Defense => {
                let rec_b = (3.0 - (m.timing.2 as f64).min(3.0)) * 0.25;
                let foot_bonus = if m.carrier == CarrierAffinity::Footwork || m.carrier == CarrierAffinity::Shield {
                    0.65
                } else {
                    0.0
                };
                1.0 + rec_b + foot_bonus + m.w_3d[2] * 0.25
            }
        }
    };

    // Stage 0 初始化
    for (i, m) in candidates_by_stage[0].iter().enumerate() {
        let s0 = motion_smoothness(bone, m);
        let mult = slot_opt.map(|sl| slot_bonus(sl, m)).unwrap_or(1.0);
        dp[0][i] = (s0 * mult, 0);
    }

    // 階段 1..5 推進
    for s in 1..5 {
        for (cur_idx, m_cur) in candidates_by_stage[s].iter().enumerate() {
            let mult = slot_opt.map(|sl| slot_bonus(sl, m_cur)).unwrap_or(1.0);
            let mut best_val = -1.0f64;
            let mut best_prev = 0;
            for (prev_idx, m_prev) in candidates_by_stage[s - 1].iter().enumerate() {
                let prev_score = dp[s - 1][prev_idx].0;
                let c_val = transition_continuity(bone, m_prev, m_cur, learned_delta, familiarity_count);
                let score = prev_score * c_val * mult;
                if score > best_val {
                    best_val = score;
                    best_prev = prev_idx;
                }
            }
            dp[s][cur_idx] = (best_val, best_prev);
        }
    }

    // 回溯最優路徑
    let mut best_end_val = -1.0f64;
    let mut best_end_idx = 0;
    for (i, _) in candidates_by_stage[4].iter().enumerate() {
        if dp[4][i].0 > best_end_val {
            best_end_val = dp[4][i].0;
            best_end_idx = i;
        }
    }

    let mut path_indices = vec![0; 5];
    path_indices[4] = best_end_idx;
    for s in (1..5).rev() {
        path_indices[s - 1] = dp[s][path_indices[s]].1;
    }

    let mut result_motions = Vec::with_capacity(5);
    let mut compensated_stages = Vec::new();

    for s in 0..5 {
        let picked = candidates_by_stage[s][path_indices[s]].clone();
        let best_bone_axis = bone.stage_best_axis(s);
        let motion_axis = if picked.w_3d[0] >= picked.w_3d[1] && picked.w_3d[0] >= picked.w_3d[2] {
            0
        } else if picked.w_3d[1] >= picked.w_3d[2] {
            1
        } else {
            2
        };

        if motion_axis != 0 && bone.r_matrix[0][s] > 750.0 && motion_axis == best_bone_axis {
            compensated_stages.push(s);
        }

        result_motions.push(picked);
    }

    // 四段轉移幾何平均：沿回溯選出的 5 階段路徑重新累乘四步實際轉移分數（含跨軸代償 0.88）
    let mut geom_product = 1.0f64;
    for s in 0..4 {
        let m_prev = &result_motions[s];
        let m_next = &result_motions[s + 1];
        let mut step_c = transition_continuity(bone, m_prev, m_next, learned_delta, familiarity_count);
        if compensated_stages.contains(&(s + 1)) {
            step_c *= 0.88;
        }
        geom_product *= step_c.max(1e-6);
    }
    let continuity_c = geom_product.powf(0.25).max(0.1);

    (result_motions, compensated_stages, continuity_c)
}

fn find_viterbi_5stages(
    bone: &InnateBonePhase,
    candidates_by_stage: &[Vec<&ElementaryMotionDef>; 5],
    learned_delta: f64,
    familiarity_count: u32,
) -> (Vec<ElementaryMotionDef>, Vec<usize>, f64) {
    find_viterbi_5stages_for_slot(bone, None, candidates_by_stage, learned_delta, familiarity_count)
}

/// 輕量無字串配置的輔助函式：直接執行 5 階段 Viterbi 算出真實幾何平均連貫度與轉位變化性
pub fn compute_viterbi_continuity_and_variation(
    bone: &InnateBonePhase,
    slot: TacticalSlot,
    carrier: CarrierAffinity,
    learned_delta: f64,
    familiarity_count: u32,
) -> (f64, f64) {
    let catalog = &*GLOBAL_MOTION_CATALOG;
    let mut by_stage: [Vec<&ElementaryMotionDef>; 5] =
        [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];

    for m in catalog {
        if m.craft_domain.is_none() && m.carrier == carrier && m.stage < 5 {
            by_stage[m.stage].push(m);
        }
    }

    if slot == TacticalSlot::Defense && carrier != CarrierAffinity::Footwork {
        for m in catalog {
            if m.craft_domain.is_none() && m.carrier == CarrierAffinity::Footwork && m.stage < 5 {
                by_stage[m.stage].push(m);
            }
        }
    }

    for s in 0..5 {
        if by_stage[s].is_empty() {
            for m in catalog {
                if m.craft_domain.is_none() && m.carrier == CarrierAffinity::Unarmed && m.stage == s {
                    by_stage[s].push(m);
                }
            }
        }
    }

    let (motions, _compensated, continuity_c) = find_viterbi_5stages_for_slot(
        bone,
        Some(slot),
        &by_stage,
        learned_delta,
        familiarity_count,
    );

    let main_turn_name = &motions[2].name;
    let preferred_target_idx = match slot {
        TacticalSlot::Opening => 2,
        TacticalSlot::Midgame => 0,
        TacticalSlot::Finisher => 1,
        TacticalSlot::Defense => 0,
    };

    let mut best_branch_fit = -1.0f64;
    for m in &by_stage[2] {
        if m.name != *main_turn_name {
            let target_fit = m.counter_fit[preferred_target_idx];
            if target_fit > best_branch_fit {
                best_branch_fit = target_fit;
            }
        }
    }

    if best_branch_fit < 0.0 {
        for m in &by_stage[2] {
            if m.name != *main_turn_name {
                let fit_sum = m.counter_fit[0].max(m.counter_fit[1]).max(m.counter_fit[2]);
                if fit_sum > best_branch_fit {
                    best_branch_fit = fit_sum;
                }
            }
        }
    }

    let variation_v = (best_branch_fit.max(0.4) * (1.0 + 0.25 * (-bone.gamma).max(0.0))).clamp(0.40, 1.80);
    (continuity_c, variation_v)
}
