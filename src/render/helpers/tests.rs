use super::radical_stack_counts;

#[test]
fn radical_stack_counts_groups_duplicate_radicals() {
    let counts = radical_stack_counts(&["水", "木", "水"]);

    assert_eq!(counts.get("水"), Some(&2));
    assert_eq!(counts.get("木"), Some(&1));
    assert_eq!(counts.len(), 2);
}

#[test]
fn radical_stack_counts_returns_empty_map_for_empty_inventory() {
    let counts = radical_stack_counts(&[]);

    assert!(counts.is_empty());
}

