//! OpenSplit Tauri backend library entry point.

use std::sync::Arc;

use clap::Parser;
use tracing_subscriber::EnvFilter;

mod config;
mod detect;
mod ipc;
mod pty;
mod session;

use ipc::{AppState, CliOverride};

/// CLI arguments understood by the `opensplit` binary.
#[derive(Debug, Parser)]
#[command(
    name = "opensplit",
    version,
    about = "Cross-platform terminal harness with right-click splits."
)]
struct Cli {
    /// Profile name (or bare command) to launch in the initial pane.
    /// When omitted and no `default_profile` is set, OpenSplit shows the
    /// launcher picker.
    profile: Option<String>,

    /// Run a raw command in the initial pane. Everything after `--` is the
    /// command + its args.
    #[arg(last = true)]
    raw: Vec<String>,
}

pub fn run() {
    let filter = EnvFilter::try_from_env("OPENSPLIT_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("info,opensplit=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();

    let cli = Cli::parse();

    let config = match config::Config::load_or_create() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("failed to load config, using defaults: {e:#}");
            config::Config::defaults()
        }
    };

    // Translate CLI into an optional override that wins over the config's
    // default_profile and over the picker.
    let cli_override = if !cli.raw.is_empty() {
        let raw = cli.raw;
        Some(CliOverride::Raw(config::LaunchSpec {
            command: raw[0].clone(),
            args: raw[1..].to_vec(),
            cwd: None,
            env: Default::default(),
            profile: None,
        }))
    } else {
        cli.profile.map(CliOverride::Profile)
    };

    tracing::info!(
        ?cli_override,
        default_profile = ?config.default_profile,
        "startup state"
    );

    let state = Arc::new(AppState::new(config, cli_override));

    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            ipc::get_version,
            ipc::get_startup_action,
            ipc::get_shell_spec,
            ipc::detect_tools,
            ipc::list_profiles,
            ipc::get_config,
            ipc::set_default_profile,
            ipc::set_ssh_inherit,
            ipc::spawn_pane,
            ipc::write_pane,
            ipc::resize_pane,
            ipc::close_pane,
            ipc::pane_foreground_info,
            ipc::resolve_split_spec,
        ])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running OpenSplit");
}
