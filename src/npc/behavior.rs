//! NPC 行為文案快取（對齊 Go `npc/behavior.go` 之 `GetShiftFlavor` 等子集）。

use std::collections::HashMap;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone)]
struct RoleBehaviorJson {
    #[serde(default)]
    shift_arrive: String,
    #[serde(default)]
    shift_leave: String,
}

#[derive(Debug, Deserialize, Default)]
struct BehaviorsFile {
    #[serde(default)]
    roles: HashMap<String, RoleBehaviorJson>,
}

static BEHAVIORS: OnceLock<RwLock<Option<BehaviorsFile>>> = OnceLock::new();

fn cache() -> &'static RwLock<Option<BehaviorsFile>> {
    BEHAVIORS.get_or_init(|| RwLock::new(None))
}

/// 載入 `npc_behaviors.json`；可重複呼叫覆寫快取。
pub fn try_load_npc_behaviors(path: &Path) -> anyhow::Result<()> {
    let raw = std::fs::read_to_string(path)
        .or_else(|_| std::fs::read_to_string(Path::new("..").join(path)))?;
    let f: BehaviorsFile = serde_json::from_str(&raw)?;
    let n = f.roles.len();
    *cache().write().expect("behaviors poisoned") = Some(f);
    tracing::info!(target: "npc_behavior", "loaded {n} roles from {}", path.display());
    Ok(())
}

/// 換班敘事（`arriving == true` 為上班）；無職稱或檔未載入時回空字串。
#[must_use]
pub fn get_shift_flavor(title: &str, npc_name: &str, arriving: bool) -> String {
    let g = cache().read().expect("behaviors poisoned");
    let Some(ref bd) = *g else {
        return String::new();
    };
    let Some(role) = bd.roles.get(title) else {
        return String::new();
    };
    let s = if arriving {
        &role.shift_arrive
    } else {
        &role.shift_leave
    };
    s.replace("{name}", npc_name)
}
