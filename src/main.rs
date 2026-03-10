mod agent;
mod cmd;
mod meta;
mod prompt;
mod tmux;
mod web;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

#[derive(Parser)]
#[command(about = "CrewMux", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new AI team session
    Team {
        #[command(subcommand)]
        action: cmd::team::TeamAction,
    },
    /// Spawn workers and dispatch tasks
    Task {
        #[command(subcommand)]
        action: cmd::task::TaskAction,
    },
    /// Control and monitor agents
    Ctl {
        #[command(subcommand)]
        action: cmd::ctl::CtlAction,
    },
    /// Start the web UI
    Web {
        /// Port to listen on
        #[arg(short, long, default_value = "7700")]
        port: u16,
    },
    /// Install/uninstall as a background service
    Install,
    /// Uninstall the background service
    Uninstall,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = parse_cli()?;

    match cli.command {
        Commands::Team { action } => cmd::team::run(action)?,
        Commands::Task { action } => cmd::task::run(action)?,
        Commands::Ctl { action } => cmd::ctl::run(action)?,
        Commands::Web { port } => web::serve(port).await?,
        Commands::Install => cmd::service::install()?,
        Commands::Uninstall => cmd::service::uninstall()?,
    }

    Ok(())
}

fn parse_cli() -> anyhow::Result<Cli> {
    let mut command = Cli::command();

    if let Some(bin_name) = current_bin_name() {
        let leaked_name: &'static str = Box::leak(bin_name.into_boxed_str());
        command = command.name(leaked_name);
    }

    let matches = command.get_matches();
    Ok(Cli::from_arg_matches(&matches)?)
}

fn current_bin_name() -> Option<String> {
    let arg0 = std::env::args_os().next()?;
    let name = std::path::Path::new(&arg0).file_stem()?;
    Some(name.to_string_lossy().into_owned())
}
