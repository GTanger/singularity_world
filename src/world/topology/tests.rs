use super::*;

#[test]
fn builtin_topology_is_accepted_and_has_one_root() -> Result<()> {
    let topology = builtin_white_tower_topology()?;
    assert!(topology.canon);
    assert_eq!(topology.status, "accepted");
    assert_eq!(
        topology
            .nodes
            .iter()
            .filter(|node| node.parent_ref.is_none())
            .count(),
        1
    );
    Ok(())
}

#[test]
fn builtin_topology_contains_every_accepted_macro_anchor() -> Result<()> {
    let topology = builtin_white_tower_topology()?;
    let names: BTreeSet<&str> = topology
        .nodes
        .iter()
        .map(|node| node.display_name.as_str())
        .collect();
    for expected in [
        "白塔",
        "北冕海",
        "風舶嶼",
        "天然港灣道",
        "風舶角",
        "文曜",
        "西南原",
        "青脊山",
        "東海",
        "黑沼林",
        "南哨",
        "青籠關",
    ] {
        assert!(names.contains(expected), "缺少已接受大節點：{expected}");
    }
    assert!(
        names.contains("白塔遺蹟") || names.contains("白塔山麓") || names.contains("白塔古蹟") || names.contains("白塔"),
        "缺少白塔大節點"
    );
    Ok(())
}

#[test]
fn structural_labels_do_not_leak_to_the_display_surface() -> Result<()> {
    let topology = builtin_white_tower_topology()?;
    for node in &topology.nodes {
        for rejected in ["中部三河", "乾燥高河階", "沖積平原", "南部深沼水網"] {
            assert!(
                !node.display_name.contains(rejected),
                "{} 仍使用設計稿結構句：{}",
                node.id,
                node.display_name
            );
        }
    }
    Ok(())
}

#[test]
fn topology_preserves_water_flow_and_hard_east_coast() -> Result<()> {
    let topology = builtin_white_tower_topology()?;
    assert!(topology.links.iter().any(|link| {
        link.from_ref == "wt-c-central-lower-river"
            && link.to_ref == "wt-s-black-marsh-north-wet-forest"
            && link.relation == "drains_into"
    }));
    assert!(topology.links.iter().any(|link| {
        link.from_ref == "wt-e-green-ridge-east-coast"
            && link.to_ref == "wt-e-east-sea"
            && link.relation == "directly_meets"
    }));
    Ok(())
}

#[test]
fn validation_rejects_unknown_link_endpoint() -> Result<()> {
    let mut topology = builtin_white_tower_topology()?;
    topology.links[0].to_ref = "missing".to_string();
    let error = validate_topology(&topology).expect_err("未知端點必須被拒絕");
    assert!(error.to_string().contains("to_ref 不存在"));
    Ok(())
}
