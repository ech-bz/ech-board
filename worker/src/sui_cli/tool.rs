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
}
