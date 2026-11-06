pub fn clean_npc_line(line: &str) -> String {
    crate::ai::sanitize::sanitize_npc_dialogue_line(line)
}
