//! rustio-design — the Claude-friendly design bridge for rustio-admin.
//!
//! One declarative spec (`rustio.design.toml`) is the single source of truth for
//! a rustio-admin look. This binary validates it against the framework's design
//! doctrine and compiles it into a `tokens.css` the running admin serves via
//! `RUSTIO_TOKENS_CSS` — no recompile, no hand-edited CSS, no dizziness.
//!
//! Above the token layer sits the design-memory stack — the WHY and WHAT layers
//! (DESIGN_BRIEF/REASONING/ARCHITECTURE/DECISIONS/HISTORY) that Claude Design
//! reasons over BEFORE touching tokens. `context` assembles them into one stream;
//! the higher layers are reserved memory, not yet compiled into output.
//!
//! Commands:
//!   init     scaffold the spec + design-memory artifacts
//!   build    validate + generate artifacts into the output dir
//!   check    validate + detect drift/staleness (CI-friendly, read-only)
//!   context  assemble the whole design stack (WHY→WHAT→HOW) into one stream
//!   wire     print the env exports that serve the generated output
//!   explain  print the workflow and the iron rules

mod allowlist;
mod color;
mod context;
mod manifest;
mod sha256;
mod spec;
mod tokens;
mod toml_lite;
mod validate;

use manifest::Manifest;
use sha256::sha256_hex;
use spec::Spec;
use std::process::ExitCode;
use toml_lite::Document;

const DEFAULT_SPEC: &str = "rustio.design.toml";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("help");
    let spec_path = flag_value(&args, "--spec").unwrap_or_else(|| DEFAULT_SPEC.to_string());

    let result = match cmd {
        "init" => cmd_init(&spec_path),
        "build" => cmd_build(&spec_path),
        "check" => cmd_check(&spec_path),
        "context" => cmd_context(&spec_path),
        "wire" => cmd_wire(&spec_path),
        "explain" => {
            print_explain();
            Ok(())
        }
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        other => Err(format!(
            "unknown command `{other}` — try `rustio-design help`"
        )),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rustio-design: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Read the value following a `--flag` in the argument list, if present.
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let pos = args.iter().position(|a| a == flag)?;
    args.get(pos + 1).cloned()
}

// ─────────────────────────────────────────────────────────────────────────
// init
// ─────────────────────────────────────────────────────────────────────────

/// The design-memory artifacts scaffolded alongside the spec, embedded so
/// `init` is self-contained (mirrors rustio-admin's `include_str!` ethos).
const DESIGN_SCAFFOLD: &[(&str, &str)] = &[
    (
        "design/DESIGN_BRIEF.md",
        include_str!("../assets/design/DESIGN_BRIEF.md"),
    ),
    (
        "design/DESIGN_REASONING.md",
        include_str!("../assets/design/DESIGN_REASONING.md"),
    ),
    (
        "design/DESIGN_ARCHITECTURE.md",
        include_str!("../assets/design/DESIGN_ARCHITECTURE.md"),
    ),
    (
        "design/DESIGN_DECISIONS.md",
        include_str!("../assets/design/DESIGN_DECISIONS.md"),
    ),
    (
        "design/DESIGN_HISTORY.md",
        include_str!("../assets/design/DESIGN_HISTORY.md"),
    ),
];

/// Scaffold the spec and the design-memory artifacts.
///
/// Adoption-friendly and non-destructive: writes the spec only if absent, and
/// creates any missing `design/DESIGN_*.md` artifact without ever overwriting an
/// existing one. An existing project can run `init` to reserve the new design
/// layers without risking its spec or its authored memory.
fn cmd_init(spec_path: &str) -> Result<(), String> {
    let root = spec_root(spec_path);
    let mut created: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    if std::path::Path::new(spec_path).exists() {
        skipped.push(spec_path.to_string());
    } else {
        write_file(spec_path, STARTER_SPEC)?;
        created.push(spec_path.to_string());
    }

    for (rel, body) in DESIGN_SCAFFOLD {
        let path = root.join(rel);
        if path.exists() {
            skipped.push(path.display().to_string());
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
        }
        std::fs::write(&path, body)
            .map_err(|e| format!("could not write {}: {e}", path.display()))?;
        created.push(path.display().to_string());
    }

    for c in &created {
        println!("✓ created {c}");
    }
    for s in &skipped {
        println!("· kept    {s}  (already present)");
    }
    println!();
    println!("  design memory reserved. Capture intent in design/DESIGN_BRIEF.md,");
    println!("  reason in design/DESIGN_REASONING.md, then edit tokens and `build`.");
    println!("  See the whole stack any time with `rustio-design context`.");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// context  (assemble the WHY→WHAT→HOW design stack into one stream)
// ─────────────────────────────────────────────────────────────────────────

fn cmd_context(spec_path: &str) -> Result<(), String> {
    let (_src, spec) = load_spec(spec_path)?;
    let root = spec_root(spec_path);
    print!("{}", context::build_context(&spec, &root));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// build
// ─────────────────────────────────────────────────────────────────────────

fn cmd_build(spec_path: &str) -> Result<(), String> {
    let (src, spec) = load_spec(spec_path)?;

    let report = validate::validate(&spec);
    for w in &report.warnings {
        eprintln!("  warning: {w}");
    }
    if !report.is_ok() {
        for e in &report.errors {
            eprintln!("  error: {e}");
        }
        return Err(format!(
            "{} doctrine error(s) — nothing was generated",
            report.errors.len()
        ));
    }

    let ramp = derive_brand_ramp(&spec);
    let css = tokens::build_tokens_css(&spec, ramp.as_deref());

    let out_dir = spec.out_dir().to_string();
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("could not create {out_dir}/: {e}"))?;

    let tokens_path = format!("{out_dir}/tokens.css");
    write_file(&tokens_path, &css)?;

    let wire = wire_env_text(&tokens_path)?;
    write_file(&format!("{out_dir}/wire.env"), &wire)?;

    let readme = generated_readme(spec.project_name());
    write_file(&format!("{out_dir}/README.md"), &readme)?;

    let mut m = Manifest::new();
    m.set(spec_path, &sha256_hex(src.as_bytes()));
    m.set("tokens.css", &sha256_hex(css.as_bytes()));
    m.set("wire.env", &sha256_hex(wire.as_bytes()));
    m.set("README.md", &sha256_hex(readme.as_bytes()));
    write_file(&format!("{out_dir}/.manifest"), &m.to_text())?;

    println!("✓ built {tokens_path}");
    if ramp.is_some() {
        println!("  brand ramp: derived via rustio-admin rio-theme");
    }
    println!("  serve it:   rustio-design wire   (or see {out_dir}/wire.env)");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// check  (read-only; the CI gate)
// ─────────────────────────────────────────────────────────────────────────

fn cmd_check(spec_path: &str) -> Result<(), String> {
    let (src, spec) = load_spec(spec_path)?;
    let report = validate::validate(&spec);
    for w in &report.warnings {
        eprintln!("  warning: {w}");
    }

    // Design memory is active, not optional: surface a missing higher layer as a
    // warning so it can't quietly drift out of existence. Non-blocking.
    let root = spec_root(spec_path);
    let p = spec.design_paths();
    for (label, rel) in [
        ("brief", &p.brief),
        ("reasoning", &p.reasoning),
        ("architecture", &p.architecture),
        ("decisions", &p.decisions),
        ("history", &p.history),
    ] {
        if !root.join(rel).exists() {
            eprintln!(
                "  warning: design memory `{label}` missing ({rel}) — run `rustio-design init`"
            );
        }
    }

    let out_dir = spec.out_dir().to_string();
    let manifest_path = format!("{out_dir}/.manifest");
    let mtext = std::fs::read_to_string(&manifest_path)
        .map_err(|_| format!("no manifest at {manifest_path} — run `rustio-design build` first"))?;
    let m = Manifest::parse(&mtext);

    let mut problems = 0usize;

    for e in &report.errors {
        eprintln!("  error:   {e}");
        problems += 1;
    }

    // staleness: spec changed since last build?
    let spec_hash = sha256_hex(src.as_bytes());
    match m.get(spec_path) {
        Some(h) if h == spec_hash => {}
        _ => {
            eprintln!(
                "  stale:   {spec_path} changed since last build — run `rustio-design build`"
            );
            problems += 1;
        }
    }

    // drift: a generated file edited by hand?
    for (path, hash) in &m.entries {
        if path == spec_path {
            continue;
        }
        let full = format!("{out_dir}/{path}");
        match std::fs::read(&full) {
            Ok(bytes) if sha256_hex(&bytes) == *hash => {}
            Ok(_) => {
                eprintln!("  drift:   {full} was hand-edited — edit the spec and run `build`");
                problems += 1;
            }
            Err(_) => {
                eprintln!("  missing: {full} — run `rustio-design build`");
                problems += 1;
            }
        }
    }

    if problems > 0 {
        return Err(format!("{problems} problem(s) found"));
    }
    println!("✓ spec valid · output in sync · no drift");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// wire
// ─────────────────────────────────────────────────────────────────────────

fn cmd_wire(spec_path: &str) -> Result<(), String> {
    let (_src, spec) = load_spec(spec_path)?;
    let path = format!("{}/wire.env", spec.out_dir());
    let text = std::fs::read_to_string(&path)
        .map_err(|_| format!("no {path} — run `rustio-design build` first"))?;
    print!("{text}");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────────────────

/// The directory that holds the spec — the root the design-memory artifact
/// paths are resolved against. Empty/no parent resolves to the current dir.
fn spec_root(spec_path: &str) -> std::path::PathBuf {
    std::path::Path::new(spec_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

/// Read and parse the spec file into `(raw_source, Spec)`.
fn load_spec(spec_path: &str) -> Result<(String, Spec), String> {
    let src = std::fs::read_to_string(spec_path)
        .map_err(|_| format!("cannot read {spec_path} — run `rustio-design init` first"))?;
    let doc = Document::parse(&src).map_err(|e| format!("{spec_path}: {e}"))?;
    Ok((src, Spec::new(doc)))
}

/// Write a file, surfacing a clear error on failure.
fn write_file(path: &str, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| format!("could not write {path}: {e}"))
}

/// Delegate brand-ramp derivation to `rustio-admin theme generate`.
///
/// Only runs when `[brand].derive = true` and `[project].rustio_admin_path` is
/// set. Any failure degrades gracefully to the literal brand color.
fn derive_brand_ramp(spec: &Spec) -> Option<String> {
    if !spec.brand_derive() {
        return None;
    }
    let path = spec.rustio_admin_path()?;
    let brand = spec.brand_color()?;
    let tmp = std::env::temp_dir().join("rustio-design-brand-ramp.css");

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "rustio-admin-cli",
            "--",
            "theme",
            "generate",
            "--brand",
            brand,
            "--out",
        ])
        .arg(&tmp)
        .current_dir(path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let css = std::fs::read_to_string(&tmp).ok();
            let _ = std::fs::remove_file(&tmp);
            if css.is_none() {
                eprintln!("  warning: brand ramp produced no file — using literal brand color");
            }
            css
        }
        Ok(o) => {
            eprintln!(
                "  warning: `rustio-admin theme generate` failed ({}) — using literal brand color",
                o.status
            );
            None
        }
        Err(e) => {
            eprintln!("  warning: could not run cargo in {path} ({e}) — using literal brand color");
            None
        }
    }
}

/// Build the `wire.env` contents (an absolute `RUSTIO_TOKENS_CSS` export).
fn wire_env_text(tokens_path: &str) -> Result<String, String> {
    let abs = std::fs::canonicalize(tokens_path)
        .map_err(|e| format!("could not resolve {tokens_path}: {e}"))?;
    let abs = abs.to_string_lossy();
    Ok(format!(
        "# Generated by rustio-design — source this to serve the design without\n\
         # recompiling rustio-admin. Re-run `rustio-design build` to refresh.\n\
         #\n\
         #   source {out}/wire.env && cargo run\n\
         #\n\
         export RUSTIO_TOKENS_CSS=\"{abs}\"\n",
        out = tokens_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("."),
    ))
}

/// The README dropped into the output directory as a guard rail.
fn generated_readme(project: &str) -> String {
    format!(
        "# generated/ — DO NOT EDIT\n\
         \n\
         Every file in this directory is generated by **rustio-design** for\n\
         project **{project}** from `../rustio.design.toml`.\n\
         \n\
         Editing files here is pointless and unsafe:\n\
         \n\
         * the next `rustio-design build` overwrites them, and\n\
         * `rustio-design check` flags the change as *drift* and fails CI.\n\
         \n\
         To change the design, edit `rustio.design.toml` and run\n\
         `rustio-design build`.\n\
         \n\
         To serve `tokens.css` without recompiling rustio-admin:\n\
         \n\
         ```sh\n\
         source generated/wire.env && cargo run\n\
         ```\n"
    )
}

// ─────────────────────────────────────────────────────────────────────────
// text
// ─────────────────────────────────────────────────────────────────────────

fn print_help() {
    println!(
        "rustio-design — the design bridge for rustio-admin\n\
         \n\
         USAGE:\n\
         \x20 rustio-design <command> [--spec <path>]\n\
         \n\
         COMMANDS:\n\
         \x20 init      Scaffold the spec + design-memory artifacts (DESIGN_*.md)\n\
         \x20 build     Validate the spec and generate tokens.css + manifest\n\
         \x20 check     Validate + detect drift/staleness (read-only, CI-friendly)\n\
         \x20 context   Assemble the design stack (WHY→WHAT→HOW) into one stream\n\
         \x20 wire      Print the env export that serves the generated output\n\
         \x20 explain   Print the workflow and the iron rules\n\
         \x20 help      Show this message\n\
         \n\
         THE STACK: Brief → Reasoning → Architecture → Spec → Generated.\n\
         THE ONE RULE: edit rustio.design.toml; never hand-edit generated/.\n\
         Reason in DESIGN_REASONING.md BEFORE changing tokens (`/design-reason`)."
    );
}

fn print_explain() {
    println!(
        "The rustio-design stack — design memory, not just tokens\n\
         =======================================================\n\
         \n\
         WHY        design/DESIGN_BRIEF.md         business context, intent, visual direction\n\
         Reasoning  design/DESIGN_REASONING.md     the ADR trail — justify BEFORE the spec\n\
         WHAT       design/DESIGN_ARCHITECTURE.md  information architecture, navigation, hierarchy\n\
         Memory     design/DESIGN_DECISIONS.md     the durable ledger of accepted decisions\n\
         Memory     design/DESIGN_HISTORY.md       how (and why) the design evolved\n\
         HOW        rustio.design.toml             the validated token spec\n\
         Output     generated/tokens.css           machine-owned; never hand-edited\n\
         \n\
         Reason TOP-DOWN (WHY → WHAT → HOW); generate BOTTOM-UP. Today only the\n\
         HOW layer compiles to output — the WHY/WHAT layers are active design\n\
         memory, surfaced together by `rustio-design context`.\n\
         \n\
         Flow (mirrors RustIO's Plan → Review → Approve → Apply):\n\
         \x20 Brief → Reasoning → Architecture → Spec → Generated\n\
         \n\
         Why this removes the dizziness:\n\
         \n\
         1. ONE file to edit. The strict, multi-file rustio-admin CSS — and its\n\
            lock-step @import / include_str! lists — is never touched. You change\n\
            tokens, not source.\n\
         2. Typos can't silently fail. Every key is checked against the real\n\
            --rio-* vocabulary; an unknown token is rejected with a suggestion.\n\
         3. Unreadable colors can't ship. Literal text colors are held to WCAG\n\
            contrast before anything is written.\n\
         4. Generated files are tamper-evident. `check` fingerprints them, so a\n\
            stray hand-edit (drift) or a forgotten rebuild (staleness) fails CI.\n\
         5. No recompile to preview. `source generated/wire.env && cargo run`.\n\
         \n\
         For a WCAG-safe brand ramp, set [project].rustio_admin_path and\n\
         [brand].derive = true; the heavy color math is delegated to the\n\
         framework's own rio-theme engine — never reimplemented here."
    );
}

/// The starter spec written by `init`. Uses only the grammar `toml_lite` accepts.
const STARTER_SPEC: &str = r##"# ════════════════════════════════════════════════════════════════════
#  rustio.design.toml — the single source of truth for your admin's look.
#
#  THE ONE RULE: edit THIS file only. Never touch anything under
#  generated/. After editing, run:        rustio-design build
#  Verify any time (CI-friendly, read-only): rustio-design check
# ════════════════════════════════════════════════════════════════════

[project]
name = "My Admin"
# Where generated artifacts go (served via RUSTIO_TOKENS_CSS):
out_dir = "generated"
# Point this at your rustio-admin checkout to unlock the WCAG-safe brand
# ramp ([brand].derive). The token overrides below work with or without it.
# rustio_admin_path = "../rustio-admin"

# ── Design memory (the WHY and WHAT layers) ──────────────────────────
# Claude Design reasons from these BEFORE touching tokens. They are
# first-class design memory, not documentation: `rustio-design context`
# assembles them into one narrative; `init` scaffolds them. Paths below
# are the defaults — change them only if you relocate the files.
[design]
brief        = "design/DESIGN_BRIEF.md"         # WHY  — context, intent, visual direction
reasoning    = "design/DESIGN_REASONING.md"     # the ADR trail (justify before the spec)
architecture = "design/DESIGN_ARCHITECTURE.md"  # WHAT — IA, navigation, UX hierarchy
decisions    = "design/DESIGN_DECISIONS.md"     # accepted-decision ledger
history      = "design/DESIGN_HISTORY.md"       # evolution over time

[brand]
# Your primary brand color. With derive = true (and rustio_admin_path set),
# rustio-admin's rio-theme engine computes a contrast-safe ramp for you.
color = "#2563eb"
# derive = true

# ── Token overrides ──────────────────────────────────────────────────
# Keys are validated against rustio-admin's real --rio-* vocabulary, so a
# typo is rejected (with a "did you mean") before anything is written —
# an override can never silently do nothing.

[colors]
# text-strong = "#0f172a"
# danger      = "#dc2626"

[radius]
default = "8px"
sm      = "5px"
lg      = "12px"

[spacing]
# sidebar-w   = "264px"
# content-max = "1440px"

[typography]
# font-sans = "'Inter', system-ui, -apple-system, sans-serif"

[custom_css]
# Escape hatch for the rare rule with no token. Validated: no @import, no
# remote url(), no markup. Prefer tokens above whenever possible.
# rules = """
# .rio-sidebar { letter-spacing: 0.01em; }
# """
"##;
