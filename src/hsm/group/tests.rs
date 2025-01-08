use crate::hsm::group::types::Group;

#[test]
fn test_add_xnames() {
    let mut group = Group::new("label", Some(vec!["xname1", "xname2"]));

    let mut new_xnames = vec!["xname3".to_string(), "xname4".to_string()];

    group.add_xnames(&mut new_xnames);

    assert_eq!(
        group.get_members(),
        vec![
            "xname1".to_string(),
            "xname2".to_string(),
            "xname3".to_string(),
            "xname4".to_string()
        ]
    )
}
