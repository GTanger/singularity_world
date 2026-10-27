use std::collections::HashMap;

use super::NpcSnapshot;

pub(super) fn occupants_by_place(
    snapshots: &HashMap<String, NpcSnapshot>,
) -> HashMap<String, Vec<String>> {
    let mut occupants: HashMap<String, Vec<String>> = HashMap::new();
    for (id, snapshot) in snapshots {
        occupants
            .entry(snapshot.place_id.clone())
            .or_default()
            .push(id.clone());
    }
    for ids in occupants.values_mut() {
        ids.sort();
    }
    occupants
}

pub(super) fn relocate_occupant(
    occupants: &mut HashMap<String, Vec<String>>,
    entity_id: &str,
    from: &str,
    to: &str,
) {
    if from == to {
        return;
    }
    let remove_source = if let Some(ids) = occupants.get_mut(from) {
        ids.retain(|id| id != entity_id);
        ids.is_empty()
    } else {
        false
    };
    if remove_source {
        occupants.remove(from);
    }
    let ids = occupants.entry(to.to_string()).or_default();
    if !ids.iter().any(|id| id == entity_id) {
        ids.push(entity_id.to_string());
        ids.sort();
    }
}
