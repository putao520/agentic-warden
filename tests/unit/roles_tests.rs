use aiw::roles::{RoleError, RoleManager};
use std::collections::HashSet;
use std::fs;

fn write_role_file(dir: &std::path::Path, name: &str, description: &str, content: &str) {
    let data = format!("{description}\n------------\n{content}");
    let path = dir.join(format!("{name}.md"));
    fs::write(path, data).expect("failed to write role file");
}

#[test]
fn list_all_roles_returns_every_markdown_role() {
    let tmp_dir = tempfile::tempdir().expect("failed to create temp dir");
    write_role_file(
        tmp_dir.path(),
        "alpha",
        "Alpha description",
        "alpha content",
    );
    write_role_file(tmp_dir.path(), "beta", "Beta description", "beta content");

    let manager = RoleManager::with_base_dir(tmp_dir.path()).expect("manager init failed");
    let roles = manager.list_all_roles().expect("listing should succeed");

    let names: HashSet<_> = roles.into_iter().map(|r| r.name).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains("alpha"));
    assert!(names.contains("beta"));
}

#[test]
fn get_role_returns_not_found_for_missing_file() {
    let tmp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let manager = RoleManager::with_base_dir(tmp_dir.path()).expect("manager init failed");

    let err = manager
        .get_role("missing")
        .expect_err("should error for missing role");
    match err {
        RoleError::NotFound(name) => assert_eq!(name, "missing"),
        other => panic!("unexpected error: {}", other),
    }
}

#[test]
fn rejects_path_traversal_in_role_name() {
    let tmp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let manager = RoleManager::with_base_dir(tmp_dir.path()).expect("manager init failed");

    let err = manager
        .get_role("../escape")
        .expect_err("should reject traversal attempt");

    match err {
        RoleError::InvalidName { .. } => {}
        other => panic!("unexpected error type: {}", other),
    }
}

#[test]
fn enforces_strict_character_set_in_role_names() {
    let tmp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let manager = RoleManager::with_base_dir(tmp_dir.path()).expect("manager init failed");

    // Valid names: alphanumeric, underscore, hyphen
    let valid_names = vec![
        "valid-role",
        "valid_role",
        "ValidRole123",
        "role-with-123_test",
    ];
    for name in valid_names {
        write_role_file(tmp_dir.path(), name, "Test description", "Test content");
        assert!(
            manager.get_role(name).is_ok(),
            "Valid role name '{}' should be accepted",
            name
        );
    }

    // Invalid names: special characters
    let invalid_names = vec![
        "role@test",
        "role.test",
        "role space",
        "role/test",
        "role\\test",
        "role!test",
    ];
    for name in invalid_names {
        let err = manager
            .get_role(name)
            .expect_err(&format!("Invalid role name '{}' should be rejected", name));

        match err {
            RoleError::InvalidName { message } => {
                assert!(
                    message.contains("invalid character") || message.contains("Only alphanumeric"),
                    "Error message should mention invalid character, got: {}",
                    message
                );
            }
            other => panic!("Expected InvalidName error for '{}', got: {}", name, other),
        }
    }
}
