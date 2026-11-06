/// 全服 500 詞元快取註冊表
pub struct WordRegistry {
    by_id: HashMap<String, WordElement>,
    by_char: HashMap<String, Vec<String>>,
    ordered_ids: Vec<String>,
}


static GLOBAL_REGISTRY: LazyLock<WordRegistry> = LazyLock::new(|| {
    WordRegistry::from_json_str(RAW_WORDS_JSON, RAW_GAMEDIMS_JSON)
        .expect("內嵌 500 詞元設定檔解析不可失敗")
});

impl WordRegistry {
    /// 獲取全局靜態快取的詞元庫
    pub fn global() -> &'static WordRegistry {
        &GLOBAL_REGISTRY
    }

    /// 從 JSON 字串解析並建立註冊表
    pub fn from_json_str(words_json: &str, gamedims_json: &str) -> Result<Self, WordPlateError> {
        #[derive(Deserialize)]
        struct RawWordsDoc {
            elements: Vec<RawWordEntry>,
        }
        #[derive(Deserialize)]
        struct RawWordEntry {
            id: String,
            #[serde(rename = "char")]
            ch: String,
            semantic: String,
            sources: Vec<String>,
            desc: String,
            #[serde(default)]
            embedding_desc: Option<String>,
        }

        #[derive(Deserialize)]
        struct RawGamedimsDoc {
            elements: Vec<RawGamedimsEntry>,
        }
        #[derive(Deserialize)]
        struct RawGamedimsEntry {
            id: String,
            game_dims: GameDimsCompat6D,
            vector_12d: Vector12D,
            #[serde(default)]
            craft_dims: CraftDims4D,
            carrier_affinity: CarrierAffinity,
            target_part: TargetPart,
        }
        let words_doc: RawWordsDoc = serde_json::from_str(words_json)
            .map_err(|e| WordPlateError::ConfigParseError(format!("word_elements: {e}")))?;
        let gamedims_doc: RawGamedimsDoc = serde_json::from_str(gamedims_json)
            .map_err(|e| WordPlateError::ConfigParseError(format!("word_elements_gamedims: {e}")))?;

        let mut gamedims_map: HashMap<String, RawGamedimsEntry> =
            HashMap::with_capacity(gamedims_doc.elements.len());
        for entry in gamedims_doc.elements {
            gamedims_map.insert(entry.id.clone(), entry);
        }

        let mut by_id = HashMap::with_capacity(words_doc.elements.len());
        let mut by_char: HashMap<String, Vec<String>> = HashMap::new();
        let mut ordered_ids = Vec::with_capacity(words_doc.elements.len());

        for w in words_doc.elements {
            let gd = gamedims_map
                .remove(&w.id)
                .ok_or_else(|| WordPlateError::ConfigParseError(format!("字訣 ID 於 gamedims 中遺失：{}", w.id)))?;

            let elem = WordElement {
                id: w.id.clone(),
                ch: w.ch.clone(),
                semantic: w.semantic,
                sources: w.sources,
                desc: w.desc,
                embedding_desc: w.embedding_desc,
                game_dims: gd.game_dims,
                vector_12d: gd.vector_12d,
                craft_dims: gd.craft_dims,
                carrier_affinity: gd.carrier_affinity,
                target_part: gd.target_part,
            };
            by_char.entry(w.ch).or_default().push(w.id.clone());
            ordered_ids.push(w.id.clone());
            by_id.insert(w.id, elem);
        }

        Ok(Self {
            by_id,
            by_char,
            ordered_ids,
        })
    }

    pub fn get_by_id(&self, id: &str) -> Option<&WordElement> {
        self.by_id.get(id)
    }

    pub fn get_by_char(&self, ch: &str) -> Option<&[String]> {
        self.by_char.get(ch).map(|v| v.as_slice())
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &WordElement> {
        self.by_id.values()
    }

    pub fn ordered_elements(&self) -> impl Iterator<Item = &WordElement> {
        self.ordered_ids.iter().filter_map(|id| self.by_id.get(id))
    }

    /// 依五源分類獲取所屬詞元清單
    pub fn words_by_source(&self, source: WordSource) -> Vec<&WordElement> {
        let tag = source.tag_zh();
        self.ordered_elements()
            .filter(|w| w.sources.iter().any(|s| s == tag))
            .collect()
    }

    /// 從指定源質庫中依種子與品質確定性隨機抽取一個詞元
    pub fn roll_word_from_source(&self, source: WordSource, quality: WordQuality, seed: u64) -> &WordElement {
        let candidates = self.words_by_source(source);
        if candidates.is_empty() {
            panic!("源質庫中無任何詞元：{:?}", source);
        }
        let mixed_seed = seed ^ ((quality as u64).wrapping_mul(0x9E3779B97F4A7C15));
        let rand_val = splitmix64(mixed_seed);
        let idx = (rand_val as usize) % candidates.len();
        candidates[idx]
    }
}

/// 隨身【詞匣】：自由收藏、增減、查詢玩家已擁有的單字詞元
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WordCasket {
    inventory: BTreeMap<String, u32>,
    #[serde(default)]
    pub qualities: BTreeMap<String, WordQuality>,
    #[serde(default)]
    pub charges: BTreeMap<String, u8>,
    #[serde(default)]
    pub extraction_ratios: BTreeMap<String, u8>,
    #[serde(default)]
    pub item_inscriptions: BTreeMap<String, Vec<String>>,
}

impl WordCasket {
    pub fn new() -> Self {
        Self {
            inventory: BTreeMap::new(),
            qualities: BTreeMap::new(),
            charges: BTreeMap::new(),
            extraction_ratios: BTreeMap::new(),
            item_inscriptions: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, word_id: impl Into<String>, count: u32) {
        if count == 0 {
            return;
        }
        *self.inventory.entry(word_id.into()).or_insert(0) += count;
    }

    pub fn add_extracted(&mut self, word_id: impl Into<String>, count: u32, quality: WordQuality, ratio: u8) {
        if count == 0 {
            return;
        }
        let wid = word_id.into();
        let current_quality = self.qualities.get(&wid).copied().unwrap_or(WordQuality::Bing);
        if quality > current_quality {
            self.qualities.insert(wid.clone(), quality);
        } else {
            self.qualities.entry(wid.clone()).or_insert(quality);
        }
        let current_ratio = self.extraction_ratios.get(&wid).copied().unwrap_or(0);
        if ratio > current_ratio {
            self.extraction_ratios.insert(wid.clone(), ratio);
        } else {
            self.extraction_ratios.entry(wid.clone()).or_insert(ratio);
        }
        self.charges.entry(wid.clone()).or_insert(0);
        *self.inventory.entry(wid).or_insert(0) += count;
    }

    pub fn quality_of(&self, word_id: &str) -> WordQuality {
        self.qualities.get(word_id).copied().unwrap_or_default()
    }

    pub fn charge_of(&self, word_id: &str) -> u8 {
        self.charges.get(word_id).copied().unwrap_or(0)
    }

    pub fn ratio_of(&self, word_id: &str) -> u8 {
        self.extraction_ratios.get(word_id).copied().unwrap_or(0)
    }

    pub fn remove(&mut self, word_id: &str, count: u32) -> Result<(), WordPlateError> {
        if count == 0 {
            return Ok(());
        }
        let current = self.inventory.get(word_id).copied().unwrap_or(0);
        if current < count {
            return Err(WordPlateError::InsufficientWordCount(
                word_id.to_string(),
                current,
                count,
            ));
        }
        if current == count {
            self.inventory.remove(word_id);
            self.qualities.remove(word_id);
            self.charges.remove(word_id);
            self.extraction_ratios.remove(word_id);
        } else {
            self.inventory.insert(word_id.to_string(), current - count);
        }
        Ok(())
    }

    pub fn count(&self, word_id: &str) -> u32 {
        self.inventory.get(word_id).copied().unwrap_or(0)
    }

    pub fn has(&self, word_id: &str) -> bool {
        self.count(word_id) > 0
    }

    pub fn total_distinct(&self) -> usize {
        self.inventory.len()
    }

    pub fn total_count(&self) -> u32 {
        self.inventory.values().sum()
    }

    pub fn list(&self) -> Vec<(&str, u32)> {
        self.inventory.iter().map(|(k, v)| (k.as_str(), *v)).collect()
    }

    /// 同名疊煉：消耗同名詞元 1 枚，積蓄靈韻；靈韻充盈圓滿時觸發進階判定
    pub fn stack_refine(
        &mut self,
        word_id: &str,
        xuan_luck_bonus: f32,
        seed: u64,
    ) -> Result<(WordQuality, u8, bool, String), WordPlateError> {
        let current_count = self.count(word_id);
        if current_count < 2 {
            return Err(WordPlateError::InsufficientWordCount(
                word_id.to_string(),
                current_count,
                2,
            ));
        }

        // 消耗同名詞元 1 枚
        self.remove(word_id, 1)?;

        let current_quality = self.quality_of(word_id);
        let current_charge = self.charge_of(word_id);
        let clamped_xuan = xuan_luck_bonus.clamp(0.0, 0.35);

        let rand_val = splitmix64(seed);
        let charge_gain = (20.0 + (rand_val % 21) as f32 + clamped_xuan * 40.0).round() as u8;
        let total_charge = (current_charge as u32) + (charge_gain as u32);

        if total_charge >= 100 {
            let ascend_rand = splitmix64(seed ^ 0xA5A5A5A55A5A5A5A);
            let roll = (ascend_rand % 100) as f32 / 100.0;
            let success_threshold = match current_quality {
                WordQuality::Bing => 0.60 + (clamped_xuan as f64 * 0.5) as f32, // 60%~77.5%
                WordQuality::Yi => 0.30 + (clamped_xuan as f64 * 0.4) as f32,   // 30%~44%
                WordQuality::Jia => 0.0, // 甲品已登極境，不再升階
            };

            if current_quality != WordQuality::Jia && roll < success_threshold {
                let next_quality = match current_quality {
                    WordQuality::Bing => WordQuality::Yi,
                    WordQuality::Yi => WordQuality::Jia,
                    WordQuality::Jia => WordQuality::Jia,
                };
                let remaining_charge = ((total_charge - 100) as u8).min(100);
                self.qualities.insert(word_id.to_string(), next_quality);
                self.charges.insert(word_id.to_string(), remaining_charge);
                let narrative = format!(
                    "同名真意交融共鳴，字訣【{}】由{}躍升為{}，靈韻相融，筆意愈發凝練。",
                    word_id,
                    current_quality.label_zh(),
                    next_quality.label_zh()
                );
                Ok((next_quality, remaining_charge, true, narrative))
            } else {
                // 未能升階或已至甲品極境
                let capped_charge = if current_quality == WordQuality::Jia {
                    100
                } else {
                    80.min(total_charge as u8)
                };
                self.charges.insert(word_id.to_string(), capped_charge);
                let narrative = if current_quality == WordQuality::Jia {
                    format!(
                        "字訣【{}】已臻甲品極境，同名相疊淬煉精華，靈韻充盈圓滿，一舉破繭昇華。",
                        word_id
                    )
                } else {
                    format!(
                        "同名真意衝擊極限未果，字訣【{}】維持{}，氣機微盪，靈韻略微回落。",
                        word_id,
                        current_quality.label_zh()
                    )
                };
                Ok((current_quality, capped_charge, false, narrative))
            }
        } else {
            let new_charge = total_charge as u8;
            self.charges.insert(word_id.to_string(), new_charge);
            let narrative = format!(
                "同名疊煉相融，字訣【{}】（{}）靈韻浸潤，筆意神彩更添一重。",
                word_id,
                current_quality.label_zh()
            );
            Ok((current_quality, new_charge, false, narrative))
        }
    }

    /// 鎔鑄化鎂：將 1 枚詞元鎔化為通用經濟資產「鎂」
    pub fn mint_to_magnesium(&mut self, word_id: &str) -> Result<(i32, WordQuality, String), WordPlateError> {
        let current_count = self.count(word_id);
        if current_count < 1 {
            return Err(WordPlateError::InsufficientWordCount(
                word_id.to_string(),
                0,
                1,
            ));
        }
        let q = self.quality_of(word_id);
        let val = q.magnesium_value();
        self.remove(word_id, 1)?;
        let narrative = format!(
            "鎔鑄{}字訣【{}】，化歸本源，化為 {} 鎂。",
            q.label_zh(),
            word_id,
            val
        );
        Ok((val, q, narrative))
    }

    /// 外盤器物銘文：將詞匣中的詞元烙印至持物或防具上（最多 3 字，超額擠出最舊詞元歸還詞匣）
    pub fn inscribe_item(
        &mut self,
        item_id: impl Into<String>,
        word_id: &str,
    ) -> Result<Vec<String>, WordPlateError> {
        let current_count = self.count(word_id);
        if current_count < 1 {
            return Err(WordPlateError::InsufficientWordCount(
                word_id.to_string(),
                0,
                1,
            ));
        }
        self.remove(word_id, 1)?;
        let key = item_id.into();
        let mut popped_oldest = None;
        let result = {
            let list = self.item_inscriptions.entry(key).or_default();
            if list.len() >= 3 {
                popped_oldest = Some(list.remove(0));
            }
            list.push(word_id.to_string());
            list.clone()
        };
        if let Some(oldest) = popped_oldest {
            self.add(oldest, 1);
        }
        Ok(result)
    }

    /// 查詢特定器物之銘文字譜
    pub fn inscribed_words_of(&self, item_id: &str) -> &[String] {
        self.item_inscriptions
            .get(item_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 格式化器物銘文後綴（例如「 [薄][巧][解]」；無則回傳空字串）
    pub fn format_inscribed_suffix(&self, item_id: &str, registry: &WordRegistry) -> String {
        let words = self.inscribed_words_of(item_id);
        if words.is_empty() {
            return String::new();
        }
        let mut out = String::from(" ");
        for wid in words {
            let ch = registry
                .get_by_id(wid)
                .map(|e| e.ch.as_str())
                .unwrap_or_else(|| {
                    if wid.is_ascii() {
                        "銘"
                    } else {
                        wid.as_str()
                    }
                });
            out.push('[');
            out.push_str(ch);
            out.push(']');
        }
        out
    }

    /// 計算器物銘文在百工日常（打鐵、山海合味烹飪、採獵剝名、行旅）的四維增益
    pub fn inscribed_craft_bonus(&self, item_id: &str, registry: &WordRegistry) -> CraftDims4D {
        let mut total = CraftDims4D::default();
        for wid in self.inscribed_words_of(item_id) {
            if let Some(elem) = registry.get_by_id(wid) {
                let mult = self.quality_of(wid).power_multiplier() as f64 * 0.35;
                total.forge += elem.craft_dims.forge * mult;
                total.culinary += elem.craft_dims.culinary * mult;
                total.harvest += elem.craft_dims.harvest * mult;
                total.wayfare += elem.craft_dims.wayfare * mult;
            }
        }
        total
    }

    /// 計算器物銘文在戰陣交鋒時的（攻擊、破甲、禦勁）加成
    pub fn inscribed_combat_bonus(&self, item_id: &str, registry: &WordRegistry) -> (f64, f64, f64) {
        let mut atk = 0.0;
        let mut pen = 0.0;
        let mut def = 0.0;
        for wid in self.inscribed_words_of(item_id) {
            if let Some(elem) = registry.get_by_id(wid) {
                let mult = self.quality_of(wid).power_multiplier() as f64;
                let v = elem.vector_12d.to_array();
                atk += (v[0] + v[1] + v[2]) * mult;
                pen += (v[2] + v[3] + v[9]) * mult;
                def += (v[4] + v[5] + v[8]) * mult;
            }
        }
        (atk, pen, def)
    }
}

/// 解析並驗證星盤節點（N000 不可鑲字，N001..N360 全可鑲字）
pub fn parse_star_node(node_id: &str) -> Result<usize, WordPlateError> {
    if node_id == "N000" {
        return Err(WordPlateError::NodeNotInlayable(
            "N000（生之樞紐為本源核心，不可鑲嵌字訣）".to_string(),
        ));
    }
    if !node_id.starts_with('N') {
        return Err(WordPlateError::InvalidNodeId(node_id.to_string()));
    }
    let num_str = &node_id[1..];
    let num: usize = num_str
        .parse()
        .map_err(|_| WordPlateError::InvalidNodeId(node_id.to_string()))?;
    if !(1..=360).contains(&num) {
        return Err(WordPlateError::InvalidNodeId(node_id.to_string()));
    }
    Ok(num)
}

/// 單一星位鑲嵌資訊
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StarInlay {
    pub node_id: String,
    pub word_id: String,
    pub fluency: OrbitFluency,
}

impl StarInlay {
    pub fn new(node_id: impl Into<String>, word_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            word_id: word_id.into(),
            fluency: OrbitFluency::PingWen,
        }
    }

    pub fn with_fluency(
        node_id: impl Into<String>,
        word_id: impl Into<String>,
        fluency: OrbitFluency,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            word_id: word_id.into(),
            fluency,
        }
    }
}

