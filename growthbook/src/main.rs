use std::env;
use std::ffi::{OsStr, OsString};
use std::process::Command;

use anyhow::{bail, Context, Result};
use codex_growthbook::{
    build_codex_args, evaluate_codex_flags, load_growthbook_client, FEATURE_MAPPINGS,
};
use growthbook_rust::client::GrowthBookClientTrait;

#[tokio::main]
async fn main() {
    if let Err(error) = entry().await {
        eprintln!("codex-growthbook: {error:#}");
        std::process::exit(2);
    }
}

async fn entry() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let command = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: codex-growthbook <list|run> [-- <codex args...>]"))?;

    match command.to_str() {
        Some("list") => list_features().await,
        Some("run") => run_codex(args.collect()).await,
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some(other) => bail!("unknown command '{other}'; run 'codex-growthbook help'"),
        None => bail!("command name must be valid UTF-8"),
    }
}

async fn list_features() -> Result<()> {
    let client = load_growthbook_client().await?;
    let decisions = evaluate_codex_flags(&client)?;

    println!("GrowthBook -> Codex feature map");
    println!("{} remote flags fetched", client.total_features());
    for mapping in FEATURE_MAPPINGS {
        match decisions.iter().find(|decision| decision.codex_key == mapping.codex_key) {
            Some(decision) => println!(
                "{:<24} -> {:<20} {}",
                mapping.growthbook_key,
                mapping.codex_key,
                if decision.enabled { "ENABLED" } else { "DISABLED" }
            ),
            None => println!(
                "{:<24} -> {:<20} UNCONFIGURED (Codex default: {})",
                mapping.growthbook_key,
                mapping.codex_key,
                if mapping.codex_default { "enabled" } else { "disabled" }
            ),
        }
    }
    Ok(())
}

async fn run_codex(mut codex_args: Vec<OsString>) -> Result<()> {
    if codex_args
        .first()
        .is_some_and(|argument| argument.as_os_str() == OsStr::new("--"))
    {
        codex_args.remove(0);
    }

    let client = load_growthbook_client().await?;
    let decisions = evaluate_codex_flags(&client)?;
    let effective_args = build_codex_args(&codex_args, &decisions);

    let codex_bin = env::var_os("CODEX_BIN").unwrap_or_else(|| OsString::from("codex"));
    let mut command = Command::new(&codex_bin);
    command
        .args(&effective_args)
        .env_remove("GB_SDK_KEY")
        .env_remove("GB_URL")
        .env_remove("GB_DECRYPTION_KEY")
        .env_remove("GB_ATTRIBUTES_JSON");

    let status = command.status().with_context(|| {
        format!(
            "could not start Codex executable '{}'; set CODEX_BIN if it is not on PATH",
            codex_bin.to_string_lossy()
        )
    })?;

    std::process::exit(status.code().unwrap_or(1));
}

fn print_help() {
    println!(
        "codex-growthbook\n\n\
         Commands:\n\
           list                 Fetch GrowthBook and show mapped Codex feature decisions\n\
           run [-- ARGS...]     Fetch GrowthBook, then launch Codex with approved overrides\n\
           help                 Show this help\n\n\
         Environment:\n\
           GB_SDK_KEY            Required GrowthBook SDK client key\n\
           GB_URL                Optional API base URL (default: https://cdn.growthbook.io)\n\
           GB_DECRYPTION_KEY     Optional key for encrypted GrowthBook feature payloads\n\
           GB_ATTRIBUTES_JSON   Optional JSON object used for GrowthBook targeting\n\
           CODEX_BIN             Optional Codex executable path (default: codex)\n\n\
         Remote decisions apply only to the public stable features listed in README.md.\n\
         GrowthBook credentials and targeting attributes are not passed to the Codex child process."
    );
}
