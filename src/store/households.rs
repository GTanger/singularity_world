#[derive(Debug, Clone, Copy)]
pub struct WenyaoHouseholdSlot {
    pub home_x: i32,
    pub home_y: i32,
    pub home_name: &'static str,
    pub home_stay_activity: &'static str,
    pub morning_hour: i32,
    pub evening_hour: i32,
    pub work_x: i32,
    pub work_y: i32,
    pub work_name: &'static str,
    pub work_purpose: &'static str,
    pub work_stay_activity: &'static str,
    pub errand_x: i32,
    pub errand_y: i32,
    pub errand_name: &'static str,
    pub errand_purpose: &'static str,
    pub errand_stay_activity: &'static str,
    pub attire_line: &'static str,
    pub brief_tag: &'static str,
}

impl WenyaoHouseholdSlot {
    #[inline]
    pub fn name(&self) -> &'static str {
        self.brief_tag
    }

    #[inline]
    pub fn held_vessel(&self) -> &'static str {
        let role_hint = if !self.brief_tag.is_empty() {
            self.brief_tag
        } else {
            self.work_name
        };
        crate::npc::causal_core::resolve_vocational_vessel(role_hint).item_id
    }
}

include!("households/part_01.rs");
include!("households/part_02.rs");
include!("households/part_03.rs");
include!("households/part_04.rs");
include!("households/part_05.rs");

const TOTAL_WENYAO: usize = PART_01.len()
    + PART_02.len()
    + PART_03.len()
    + PART_04.len()
    + PART_05.len();

const fn concat_wenyao_slots() -> [WenyaoHouseholdSlot; TOTAL_WENYAO] {
    let mut out = [PART_01[0]; TOTAL_WENYAO];
    let mut i = 0;
    let mut j = 0;
    while j < PART_01.len() {
        out[i] = PART_01[j];
        i += 1;
        j += 1;
    }
    j = 0;
    while j < PART_02.len() {
        out[i] = PART_02[j];
        i += 1;
        j += 1;
    }
    j = 0;
    while j < PART_03.len() {
        out[i] = PART_03[j];
        i += 1;
        j += 1;
    }
    j = 0;
    while j < PART_04.len() {
        out[i] = PART_04[j];
        i += 1;
        j += 1;
    }
    j = 0;
    while j < PART_05.len() {
        out[i] = PART_05[j];
        i += 1;
        j += 1;
    }
    out
}

static ALL_WENYAO_HOUSEHOLD_SLOTS: [WenyaoHouseholdSlot; TOTAL_WENYAO] = concat_wenyao_slots();
pub static WENYAO_HOUSEHOLD_SLOTS: &[WenyaoHouseholdSlot] = &ALL_WENYAO_HOUSEHOLD_SLOTS;
