//! Copy-on-Write 全服人物表 `ActorMap`（獨立子模組，維持 `types.rs` 精簡清晰）。

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Index;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::types::UnifiedMindBody;

/// Copy-on-Write 全服人物表：工作副本 `clone()` 僅遞增 `Arc` 計數（萬人規模 < 0.2ms 零深拷貝），`get_mut` 自動執行 `Arc::make_mut` 並登記 `dirty_keys`！
#[derive(Debug, Clone, Default)]
pub struct ActorMap {
    inner: BTreeMap<String, Arc<UnifiedMindBody>>,
    dirty_keys: BTreeSet<String>,
}

impl Serialize for ActorMap {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.inner.iter().map(|(k, v)| (k, v.as_ref())))
    }
}

impl<'de> Deserialize<'de> for ActorMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = BTreeMap::<String, UnifiedMindBody>::deserialize(deserializer)?;
        let mut inner = BTreeMap::new();
        let mut dirty_keys = BTreeSet::new();
        for (k, v) in raw {
            if v.dirty {
                dirty_keys.insert(k.clone());
            }
            inner.insert(k, Arc::new(v));
        }
        Ok(Self { inner, dirty_keys })
    }
}

impl ActorMap {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn contains_key(&self, k: &str) -> bool {
        self.inner.contains_key(k)
    }

    pub fn get(&self, k: &str) -> Option<&UnifiedMindBody> {
        self.inner.get(k).map(Arc::as_ref)
    }

    pub fn get_mut(&mut self, k: &str) -> Option<&mut UnifiedMindBody> {
        if let Some(arc) = self.inner.get_mut(k) {
            self.dirty_keys.insert(k.to_string());
            Some(Arc::make_mut(arc))
        } else {
            None
        }
    }

    pub fn insert(&mut self, k: String, v: UnifiedMindBody) -> Option<UnifiedMindBody> {
        if v.dirty || v.rev != v.flushed_rev || v.copper != v.flushed_copper {
            self.dirty_keys.insert(k.clone());
        } else {
            self.dirty_keys.remove(&k);
        }
        self.inner.insert(k, Arc::new(v)).map(Arc::unwrap_or_clone)
    }

    pub fn remove(&mut self, k: &str) -> Option<UnifiedMindBody> {
        self.dirty_keys.remove(k);
        self.inner.remove(k).map(Arc::unwrap_or_clone)
    }

    pub fn keys(&self) -> std::collections::btree_map::Keys<'_, String, Arc<UnifiedMindBody>> {
        self.inner.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &UnifiedMindBody> {
        self.inner.values().map(Arc::as_ref)
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut UnifiedMindBody> {
        for k in self.inner.keys() {
            self.dirty_keys.insert(k.clone());
        }
        self.inner.values_mut().map(Arc::make_mut)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &UnifiedMindBody)> {
        self.inner.iter().map(|(k, v)| (k, v.as_ref()))
    }

    pub fn dirty_keys(&self) -> &BTreeSet<String> {
        &self.dirty_keys
    }

    pub fn unmark_dirty_key(&mut self, k: &str) {
        self.dirty_keys.remove(k);
    }
}

impl Index<&str> for ActorMap {
    type Output = UnifiedMindBody;
    fn index(&self, index: &str) -> &Self::Output {
        self.get(index).expect("actor key exists")
    }
}

impl Index<&String> for ActorMap {
    type Output = UnifiedMindBody;
    fn index(&self, index: &String) -> &Self::Output {
        self.get(index.as_str()).expect("actor key exists")
    }
}

impl<'a> IntoIterator for &'a ActorMap {
    type Item = (&'a String, &'a UnifiedMindBody);
    type IntoIter = std::iter::Map<
        std::collections::btree_map::Iter<'a, String, Arc<UnifiedMindBody>>,
        fn((&'a String, &'a Arc<UnifiedMindBody>)) -> (&'a String, &'a UnifiedMindBody),
    >;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter().map(|(k, v)| (k, v.as_ref()))
    }
}
