use anyhow::Context;

use super::SuiCli;
use std::{fs, path::Path};

#[derive(Debug, Clone)]
pub(crate) struct GenesisOutput {
    pub(crate) blob: Vec<u8>,
}

pub(crate) fn run_from_file(
    cli: &SuiCli,
    config_path: &Path,
    work_dir: &Path,
    force: bool,
) -> anyhow::Result<GenesisOutput> {
    let mut args = vec![
        "genesis",
        "--working-dir",
        work_dir.to_str().context("Invalid work_dir")?,
        "--from-config",
        config_path.to_str().context("Invalid config_path")?,
    ];
    if force {
        args.push("--force");
    }
    cli.run_binary_command("sui", &args, super::SUI_CONFIG_DIR)?;

    read_genesis_output(work_dir)
}

fn read_genesis_output(work_dir: &Path) -> anyhow::Result<GenesisOutput> {
    let blob = fs::read(work_dir.join("genesis.blob"))?;
    Ok(GenesisOutput { blob })
}
