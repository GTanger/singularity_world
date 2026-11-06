//! 物件生命週期（原料 → 成品 → 異變）與自主經濟循環模組。
//!
//! 定義白塔半島全鏈路物質演化：
//! 1. 原料（raw）：採集所得自然物質與粗料。
//! 2. 成品（finished）：工坊匠師以百工技藝切磋琢磨製成之各類器物與靈藥。
//! 3. 異變（mutated）：在天地氣脈共鳴或特定氣場淬火下，引發質理蛻變之神異奇珍。
//! 4. 自主經濟循環（LIVE_ECONOMY_LEDGER）：動態供需庫存、價格浮動與自主工造事件。

use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::db;
use crate::store::{self, Item};


include!("object_ecology/economy_ledger.rs");
include!("object_ecology/market_pricing.rs");
include!("object_ecology/supply_and_catalog.rs");
include!("object_ecology/item_definitions.rs");
include!("object_ecology/crafting_and_mutation.rs");
include!("object_ecology/harvest_and_production.rs");
include!("object_ecology/campfires_and_refining.rs");
include!("object_ecology/facilities_and_tests.rs");
