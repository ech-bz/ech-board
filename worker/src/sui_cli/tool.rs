mod download_db_snapshot;
mod dump_validators;

use super::SuiCli;
use std::path::Path;

pub(crate) use dump_validators::DumpValidatorsOutput;

pub(crate) struct SuiTool<'a> {
    cli: &'a SuiCli,
}

impl<'a> SuiTool<'a> {
    pub(crate) fn new(cli: &'a SuiCli) -> Self {
        Self { cli }
    }

    pub(crate) fn dump_validators(
        &self,
        genesis: impl AsRef<Path>,
    ) -> anyhow::Result<Vec<DumpValidatorsOutput>> {
        dump_validators::run(self.cli, genesis.as_ref())
    }

    pub(crate) fn download_db_snapshot(
        &self,
        config_path: impl AsRef<Path>,
        db_path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        download_db_snapshot::run(self.cli, config_path.as_ref(), db_path.as_ref())
    }
}
