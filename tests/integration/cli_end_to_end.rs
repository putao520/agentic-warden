use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

struct TempHome {
    dir: TempDir,
    old_home: Option<std::ffi::OsString>,
    old_user: Option<std::ffi::OsString>,
}

impl TempHome {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().to_path_buf();
        let old_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &path);
        let old_user = std::env::var_os("USERPROFILE");
        if cfg!(windows) {
            std::env::set_var("USERPROFILE", &path);
        }
        Self {
            dir,
            old_home,
            old_user,
        }
    }

    fn command(&self) -> Command {
        let mut cmd = Command::cargo_bin("agentic-warden").expect("binary built");
        cmd.env("HOME", self.dir.path());
        if cfg!(windows) {
            cmd.env("USERPROFILE", self.dir.path());
        }
        cmd.env("RUST_LOG", "off");
        cmd
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        if let Some(old) = self.old_home.take() {
            std::env::set_var("HOME", old);
        } else {
            std::env::remove_var("HOME");
        }
        if cfg!(windows) {
            if let Some(old) = self.old_user.take() {
                std::env::set_var("USERPROFILE", old);
            } else {
                std::env::remove_var("USERPROFILE");
            }
        }
    }
}

#[test]
fn help_command_displays_usage_information() {
    let home = TempHome::new();
    let mut cmd = home.command();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("USAGE"))
        .stdout(predicate::str::contains("agentic-warden"));
}

#[test]
fn examples_command_outputs_quick_start_examples() {
    let home = TempHome::new();
    let mut cmd = home.command();
    cmd.arg("examples");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("QUICK START EXAMPLES"))
        .stdout(predicate::str::contains("agentic-warden push"));
}

#[test]
fn contextual_help_for_push_command() {
    let home = TempHome::new();
    let mut cmd = home.command();
    cmd.args(["help", "push"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("PUSH COMMAND"))
        .stdout(predicate::str::contains("agentic-warden push"));
}

#[test]
fn roles_list_outputs_available_roles() {
    let home = TempHome::new();
    let role_dir = home.dir.path().join(".aiw").join("role");
    fs::create_dir_all(&role_dir).expect("role dir created");

    let role_path = role_dir.join("dev.md");
    fs::write(&role_path, "Developer\n------------\nBuild things.")
        .expect("role file written");

    let mut cmd = home.command();
    cmd.args(["roles", "list"]);

    let role_path_str = role_path.display().to_string();
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Available roles (1):"))
        .stdout(predicate::str::contains("- dev: Developer"))
        .stdout(predicate::str::contains(role_path_str.as_str()));
}
