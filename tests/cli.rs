//! End-to-end CLI tests. These guard behaviour that the unit tests can't reach
//! because it spans the filesystem and the working directory.

use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_rustio-design");

/// A throwaway project dir under the temp directory, cleaned on drop.
struct Project(PathBuf);

impl Project {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("rio-cli-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        Project(dir)
    }
    fn write(&self, rel: &str, body: &str) {
        std::fs::write(self.0.join(rel), body).unwrap();
    }
    fn spec(&self) -> String {
        self.0
            .join("rustio.design.toml")
            .to_string_lossy()
            .into_owned()
    }
    /// Run from an unrelated working directory to prove paths resolve against the
    /// spec root, not the cwd.
    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(BIN)
            .args(args)
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Regression: `build`/`check` must resolve every artifact against the spec root
/// even when invoked from another directory via `--spec`. Previously tokens.css /
/// wire.env / .manifest were written relative to the cwd, so `--spec <subdir>`
/// split the output and `check` reported the artifacts as missing.
#[test]
fn build_and_check_resolve_output_against_spec_root() {
    let p = Project::new("root");
    p.write(
        "rustio.design.toml",
        "[project]\nname = \"T\"\n\n[views.bookings]\nmodel = \"Booking\"\nprimary = \"id\"\n",
    );
    p.write(
        "src/main.rs",
        "pub struct Booking { pub id: Uuid }\nfn main() {}\n",
    );

    let build = p.run(&["build", "--spec", &p.spec()]);
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(p.0.join("generated/tokens.css").exists());
    assert!(p.0.join("generated/wire.env").exists());
    assert!(p.0.join("generated/.manifest").exists());
    assert!(p.0.join("generated/views/bookings.view.json").exists());

    let check = p.run(&["check", "--spec", &p.spec()]);
    assert!(
        check.status.success(),
        "check failed: {}",
        String::from_utf8_lossy(&check.stderr)
    );
}

/// `schema --all` extracts every registered model's columns in one pass.
#[test]
fn schema_all_extracts_every_registered_model() {
    let p = Project::new("schema");
    p.write("rustio.design.toml", "[project]\nname = \"T\"\n");
    p.write(
        "src/main.rs",
        "pub struct Booking { pub id: Uuid, pub customer: String }\n\
         pub struct Invoice { pub total: i64 }\n\
         fn main() { App::new().model::<Booking>().model::<Invoice>().run(); }\n",
    );
    let out_dir = p.0.join("schemas");

    let r = p.run(&[
        "schema",
        "--all",
        "--spec",
        &p.spec(),
        "--source",
        "src/main.rs",
        "--out-dir",
        out_dir.to_str().unwrap(),
    ]);
    assert!(
        r.status.success(),
        "schema --all failed: {}",
        String::from_utf8_lossy(&r.stderr)
    );
    let bookings = std::fs::read_to_string(out_dir.join("bookings.json")).unwrap();
    assert!(bookings.contains("\"name\": \"customer\""));
    assert!(bookings.contains("\"type\": \"uuid\""));
    assert!(out_dir.join("invoices.json").exists());
}
