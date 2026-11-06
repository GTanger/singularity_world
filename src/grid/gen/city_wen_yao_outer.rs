fn build_wen_yao_outer_perimeter(
    map: &mut SettlementCellMap,
    cx: i32,
    cy: i32,
    city_name: &str,
    outer_slots: &mut Vec<(i32, i32, SquareCoord)>,
) {
    for dx in -17i32..=17i32 {
        for dy in -17i32..=17i32 {
            if dx.abs() < 10 && dy.abs() < 10 {
                continue;
            }
            let coord = SquareCoord::new(cx + dx, cy + dy);

                // 1. 最外圍城牆與四向城門（dx.abs() == 17 || dy.abs() == 17，共 136 格）
                if dx.abs() == 17 || dy.abs() == 17 {
                    if dx == 0 && dy == 17 {
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "北城門"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}厚重雄偉之北城門，守衛森嚴，商流行人穿梭不絕。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else if dx == 0 && dy == -17 {
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "南城門"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}厚重雄偉之南城門，守衛森嚴，商流行人穿梭不絕。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else if dx == 17 && dy == 0 {
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "東城門"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}厚重雄偉之東城門，守衛森嚴，商流行人穿梭不絕。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else if dx == -17 && dy == 0 {
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "西城門"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}厚重雄偉之西城門，守衛森嚴，商流行人穿梭不絕。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else {
                        let mut cell = GridCell::new(coord, Terrain::Wall, format_settlement_cell_name(city_name, "實心城牆"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}高大堅固之條石城牆，上覆防禦機關與哨堞，固若金湯。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    }
                }
                // 2. 外環坊區街道與格位（11 <= max(|dx|, |dy|) <= 16，共 648 格）
                else if dx.abs() >= 11 || dy.abs() >= 11 {
                    let is_cross_outer = dx == 0 || dy == 0;
                    let is_ring_outer = dx.abs() == 13 || dy.abs() == 13;
                    let is_district_outer = dx.abs() == 3 || dy.abs() == 3 || dx.abs() == 5 || dy.abs() == 5 || dx.abs() == 8 || dy.abs() == 8;

                    if is_cross_outer || is_ring_outer || is_district_outer {
                        if is_cross_outer {
                            let (terrain, street_name, desc) = if dx == 0 && dy > 0 {
                            match dy {
                                11 => (
                                    Terrain::Road,
                                    "北門外街南段",
                                    "文曜北門外街南段，南接北內門坊，兩旁青石鋪地，商旅車馬穿梭，北望津渡碼頭與商鋪。",
                                ),
                                12 => (
                                    Terrain::Road,
                                    "北門外街船埠口",
                                    "文曜北門外街船埠路口，西側緊鄰三水外坊貨運碼頭，江風送爽，常有水手拉縴運卸重物。",
                                ),
                                13 => (
                                    Terrain::Road,
                                    "北門外十字街口",
                                    "文曜北門外十字大街交叉口，東西橫貫外環道，北望北城門哨塔，四向人潮車馬絡繹不絕。",
                                ),
                                14 => (
                                    Terrain::Road,
                                    "北門外街學院口",
                                    "文曜北門外街學院分院路口，東臨格物輿圖外院，文士教席與行商在此交匯，竹蔭清幽。",
                                ),
                                15 => (
                                    Terrain::Road,
                                    "北門外街北段",
                                    "文曜北門外街北段，平整寬闊的條石驛道延伸向前，兩側修築石質排水溝與栓馬樁。",
                                ),
                                _ => (
                                    Terrain::Road,
                                    "北門外街關前",
                                    "文曜北門外街北端關前石坪，正對高大巍峨的北城門，守衛巡卒駐守檢驗，氣度森嚴。",
                                ),
                            }
                        } else if dx == 0 && dy < 0 {
                            match dy {
                                -11 => (
                                    Terrain::Road,
                                    "南門外街北段",
                                    "文曜南門外街北段，北鄰南內門坊，車馬載運工坊各色物資南來北往，地面車轍隱然。",
                                ),
                                -12 => (
                                    Terrain::Road,
                                    "南門外街鍛坊口",
                                    "文曜南門外街鍛坊路口，東側毗鄰重鐵水錘大鍛坊，陣陣洪亮錘擊聲與火星熱氣隱隱傳來。",
                                ),
                                -13 => (
                                    Terrain::Road,
                                    "南門外十字街口",
                                    "文曜南門外十字大街交匯處，東西橫穿外環大道，南通南哨與淺澤泥口，車水馬龍，商旅駐足歇腳。",
                                ),
                                -14 => (
                                    Terrain::Road,
                                    "南門外街車馬口",
                                    "文曜南門外街車馬行路口，西接駝馬車行棧房，長途大車常在此換韁修轅，人聲鼎沸。",
                                ),
                                -15 => (
                                    Terrain::Road,
                                    "南門外街南段",
                                    "文曜南門外街南段，寬闊條石大道筆直向南鋪展，遠山如黛，道旁列植迎客青松。",
                                ),
                                _ => (
                                    Terrain::Road,
                                    "南門外街關前",
                                    "文曜南門外街南端關前廣場，前臨高聳堅固的南城門，守兵巡查出入關牒，防守森嚴。",
                                ),
                            }
                        } else if dy == 0 && dx > 0 {
                            match dx {
                                11 => (
                                    Terrain::Road,
                                    "東門外街西段",
                                    "文曜東門外街西段，西依東內門坊，向東通往紙墨印書諸坊，街面潔淨，槐柳成蔭。",
                                ),
                                12 => (
                                    Terrain::Road,
                                    "東門外街紙坊口",
                                    "文曜東門外街紙坊路口，北側緊鄰青檀宣紙工坊，陣陣草木紙漿清香隨風飄拂。",
                                ),
                                13 => (
                                    Terrain::Road,
                                    "東門外十字街口",
                                    "文曜東門外十字街交會處，南北連接外環林蔭大道，向東直望巍峨東城門，行人如織。",
                                ),
                                14 => (
                                    Terrain::Road,
                                    "東門外街機樞口",
                                    "文曜東門外街機樞工坊路口，南側百工器械機坊轉軸聲隱然傳來，工匠運料絡繹不絕。",
                                ),
                                15 => (
                                    Terrain::Road,
                                    "東門外街東段",
                                    "文曜東門外街東段，平坦堅固的青石長街向東延展，臨近東關城門，街面寬廣平整。",
                                ),
                                _ => (
                                    Terrain::Road,
                                    "東門外街關前",
                                    "文曜東門外街東端關前石坪，扼守東城門樞紐，東臨江港外陸通道，關防嚴整。",
                                ),
                            }
                        } else {
                            match dx {
                                -11 => (
                                    Terrain::Road,
                                    "西門外街東段",
                                    "文曜西門外街東段，東接西內門坊，往來商賈駝鈴悠揚，大車載著皮毛藥材出入城關。",
                                ),
                                -12 => (
                                    Terrain::Road,
                                    "西門外街貨棧口",
                                    "文曜西門外街貨棧路口，南北兩側商行貨棧林立，堆積著來自西原大地的厚重貨包。",
                                ),
                                -13 => (
                                    Terrain::Road,
                                    "西門外十字街口",
                                    "文曜西門外十字大街交叉點，南北橫貫駝馬外環道，四方商隊匯聚，叫賣與卸貨聲不絕。",
                                ),
                                -14 => (
                                    Terrain::Road,
                                    "西門外街商埠口",
                                    "文曜西門外街商埠路口，臨近駝馬大旅社與皮毛拍賣行，西行商旅在此打點行裝鞍具。",
                                ),
                                -15 => (
                                    Terrain::Road,
                                    "西門外街西段",
                                    "文曜西門外街西段，開闊堅實的石板長街直貫西方，大道兩側建有防風土壘與栓馬欄木。",
                                ),
                                _ => (
                                    Terrain::Road,
                                    "西門外街關前",
                                    "文曜西門外街西端關前石坪，西對高大厚重的西城門，長途商旅於此驗引換符，守備謹嚴。",
                                ),
                            }
                        };
                            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, street_name));
                            cell.zone = city_name.to_string();
                            cell.description = desc.to_string();
                            cell.explored = true;
                            map.insert(coord, cell);
                        } else if is_ring_outer {
                            let (terrain, street_name, desc): (Terrain, String, String) = if dx < 0 && dy > 0 {
                                let t = if (dx.abs() + dy.abs()) % 4 == 0 { Terrain::Bridge } else { Terrain::Road };
                                if dx == -13 && dy == 13 {
                                    (
                                        t,
                                        "三水外環西北隅".to_string(),
                                        "三水外坊外環西北隅轉角，三水河口水網匯入，波濤粼粼，設有防汛石堤與航標燈樁。".to_string(),
                                    )
                                } else if dy == 13 {
                                    if dx == -8 {
                                        (
                                            t,
                                            "三水外環北道".to_string(),
                                            "文曜三水外坊外環北道，沿著北灣水岸修築，臨水一側泊滿各色漁舟與小火輪艟。".to_string(),
                                        )
                                    } else {
                                        let suf = match dx {
                                            -1 => "東口", -2 => "大堤段", -3 => "支巷口", -4 => "堤中段",
                                            -5 => "津渡口", -6 => "堤東段", -7 => "水岸段", -9 => "渡口段",
                                            -10 => "內口", -11 => "外段", -12 => "防汛段", -14 => "北西段",
                                            -15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            t,
                                            format!("三水外環北{suf}"),
                                            format!("文曜三水外坊外環北道{suf}，沿著水岸修築，波濤粼粼，設有防汛石堤與停泊木樁。"),
                                        )
                                    }
                                } else {
                                    if dy == 8 {
                                        (
                                            t,
                                            "三水外環西道".to_string(),
                                            "文曜三水外坊外環西道，西臨水路河堤，石路平坦寬闊，漁具修補棚架連綿。".to_string(),
                                        )
                                    } else {
                                        let suf = match dy {
                                            1 => "南口", 2 => "泊船段", 3 => "支巷口", 4 => "水網段",
                                            5 => "修船口", 6 => "河堤段", 7 => "中段", 9 => "水柵段",
                                            10 => "內口", 11 => "外段", 12 => "燈樁段", 14 => "西北段",
                                            15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            t,
                                            format!("三水外環西{suf}"),
                                            format!("文曜三水外坊外環西道{suf}，西臨水路河堤，石路平坦寬闊，漁具修補棚架連綿。"),
                                        )
                                    }
                                }
                            } else if dx > 0 && dy > 0 {
                                if dx == 13 && dy == 13 {
                                    (
                                        Terrain::Road,
                                        "講堂外環東北隅".to_string(),
                                        "講堂外坊外環東北隅轉角，修竹萬竿，古松環抱，幽靜林道交匯於此，清風習習。".to_string(),
                                    )
                                } else if dy == 13 {
                                    if dx == 8 {
                                        (
                                            Terrain::Road,
                                            "講堂外環北道".to_string(),
                                            "文曜講堂外坊外環北道，修竹夾道，林蔭遮蔽，學堂研討之所星羅棋布，書卷氣濃。".to_string(),
                                        )
                                    } else {
                                        let suf = match dx {
                                            1 => "西口", 2 => "林蔭段", 3 => "支巷口", 4 => "修竹段",
                                            5 => "校舍口", 6 => "竹林段", 7 => "清幽段", 9 => "書香段",
                                            10 => "內口", 11 => "外段", 12 => "研討段", 14 => "東北段",
                                            15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("講堂外環北{suf}"),
                                            format!("文曜講堂外坊外環北道{suf}，修竹夾道，林蔭遮蔽，校舍研討之所星羅棋布。"),
                                        )
                                    }
                                } else {
                                    if dy == 8 {
                                        (
                                            Terrain::Road,
                                            "講堂外環東道".to_string(),
                                            "文曜講堂外坊外環東道，青石大道平闊幽靜，兩旁多見紙坊、墨莊與著述譯館。".to_string(),
                                        )
                                    } else {
                                        let suf = match dy {
                                            1 => "南口", 2 => "松柏段", 3 => "支巷口", 4 => "譯館段",
                                            5 => "紙坊口", 6 => "碑帖段", 7 => "中段", 9 => "石道段",
                                            10 => "內口", 11 => "外段", 12 => "著述段", 14 => "東南段",
                                            15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("講堂外環東{suf}"),
                                            format!("文曜講堂外坊外環東道{suf}，青石大道平闊幽靜，兩旁多見紙坊、墨莊與著述譯館。"),
                                        )
                                    }
                                }
                            } else if dx > 0 && dy < 0 {
                                if dx == 13 && dy == -13 {
                                    (
                                        Terrain::Road,
                                        "百工外環東南隅".to_string(),
                                        "百工外坊外環東南隅轉角，臨近重工業區，石料砧台與木料排垛堆疊有序。".to_string(),
                                    )
                                } else if dy == -13 {
                                    if dx == 8 {
                                        (
                                            Terrain::Road,
                                            "百工外環南道".to_string(),
                                            "文曜百工外坊外環南道，工坊密集，大水車與鍛錘運轉聲響徹長空，熱力逼人。".to_string(),
                                        )
                                    } else {
                                        let suf = match dx {
                                            1 => "西口", 2 => "砧台段", 3 => "支巷口", 4 => "大車段",
                                            5 => "水車口", 6 => "爐火段", 7 => "鍛打段", 9 => "重工段",
                                            10 => "內口", 11 => "外段", 12 => "火磚段", 14 => "東南段",
                                            15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("百工外環南{suf}"),
                                            format!("文曜百工外坊外環南道{suf}，工坊密集，石料砧台與木料排垛堆疊有序，熱力逼人。"),
                                        )
                                    }
                                } else {
                                    if dy == -8 {
                                        (
                                            Terrain::Road,
                                            "百工外環東道".to_string(),
                                            "文曜百工外坊外環東道，條石堅厚，運送礦料石胚與巨木的大車絡繹不絕。".to_string(),
                                        )
                                    } else {
                                        let suf = match dy {
                                            -1 => "北口", -2 => "礦料段", -3 => "支巷口", -4 => "木料段",
                                            -5 => "巨木口", -6 => "石胚段", -7 => "中段", -9 => "重料段",
                                            -10 => "內口", -11 => "外段", -12 => "冶鑄段", -14 => "東北段",
                                            -15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("百工外環東{suf}"),
                                            format!("文曜百工外坊外環東道{suf}，條石堅厚，運送礦料石胚與巨木的大車絡繹不絕。"),
                                        )
                                    }
                                }
                            } else {
                                if dx == -13 && dy == -13 {
                                    (
                                        Terrain::Road,
                                        "西門外環西南隅".to_string(),
                                        "西門外坊外環西南隅轉角，視野開闊，設有大隊馱馬調頭石坪與避風擋牆。".to_string(),
                                    )
                                } else if dy == -13 {
                                    if dx == -8 {
                                        (
                                            Terrain::Road,
                                            "西門外環南道".to_string(),
                                            "文曜西門外坊外環南道，商道通暢，西域香料、山珍藥材與駝隊輜重雲集於此。".to_string(),
                                        )
                                    } else {
                                        let suf = match dx {
                                            -1 => "東口", -2 => "馱馬段", -3 => "支巷口", -4 => "商道段",
                                            -5 => "草料口", -6 => "輜重段", -7 => "避風段", -9 => "歇馬段",
                                            -10 => "內口", -11 => "外段", -12 => "香料段", -14 => "西南段",
                                            -15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("西門外環南{suf}"),
                                            format!("文曜西門外坊外環南道{suf}，商道通暢，西域香料、山珍藥材與駝隊輜重雲集於此。"),
                                        )
                                    }
                                } else {
                                    if dy == -8 {
                                        (
                                            Terrain::Road,
                                            "西門外環西道".to_string(),
                                            "文曜西門外坊外環西道，大道開闊宏敞，旅社歇所與飲馬水井整齊分佈。".to_string(),
                                        )
                                    } else {
                                        let suf = match dy {
                                            -1 => "北口", -2 => "大路段", -3 => "支巷口", -4 => "棧房段",
                                            -5 => "水井口", -6 => "旅社段", -7 => "中段", -9 => "行道段",
                                            -10 => "內口", -11 => "外段", -12 => "馬隊段", -14 => "西北段",
                                            -15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("西門外環西{suf}"),
                                            format!("文曜西門外坊外環西道{suf}，大道開闊宏敞，旅社歇所與飲馬水井整齊分佈。"),
                                        )
                                    }
                                }
                            };
                            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, &street_name));
                            cell.zone = city_name.to_string();
                            cell.description = desc;
                            cell.explored = true;
                            map.insert(coord, cell);
                        } else {
                            let pos_seg = |v: i32| match v {
                                11 => "內口", 12 => "二段", 13 => "中段", 14 => "四段", 15 => "外段", _ => "關前",
                            };
                            let neg_seg = |v: i32| match v {
                                -11 => "內口", -12 => "二段", -13 => "中段", -14 => "四段", -15 => "外段", _ => "關前",
                            };
                            let (terrain, street_name, desc): (Terrain, String, String) = if dx < 0 && dy > 0 {
                                let t = if (dx.abs() + dy.abs()) % 3 == 0 { Terrain::Bridge } else { Terrain::Road };
                                let (sub, seg) = if (dx == -3 || dx == -5 || dx == -8) && dy >= 11 {
                                    let sub = match dx { -3 => "河市街", -5 => "漁舟巷", _ => "通濟道" };
                                    (sub, pos_seg(dy))
                                } else {
                                    let sub = match dy { 3 => "船塢街", 5 => "早市街", _ => "水柵巷" };
                                    (sub, neg_seg(dx))
                                };
                                (
                                    t,
                                    format!("三水外坊・{sub}{seg}"),
                                    format!("三水外坊臨水石街{sub}{seg}，魚市船塢夾道，水波蕩漾。"),
                                )
                            } else if dx > 0 && dy > 0 {
                                let (sub, seg) = if (dx == 3 || dx == 5 || dx == 8) && dy >= 11 {
                                    let sub = match dx { 3 => "校舍街", 5 => "格物巷", _ => "文風道" };
                                    (sub, pos_seg(dy))
                                } else {
                                    let sub = match dy { 3 => "墨莊街", 5 => "印書街", _ => "譯館巷" };
                                    (sub, pos_seg(dx))
                                };
                                (
                                    Terrain::Road,
                                    format!("講堂外坊・{sub}{seg}"),
                                    format!("講堂外坊幽雅的校舍石街{sub}{seg}，文人墨客執卷漫步，墨香盈袖。"),
                                )
                            } else if dx > 0 && dy < 0 {
                                let (sub, seg) = if (dx == 3 || dx == 5 || dx == 8) && dy <= -11 {
                                    let sub = match dx { 3 => "水錘街", 5 => "大木巷", _ => "五金道" };
                                    (sub, neg_seg(dy))
                                } else {
                                    let sub = match dy { -3 => "鐵匠街", -5 => "車船街", _ => "鍛打巷" };
                                    (sub, pos_seg(dx))
                                };
                                (
                                    Terrain::Road,
                                    format!("百工外坊・{sub}{seg}"),
                                    format!("百工外坊熱絡的工匠街巷{sub}{seg}，叮噹鐵聲與刨木芳香洋溢其間。"),
                                )
                            } else {
                                let (sub, seg) = if (dx == -3 || dx == -5 || dx == -8) && dy <= -11 {
                                    let sub = match dx { -3 => "駝隊街", -5 => "皮毛巷", _ => "糧草道" };
                                    (sub, neg_seg(dy))
                                } else {
                                    let sub = match dy { -3 => "驛站街", -5 => "客商街", _ => "護衛巷" };
                                    (sub, neg_seg(dx))
                                };
                                (
                                    Terrain::Road,
                                    format!("西門外坊・{sub}{seg}"),
                                    format!("西門外坊平實堅固的車馬街市{sub}{seg}，堆棧貨棚連綿，行商熙攘。"),
                                )
                            };
                            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, &street_name));
                            cell.zone = city_name.to_string();
                            cell.description = desc;
                            cell.explored = true;
                            map.insert(coord, cell);
                        }
                    } else {
                        outer_slots.push((dx, dy, coord));
                    }
                }
                // 3. 內城環道與四向內門坊（dx.abs() == 10 || dy.abs() == 10，共 80 格）
                else if dx.abs() == 10 || dy.abs() == 10 {
                    let (street_name, desc): (String, String) = if dx == 0 && dy == 10 {
                        (
                            "北內門坊".to_string(),
                            "文曜內外城交界的青石牌坊，北接北門外街，南連講堂街北段，車馬行人絡繹不絕。".to_string(),
                        )
                    } else if dx == 0 && dy == -10 {
                        (
                            "南內門坊".to_string(),
                            "文曜內外城交界的雄渾石坊，南通南門外街，北接南門石板大街。".to_string(),
                        )
                    } else if dx == 10 && dy == 0 {
                        (
                            "東內門坊".to_string(),
                            "文曜內外城交界的飛簷石坊，東連東門外街，西接東門石板大道。".to_string(),
                        )
                    } else if dx == -10 && dy == 0 {
                        (
                            "西內門坊".to_string(),
                            "文曜內外城交界的寬闊石坊，西接西門外街，東連西門聯運大道。".to_string(),
                        )
                    } else if dx < 0 && dy > 0 {
                        if dx == -10 && dy == 10 {
                            (
                                "三水內環西北隅".to_string(),
                                "文曜三水坊內環西北轉角石坪，水波微漾，石階與柳蔭環護，北通三水外坊，東連水坊北街。".to_string(),
                            )
                        } else if dy == 10 {
                            let suf = match dx {
                                -1 => "北門西口", -2 => "北一段", -3 => "北水驛段", -4 => "北二段",
                                -5 => "北中段", -6 => "北三段", -7 => "北泊船段", -8 => "北四段", _ => "北西口",
                            };
                            (
                                format!("三水內環{suf}"),
                                format!("文曜三水坊內環北側{suf}，傍水依岸而建，石路平坦寬闊，茶鋪與水上貨站林立。"),
                            )
                        } else {
                            let suf = match dy {
                                1 => "西門北口", 2 => "西一段", 3 => "西商倉段", 4 => "西二段",
                                5 => "西中段", 6 => "西三段", 7 => "西河堤段", 8 => "西四段", _ => "西北口",
                            };
                            (
                                format!("三水內環{suf}"),
                                format!("文曜三水坊內環西側{suf}，沿護城水渠鋪就，微風拂面，水陸換運車馬絡繹不絕。"),
                            )
                        }
                    } else if dx > 0 && dy > 0 {
                        if dx == 10 && dy == 10 {
                            (
                                "講堂內環東北隅".to_string(),
                                "文曜講堂坊內環東北轉角，古槐參天，竹林掩映，青石坪上偶有求學士子駐足默誦。".to_string(),
                            )
                        } else if dy == 10 {
                            let suf = match dx {
                                1 => "北門東口", 2 => "北一段", 3 => "北學寮段", 4 => "北二段",
                                5 => "北中段", 6 => "北三段", 7 => "北林苑段", 8 => "北四段", _ => "北東口",
                            };
                            (
                                format!("講堂內環{suf}"),
                                format!("文曜講堂坊內環北側{suf}，修竹夾道，林蔭遮蔽，校舍研討之所星羅棋布。"),
                            )
                        } else {
                            let suf = match dy {
                                1 => "東門北口", 2 => "東一段", 3 => "東著述段", 4 => "東二段",
                                5 => "東中段", 6 => "東三段", 7 => "東幽徑段", 8 => "東四段", _ => "東北口",
                            };
                            (
                                format!("講堂內環{suf}"),
                                format!("文曜講堂坊內環東側{suf}，石板路面平整，文士墨客攜卷緩行，字畫碑帖鋪面錯落。"),
                            )
                        }
                    } else if dx > 0 && dy < 0 {
                        if dx == 10 && dy == -10 {
                            (
                                "分家內環東南隅".to_string(),
                                "文曜百工坊內環東南轉角，條石厚重平整，角落設有消防引水石槽與工匠歇息木亭。".to_string(),
                            )
                        } else if dy == -10 {
                            let suf = match dx {
                                1 => "南門東口", 2 => "南一段", 3 => "南工坊段", 4 => "南二段",
                                5 => "南中段", 6 => "南三段", 7 => "南炭場段", 8 => "南四段", _ => "南東口",
                            };
                            (
                                format!("分家內環{suf}"),
                                format!("文曜百工坊內環南側{suf}，車輛運載生鐵木料絡繹不絕，空氣中瀰漫著木炭與熱金氣息。"),
                            )
                        } else {
                            let suf = match dy {
                                -1 => "東門南口", -2 => "東一段", -3 => "東鐵料段", -4 => "東二段",
                                -5 => "東中段", -6 => "東三段", -7 => "東木器段", -8 => "東四段", _ => "東南口",
                            };
                            (
                                format!("分家內環{suf}"),
                                format!("文曜百工坊內環東側{suf}，石板路面經重車碾壓顯得油亮堅實，爐火工聲晝夜不息。"),
                            )
                        }
                    } else {
                        if dx == -10 && dy == -10 {
                            (
                                "西門內環西南隅".to_string(),
                                "文曜西門坊內環西南轉角，設有大車轉向與維修開闊石坪，粗大木樁整齊排列。".to_string(),
                            )
                        } else if dy == -10 {
                            let suf = match dx {
                                -1 => "南門西口", -2 => "南一段", -3 => "南車行段", -4 => "南二段",
                                -5 => "南中段", -6 => "南三段", -7 => "南牧馬段", -8 => "南四段", _ => "南西口",
                            };
                            (
                                format!("西門內環{suf}"),
                                format!("文曜西門坊內環南側{suf}，地勢平曠，沿道設有飲馬水槽與馱貨棧寮，蹄聲嘚嘚。"),
                            )
                        } else {
                            let suf = match dy {
                                -1 => "西門南口", -2 => "西一段", -3 => "西棧道段", -4 => "西二段",
                                -5 => "西中段", -6 => "西三段", -7 => "西馱場段", -8 => "西四段", _ => "西南口",
                            };
                            (
                                format!("西門內環{suf}"),
                                format!("文曜西門坊內環西側{suf}，商會旗幡迎風招展，承載各路客商厚重輜重。"),
                            )
                        }
                    };

                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, &street_name));
                    cell.zone = city_name.to_string();
                    cell.description = desc;
                    cell.explored = true;
                    map.insert(coord, cell);
                }
        }
    }
}
