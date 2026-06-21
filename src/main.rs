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
mod nav;
mod schema;
mod sha256;
mod spec;
mod tokens;
mod toml_lite;
mod validate;
mod views;

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
        "schema" => cmd_schema(&args, &spec_path),
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

    // --- navigation layer (WHAT → output); refuse on structural errors ---
    let root = spec_root(spec_path);
    let navigation = spec.navigation();
    if let Some(nav) = &navigation {
        let models = read_project_models(&root);
        let lint = nav::lint(nav, models.as_deref());
        for w in &lint.warnings {
            eprintln!("  warning: {w}");
        }
        if !lint.errors.is_empty() {
            for e in &lint.errors {
                eprintln!("  error: {e}");
            }
            return Err(format!(
                "{} navigation error(s) — nothing was generated",
                lint.errors.len()
            ));
        }
    }

    // --- view layer (WHAT → output); refuse on structural errors ---
    let views = spec.views();
    for view in &views {
        let columns = resolve_view_columns(&root, view);
        let lint = views::lint(view, columns.as_deref());
        for w in &lint.warnings {
            eprintln!("  warning: {w}");
        }
        if !lint.errors.is_empty() {
            for e in &lint.errors {
                eprintln!("  error: {e}");
            }
            return Err(format!(
                "{} view error(s) — nothing was generated",
                lint.errors.len()
            ));
        }
    }

    let ramp = derive_brand_ramp(&spec);
    let css = tokens::build_tokens_css(&spec, ramp.as_deref());

    // All artifacts resolve against the spec root, so `build`/`check` agree no
    // matter the working directory. Manifest *keys* stay root-relative (so the
    // drift check's `root.join(key)` round-trips); filesystem writes use the
    // absolute path. With the spec at the repo root, `root` is `.` and this is a
    // no-op — only `--spec <subdir>/…` runs change.
    let out_dir = spec.out_dir().to_string();
    let out_abs = root.join(&out_dir);
    std::fs::create_dir_all(&out_abs)
        .map_err(|e| format!("could not create {}: {e}", out_abs.display()))?;

    let mut m = Manifest::new();
    m.set(spec_path, &sha256_hex(src.as_bytes()));

    let tokens_path = format!("{out_dir}/tokens.css");
    write_file(&out_abs.join("tokens.css").to_string_lossy(), &css)?;
    m.set(&tokens_path, &sha256_hex(css.as_bytes()));

    let wire = wire_env_text(&out_abs.join("tokens.css").to_string_lossy())?;
    write_file(&out_abs.join("wire.env").to_string_lossy(), &wire)?;
    m.set(&format!("{out_dir}/wire.env"), &sha256_hex(wire.as_bytes()));

    let readme = generated_readme(spec.project_name());
    write_file(&out_abs.join("README.md").to_string_lossy(), &readme)?;
    m.set(
        &format!("{out_dir}/README.md"),
        &sha256_hex(readme.as_bytes()),
    );

    let mut nav_msg = None;
    if let Some(nav) = &navigation {
        let html = nav::build_sidebar_html(spec.project_name(), nav);
        let nav_path = root.join(&nav.out);
        if let Some(parent) = nav_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
        }
        std::fs::write(&nav_path, &html)
            .map_err(|e| format!("could not write {}: {e}", nav_path.display()))?;
        m.set(&nav.out, &sha256_hex(html.as_bytes()));
        nav_msg = Some(nav.out.clone());
    }

    // Freeze each table view to a deterministic *.view.json the renderer reads.
    let mut view_count = 0usize;
    for view in &views {
        let json = views::build_view_json(spec.project_name(), view);
        let view_path = root.join(&view.out);
        if let Some(parent) = view_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
        }
        std::fs::write(&view_path, &json)
            .map_err(|e| format!("could not write {}: {e}", view_path.display()))?;
        m.set(&view.out, &sha256_hex(json.as_bytes()));
        view_count += 1;
    }

    write_file(&out_abs.join(".manifest").to_string_lossy(), &m.to_text())?;

    println!("✓ built {tokens_path}");
    if ramp.is_some() {
        println!("  brand ramp: derived via rustio-admin rio-theme");
    }
    if let Some(out) = nav_msg {
        println!("  navigation: {out}  (serve via RUSTIO_TEMPLATE_DIR)");
    }
    if view_count > 0 {
        println!(
            "  views:      {view_count} table(s) → *.view.json  (read at runtime by the renderer)"
        );
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

    // Navigation layer lint (non-blocking warnings; structural errors counted).
    let nav_errors = if let Some(nav) = spec.navigation() {
        let models = read_project_models(&root);
        let lint = nav::lint(&nav, models.as_deref());
        for w in &lint.warnings {
            eprintln!("  warning: {w}");
        }
        lint.errors
    } else {
        Vec::new()
    };

    // View layer lint (non-blocking warnings; structural errors counted).
    let mut view_errors: Vec<String> = Vec::new();
    for view in spec.views() {
        let columns = resolve_view_columns(&root, &view);
        let lint = views::lint(&view, columns.as_deref());
        for w in &lint.warnings {
            eprintln!("  warning: {w}");
        }
        view_errors.extend(lint.errors);
    }

    let manifest_path = root.join(spec.out_dir()).join(".manifest");
    let mtext = std::fs::read_to_string(&manifest_path).map_err(|_| {
        format!(
            "no manifest at {} — run `rustio-design build` first",
            manifest_path.display()
        )
    })?;
    let m = Manifest::parse(&mtext);

    let mut problems = 0usize;

    for e in &report.errors {
        eprintln!("  error:   {e}");
        problems += 1;
    }
    for e in &nav_errors {
        eprintln!("  error:   {e}");
        problems += 1;
    }
    for e in &view_errors {
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
        let full = root.join(path);
        let full = full.to_string_lossy().to_string();
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
    let path = spec_root(spec_path).join(spec.out_dir()).join("wire.env");
    let text = std::fs::read_to_string(&path)
        .map_err(|_| format!("no {} — run `rustio-design build` first", path.display()))?;
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

/// Best-effort read of the project's registered models from `<root>/src/main.rs`,
/// for navigation coverage checks. `None` if the file can't be read.
fn read_project_models(root: &std::path::Path) -> Option<Vec<String>> {
    let src = std::fs::read_to_string(root.join("src/main.rs")).ok()?;
    Some(nav::registered_models(&src))
}

/// Best-effort resolution of a table view's columns from its model struct, for
/// schema-aware validation. `None` when no `model` is declared or it can't be
/// parsed (in which case `views::lint` warns that coverage is unverified).
fn resolve_view_columns(root: &std::path::Path, view: &spec::TableView) -> Option<Vec<String>> {
    let model = view.model.as_ref()?;
    let source = view
        .source
        .clone()
        .unwrap_or_else(|| "src/main.rs".to_string());
    let src = std::fs::read_to_string(root.join(&source)).ok()?;
    let cols = schema::model_columns(&src, model);
    if cols.is_empty() {
        None
    } else {
        Some(cols.into_iter().map(|c| c.name).collect())
    }
}

// ─────────────────────────────────────────────────────────────────────────
// schema  (extract a table's columns from its model — feeds the view editor)
// ─────────────────────────────────────────────────────────────────────────

/// Extract model struct fields into view-editor schema files.
///
/// One model:
///   `rustio-design schema --model Booking [--source src/models.rs]
///    [--table bookings] [--out path.json]`
///
/// Every registered model at once (zero per-table repetition):
///   `rustio-design schema --all [--source src/main.rs] --out-dir <dir>`
///
/// Best-effort: parses structs from source rather than compiling, so it stays
/// zero-dependency.
fn cmd_schema(args: &[String], spec_path: &str) -> Result<(), String> {
    let root = spec_root(spec_path);
    let source = flag_value(args, "--source").unwrap_or_else(|| "src/main.rs".to_string());
    let project = load_spec(spec_path)
        .map(|(_, s)| s.project_name().to_string())
        .unwrap_or_else(|_| "app".to_string());
    let src = std::fs::read_to_string(root.join(&source))
        .map_err(|_| format!("cannot read {source} — pass --source <path to the model(s)>"))?;

    if args.iter().any(|a| a == "--all") {
        return cmd_schema_all(args, &project, &source, &src);
    }

    let model = flag_value(args, "--model")
        .ok_or("`schema` needs --model <Type> (e.g. --model Booking), or --all for every model")?;
    let table = flag_value(args, "--table").unwrap_or_else(|| default_table(&model));
    let cols = schema::model_columns(&src, &model);
    if cols.is_empty() {
        return Err(format!(
            "no fields found for `struct {model}` in {source} — check --model / --source"
        ));
    }

    let json = schema::columns_json(&project, &table, &cols);
    if let Some(out) = flag_value(args, "--out") {
        write_file(&out, &json)?;
        println!("✓ wrote {out}  ({} columns)", cols.len());
        println!("  drop it into the editor's data/schemas/{project}/{table}.json");
    } else {
        print!("{json}");
    }
    Ok(())
}

/// Extract a schema for every `.model::<T>()` registered in the source — one
/// command for the whole app. Writes `<out-dir>/<table>.json` when `--out-dir`
/// is given, otherwise prints each schema. Structs not found in `--source` are
/// reported so they can be pointed at with a single-model run.
fn cmd_schema_all(args: &[String], project: &str, source: &str, src: &str) -> Result<(), String> {
    let models = nav::registered_models(src);
    if models.is_empty() {
        return Err(format!(
            "no `.model::<…>()` registrations found in {source} — pass --source <path>"
        ));
    }
    let out_dir = flag_value(args, "--out-dir");
    if let Some(dir) = &out_dir {
        std::fs::create_dir_all(dir).map_err(|e| format!("could not create {dir}: {e}"))?;
    }

    let mut written = 0usize;
    let mut missing: Vec<String> = Vec::new();
    for model in &models {
        let cols = schema::model_columns(src, model);
        if cols.is_empty() {
            missing.push(model.clone());
            continue;
        }
        let table = default_table(model);
        let json = schema::columns_json(project, &table, &cols);
        match &out_dir {
            Some(dir) => {
                let path = format!("{dir}/{table}.json");
                write_file(&path, &json)?;
                println!("✓ {table:<16} {model} → {path}  ({} columns)", cols.len());
                written += 1;
            }
            None => {
                println!("// ── {table} ({model}) ──");
                print!("{json}");
            }
        }
    }

    if !missing.is_empty() {
        println!();
        println!(
            "  note: {} model(s) not defined in {source}: {}",
            missing.len(),
            missing.join(", ")
        );
        println!("        run `schema --model <Name> --source <file>` for those.");
    }
    if out_dir.is_some() {
        println!();
        println!("  {written} schema(s) ready — copy into the editor's data/schemas/{project}/");
    }
    Ok(())
}

/// Default table identifier for a model: lowercase + a naive plural `s`.
/// Override with `--table`; it just needs to match the `[views.<table>]` key.
fn default_table(model: &str) -> String {
    format!("{}s", model.to_lowercase())
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
         \x20 schema    Extract model columns into editor schemas (--model | --all)\n\
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

# ── Navigation (WHAT → output) ───────────────────────────────────────
# Group the admin sidebar by domain. Each `Group = "Item, Item"` matches
# the model display names rustio-admin shows; order is preserved. Reserved
# keys: `_hidden` (models reached via their parent, kept out of the nav)
# and `_out` (the generated _sidebar.html path, served via RUSTIO_TEMPLATE_DIR).
# Reason in DESIGN_ARCHITECTURE.md / DESIGN_REASONING.md before changing this.
# [navigation]
# Catalogue = "Products, Categories"
# Customers = "Customers"
# Sales     = "Orders, Payments"
# _hidden   = "Order items, Cart items"
# _out      = "generated/templates/admin/_sidebar.html"

# ── Views (WHAT → output) ────────────────────────────────────────────
# Per-table record layout. Roles are comma-separated cells; compose with
# `+` and hint a style in parens: (stacked) | (inline) | (badge). `build`
# freezes each table to generated/views/<table>.view.json — the frozen
# file the runtime renderer reads. Set `model` (+ optional `source`) to
# validate field names against the table's real columns. Author this with
# the Adaptive View Editor; reason in DESIGN_ARCHITECTURE.md first.
# [views.bookings]
# model     = "Booking"
# source    = "src/main.rs"
# mode      = "list"
# primary   = "booked_at"
# secondary = "customer + phone (inline), status (badge), assigned_to"
# detail    = "address, notes"
# hidden    = "id, internal_uuid"

[custom_css]
# Escape hatch for the rare rule with no token. Validated: no @import, no
# remote url(), no markup. Prefer tokens above whenever possible.
# rules = """
# .rio-sidebar { letter-spacing: 0.01em; }
# """
"##;
