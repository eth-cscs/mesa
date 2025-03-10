use crate::hsm::{
    self,
    group::{
        hacks::{KEYCLOAK_ROLES_TO_IGNORE, PA_ADMIN},
        types::Group,
    },
};

use super::hacks::ROLES;

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

#[test]
fn test_validate_groups_tenant() {
    let cfs_session_groups: Vec<String> = vec![
        ROLES[0].to_string(),
        "my_group".to_string(),
        "my_group_cn".to_string(),
    ];

    dbg!(&cfs_session_groups);

    let auth_token_groups: Vec<String> = vec![
        KEYCLOAK_ROLES_TO_IGNORE[0].to_string(),
        KEYCLOAK_ROLES_TO_IGNORE[1].to_string(),
        "my_group".to_string(),
        "my_group_cn".to_string(),
    ];

    assert!(hsm::group::hacks::validate_groups(
        cfs_session_groups.as_slice(),
        auth_token_groups.as_slice()
    )
    .is_empty());
}

#[test]
fn test_validate_groups_tenant_fail() {
    let cfs_session_groups: Vec<String> = vec![
        ROLES[0].to_string(),
        "my_group".to_string(),
        "my_group_cn".to_string(),
        "unwanted".to_string(),
    ];

    dbg!(&cfs_session_groups);

    let auth_token_groups: Vec<String> = vec![
        KEYCLOAK_ROLES_TO_IGNORE[0].to_string(),
        KEYCLOAK_ROLES_TO_IGNORE[1].to_string(),
        "my_group".to_string(),
        "my_group_cn".to_string(),
    ];

    assert_eq!(
        vec!["unwanted".to_string()],
        hsm::group::hacks::validate_groups(
            cfs_session_groups.as_slice(),
            auth_token_groups.as_slice()
        )
    );
}

#[test]
fn test_validate_groups_admin() {
    let cfs_session_groups: Vec<String> = vec![
        ROLES[0].to_string(),
        "my_group".to_string(),
        "my_group_cn".to_string(),
    ];

    dbg!(&cfs_session_groups);

    let auth_token_groups: Vec<String> = vec![
        KEYCLOAK_ROLES_TO_IGNORE[0].to_string(),
        KEYCLOAK_ROLES_TO_IGNORE[1].to_string(),
        PA_ADMIN.to_string(),
        "my_group".to_string(),
        "my_group_cn".to_string(),
    ];

    assert!(hsm::group::hacks::validate_groups(
        cfs_session_groups.as_slice(),
        auth_token_groups.as_slice()
    )
    .is_empty());
}

#[test]
fn test_validate_groups_admin_2() {
    let cfs_session_groups: Vec<String> = vec![
        ROLES[0].to_string(),
        "my_group".to_string(),
        "my_group_cn".to_string(),
        "unwanted".to_string(),
    ];

    dbg!(&cfs_session_groups);

    let auth_token_groups: Vec<String> = vec![
        KEYCLOAK_ROLES_TO_IGNORE[0].to_string(),
        KEYCLOAK_ROLES_TO_IGNORE[1].to_string(),
        PA_ADMIN.to_string(),
        "my_group".to_string(),
        "my_group_cn".to_string(),
    ];

    assert!(hsm::group::hacks::validate_groups(
        cfs_session_groups.as_slice(),
        auth_token_groups.as_slice()
    )
    .is_empty());
}
