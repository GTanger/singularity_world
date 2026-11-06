use super::households::WenyaoHouseholdSlot;

include!("outer_households/part_01.rs");
include!("outer_households/part_02.rs");
include!("outer_households/part_03.rs");
include!("outer_households/part_04.rs");
include!("outer_households/part_05.rs");
include!("outer_households/part_06.rs");
include!("outer_households/part_07.rs");
include!("outer_households/part_08.rs");
include!("outer_households/part_09.rs");
include!("outer_households/part_10.rs");

const TOTAL_OUTER: usize = PART_01.len()
    + PART_02.len()
    + PART_03.len()
    + PART_04.len()
    + PART_05.len()
    + PART_06.len()
    + PART_07.len()
    + PART_08.len()
    + PART_09.len()
    + PART_10.len();

const fn concat_outer_slots() -> [WenyaoHouseholdSlot; TOTAL_OUTER] {
    let mut out = [PART_01[0]; TOTAL_OUTER];
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
    j = 0;
    while j < PART_06.len() {
        out[i] = PART_06[j];
        i += 1;
        j += 1;
    }
    j = 0;
    while j < PART_07.len() {
        out[i] = PART_07[j];
        i += 1;
        j += 1;
    }
    j = 0;
    while j < PART_08.len() {
        out[i] = PART_08[j];
        i += 1;
        j += 1;
    }
    j = 0;
    while j < PART_09.len() {
        out[i] = PART_09[j];
        i += 1;
        j += 1;
    }
    j = 0;
    while j < PART_10.len() {
        out[i] = PART_10[j];
        i += 1;
        j += 1;
    }
    out
}

static ALL_OUTER_HOUSEHOLD_SLOTS: [WenyaoHouseholdSlot; TOTAL_OUTER] = concat_outer_slots();
pub static OUTER_HOUSEHOLD_SLOTS: &[WenyaoHouseholdSlot] = &ALL_OUTER_HOUSEHOLD_SLOTS;
