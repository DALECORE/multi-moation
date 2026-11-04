use std::env;
use std::ffi::{OsStr, OsString};
use std::process::Command;

use anyhow::{bail, Context, Result};
use codex_growthbook::{build_codex_args, evaluate_codex_flags, load_export, FEATURE_MAPPINGS};

fn main() {
    if let Err(error) = entry() {
        eprintln!("codex-growthbook: {error:#}");
        std::process::exit(2);
    }
}

fn entry() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let command = args.next()
        .ok_or_else(|| anyhow::anyhow!("usage: codex-growthbook <list|run> [-- <codex args...>]"))?;

    match command.to_str() {
        Some("list") => list_features(),
        Some("run") => run_codex(args.collect()),
        Some("help") | Some("--help") | Some("-h") => { print_help(); Ok(()) }
        Some(other) => bail!("unknown command '{other}'; run 'codex-growthbook help'"),
        None => bail!("command name must be valid UTF-8"),
    }
}

fn load_local_features() -> Result<std::collections::BTreeMap<String, serde_json::Value>> {
    let path = env::var_os("GROWTHBOOK_FEATURES_FILE")
        .unwrap_or_else(|| OsString::from("growthbook/all-features.json"));
    load_export(path)
}

fn list_features() -> Result<()> {
    let features = load_local_features()?;
    let decisions = evaluate_codex_flags(&features)?;
    println!("Local GrowthBook export -> Codex feature map");
    println!("{} exported keys loaded; no network access", features.len());
    for mapping in FEATURE_MAPPINGS {
        match decisions.iter().find(|decision| decision.codex_key == mapping.codex_key) {
            Some(decision) => println!(
                "{:<24} -> {:<20} {}",
                mapping.growthbook_key, mapping.codex_key,
                if decision.enabled { "ENABLED" } else { "DISABLED" }
            ),
            None => println!(
                "{:<24} -> {:<20} UNCONFIGURED (Codex default: {})",
                mapping.growthbook_key, mapping.codex_key,
                if mapping.codex_default { "enabled" } else { "disabled" }
            ),
        }
    }
    Ok(())
}

fn run_codex(mut codex_args: Vec<OsString>) -> Result<()> {
    if codex_args.first().is_some_and(|arg| arg.as_os_str() == OsStr::new("--")) {
        codex_args.remove(0);
    }

    let features = load_local_features()?;
    let decisions = evaluate_codex_flags(&features)?;
    let effective_args = build_codex_args(&codex_args, &decisions);
    let codex_bin = env::var_os("CODEX_BIN").unwrap_or_else(|| OsString::from("codex"));

    let status = Command::new(&codex_bin)
        .args(&effective_args)
        .env_remove("GROWTHBOOK_FEATURES_FILE")
        .status()
        .with_context(|| format!(
            "could not start Codex executable '{}'; set CODEX_BIN if it is not on PATH",
            codex_bin.to_string_lossy()
        ))?;

    std::process::exit(status.code().unwrap_or(1));
}

fn print_help() {
    println!(
        "codex-growthbook\n\n\
         Commands:\n\
           list                 Read local export and show mapped Codex decisions\n\
           run [-- ARGS...]     Read local export, then launch Codex\n\
           help                 Show this help\n\n\
         Environment:\n\
           GROWTHBOOK_FEATURES_FILE  JSON export path (default: growthbook/all-features.json)\n\
           CODEX_BIN                 Optional Codex executable path (default: codex)\n\n\
         No GrowthBook SDK key, API URL, HTTP request, or background refresh is used."
    );
}
