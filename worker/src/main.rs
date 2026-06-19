mod commands;
mod config;
mod sui_cli;

use clap::Parser;
use std::path::PathBuf;
use sui_cli::SuiCli;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "ech-board-worker")]
struct Cli {
    #[arg(short, long)]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    #[command(name = "download-db-snapshot")]
    DownloadDbSnapshot,
    #[command(name = "download-genesis")]
    DownloadGenesis,
    #[command(name = "genesis")]
    Genesis,
    #[command(name = "keys")]
    Keys,
    #[command(name = "seed-peers")]
    SeedPeers,
    #[command(name = "move-publish")]
    MovePublish,
}

pub(crate) struct Ctx {
    pub(crate) k8s: ech_k8s::K8sClient,
    pub(crate) sui: SuiCli,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install rustls crypto provider"))?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let cli = Cli::parse();
    let ctx = Ctx {
        k8s: ech_k8s::K8sClient::try_new("ech-board-worker").await?,
        sui: SuiCli::new("http://127.0.0.1:9000")?,
    };
    match cli.command {
        Command::DownloadDbSnapshot => commands::download_db_snapshot::run(&ctx, &cli.config).await,
        Command::DownloadGenesis => commands::download_genesis::run(&ctx, &cli.config).await,
        Command::Genesis => commands::genesis::run(&ctx, &cli.config).await,
        Command::Keys => commands::keys::run(&ctx, &cli.config).await,
        Command::SeedPeers => commands::seed_peers::run(&ctx, &cli.config).await,
        Command::MovePublish => commands::move_publish::run(&ctx, &cli.config).await,
    }
}
