#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_lookup_and_helpers() {
        let entry = find_catalog_entry("wt_cat_0001").expect("should find by id");
        assert_eq!(entry.name, "赤鐵重礦");
        assert_eq!(find_catalog_entry("三號坑赤鐵重礦").map(|e| e.id), Some("wt_cat_0001"));
        assert_eq!(catalog_entry_weight_kg(entry), 1.5);
        assert_eq!(catalog_entry_price_mg(entry), 3);

        // 驗證 1,109 件物件的屬性標籤 attribute_tags
        for item in PENINSULA_WORLD_OBJECTS {
            assert!(
                item.attribute_tags.len() >= 3,
                "Item {} attribute_tags len {} < 3",
                item.id,
                item.attribute_tags.len()
            );
            for &attr in item.attribute_tags {
                assert!(
                    !attr.is_empty(),
                    "Item {} has empty attribute tag",
                    item.id
                );
                assert!(
                    attr.chars().next().unwrap().is_ascii_lowercase(),
                    "Item {} attribute tag '{}' must start with lowercase a-z",
                    item.id,
                    attr
                );
                assert!(
                    attr.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                    "Item {} attribute tag '{}' must be lowercase ascii/underscore",
                    item.id,
                    attr
                );
            }
        }

        // 驗證 1,109 件物件的專屬英文標籤 object_tag
        let mut object_tags = std::collections::HashSet::new();
        for item in PENINSULA_WORLD_OBJECTS {
            assert!(
                !item.object_tag.is_empty(),
                "Item {} has empty object_tag",
                item.id
            );
            assert!(
                item.object_tag.chars().next().unwrap().is_ascii_lowercase(),
                "Item {} object_tag '{}' must start with lowercase a-z",
                item.id,
                item.object_tag
            );
            assert!(
                item.object_tag.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "Item {} object_tag '{}' contains invalid chars",
                item.id,
                item.object_tag
            );
            assert!(
                object_tags.insert(item.object_tag),
                "Duplicate object_tag '{}' found on item {}",
                item.object_tag,
                item.id
            );
        }
        assert_eq!(object_tags.len(), PENINSULA_WORLD_OBJECTS.len());

        // 驗證 1,109 件物件的分類標籤 category_tag 與超類標籤
        for item in PENINSULA_WORLD_OBJECTS {
            assert!(
                !item.category_tag.is_empty(),
                "Item {} has empty category_tag",
                item.id
            );
            assert!(
                item.category_tag.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "Item {} category_tag '{}' must be lowercase ascii/underscore",
                item.id,
                item.category_tag
            );
            assert_eq!(
                item.category_tag,
                category_tag_for_num(item.category_num),
                "Item {} category_tag mismatch with category_tag_for_num({})",
                item.id,
                item.category_num
            );
            let supers = super_category_tags_for_category(item.category_tag);
            assert!(
                !supers.is_empty(),
                "Item {} category_tag '{}' has empty super category tags",
                item.id,
                item.category_tag
            );
            let all_tags = item.tags();
            assert!(all_tags.contains(&item.object_tag));
            assert!(all_tags.contains(&item.category_tag));
            assert!(item.has_tag(item.category_tag));
            assert!(item.has_tag(item.object_tag));
            for &st in supers {
                assert!(all_tags.contains(&st));
                assert!(item.has_tag(st));
            }
        }

        // 驗證 find_catalog_entry_by_id_or_name
        let found_by_id = find_catalog_entry_by_id_or_name("wt_cat_0001", "").expect("find by id");
        assert_eq!(found_by_id.object_tag, "heavy_hematite_ore");
        let found_by_name = find_catalog_entry_by_id_or_name("", "赤鐵重礦").expect("find by name");
        assert_eq!(found_by_name.id, "wt_cat_0001");
        let found_by_short = find_catalog_entry_by_id_or_name("", "陳年醬菜罈").expect("find by short");
        assert_eq!(found_by_short.id, "wt_cat_0372");

        let deed = find_catalog_entry("wt_cat_0793").expect("deed entry exists");
        assert_eq!(catalog_entry_price_mg(deed), 50);

        let heavy = find_catalog_entry("wt_cat_0555").expect("cauldron exists");
        assert!(!is_portable_catalog_entry(heavy));

        let portable = find_catalog_entry("wt_cat_0561").expect("cup exists");
        assert!(is_portable_catalog_entry(portable));
        let room_obj = portable.to_room_object();
        assert!(room_obj.sockets.contains(&"Acquire".to_string()));

        // 驗證 short_scene_name 與 find_catalog_entry 支援短名搜尋
        assert_eq!(entry.short_scene_name(), "赤鐵重礦");
        assert_eq!(find_catalog_entry("赤鐵重礦").map(|e| e.id), Some("wt_cat_0001"));

        // 驗證 wt_cat_0372 正確修正
        let cat_0372 = find_catalog_entry("wt_cat_0372").expect("wt_cat_0372 exists");
        assert_eq!(cat_0372.category_name, "油鹽醬醋醃漬醬菜與糖蜜調味");
        assert_eq!(cat_0372.name, "陳年醬菜罈");
        assert_eq!(cat_0372.short_scene_name(), "陳年醬菜罈");

        // 驗證民宅後綴房間標籤對齊
        assert_eq!(room_tag_for_peninsula_room("松風居"), "residence");
        assert_eq!(room_tag_for_peninsula_room("陳宅"), "residence");
        assert_eq!(room_tag_for_peninsula_room("靜心齋"), "residence");
        assert_eq!(room_tag_for_peninsula_room("草廬"), "residence");
        assert_eq!(room_tag_for_peninsula_room("小院"), "residence");
        assert_eq!(room_tag_for_peninsula_room("西舍"), "residence");
        assert_eq!(room_tag_for_peninsula_room("李家"), "residence");
        assert_eq!(room_tag_for_peninsula_room("水雲築"), "residence");
        assert_eq!(room_tag_for_peninsula_room("漁寮"), "residence");
        assert_eq!(room_tag_for_peninsula_room("東房"), "residence");
        assert_eq!(room_tag_for_peninsula_room("林莊"), "residence");
        assert_eq!(room_tag_for_peninsula_room("石屋"), "residence");
        assert_eq!(room_tag_for_peninsula_room("望海樓"), "residence");

        // 驗證全目錄 1,109 件物件 to_room_object 的短名長度皆在 3..=5 字元且無「與」
        for item in PENINSULA_WORLD_OBJECTS {
            let obj = item.to_room_object();
            let len = obj.name.chars().count();
            assert!(
                (3..=5).contains(&len),
                "Item {} short name '{}' (from '{}') length {} not in 3..=5",
                item.id,
                obj.name,
                item.name,
                len
            );
            assert!(
                !obj.name.contains('與'),
                "Item {} short name '{}' contains '與'",
                item.id,
                obj.name
            );
        }
    }

    #[test]
    fn test_uniform_and_semantic_tags_objectification() {
        let apron = find_catalog_entry("wt_cat_0840").expect("apron entry");
        assert!(apron.is_work_uniform_object());
        assert!(is_work_uniform_item_id_or_name("wt_cat_0840"));
        assert!(is_work_uniform_item_id_or_name("鍛造坊厚牛皮防燙圍裙"));

        let tags = apron.semantic_tags();
        assert!(tags.contains(&"uniform"));
        assert!(tags.contains(&"fabric"));

        assert!(is_work_uniform_item_id_or_name("店員制服"));
        assert!(!is_work_uniform_item_id_or_name("赤鐵重礦"));

        let hammer = find_catalog_entry("wt_cat_0957").expect("hammer entry");
        let hammer_tags = hammer.semantic_tags();
        assert!(hammer_tags.contains(&"tool"));

        let lamp = find_catalog_entry("wt_cat_0723").expect("lamp entry");
        assert!(!lamp.is_work_uniform_object());
    }
}
