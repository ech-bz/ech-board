use super::super::SuiCli;
use std::path::Path;

pub(crate) fn run(
    cli: &SuiCli,
    config_path: &Path,
    db_path: &Path,
) -> anyhow::Result<()> {
    cli.run_binary_command(
        "sui-tool",
        &[
            "download-db-snapshot",
            "--config-path",
            config_path.to_str().unwrap_or_default(),
            "--db-path",
            db_path.to_str().unwrap_or_default(),
            "--skip-indexes",
        ],
        super::super::SUI_CONFIG_DIR,
    )?;
    Ok(())
}
