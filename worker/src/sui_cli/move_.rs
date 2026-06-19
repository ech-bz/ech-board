use super::SuiCli;
use anyhow::Context;
use std::path::Path;

pub(crate) struct SuiMove<'a> {
    cli: &'a SuiCli,
}

impl<'a> SuiMove<'a> {
    pub(crate) fn new(cli: &'a SuiCli) -> Self {
        Self { cli }
    }

    pub(crate) fn build(&self, package_dir: &Path) -> anyhow::Result<()> {
        let path = package_dir
            .to_str()
            .context("package path is not valid UTF-8")?;
        self.cli
            .run_binary_command(
                "sui",
                &["move", "build", "--path", path, "-e", "localnet"],
                package_dir,
            )
            .map(|_| ())
    }
}
