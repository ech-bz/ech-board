use super::{KeyScheme, SuiCli, SuiKeytoolOutput};
use ech_board_common::SuiKeypair;
use std::fs;

pub(crate) struct SuiKeytool<'a> {
    cli: &'a SuiCli,
}

impl<'a> SuiKeytool<'a> {
    pub(crate) fn new(cli: &'a SuiCli) -> Self {
        Self { cli }
    }

    pub(crate) fn generate(&self, scheme: KeyScheme) -> anyhow::Result<SuiKeypair> {
        let generated: SuiKeytoolOutput = serde_json::from_slice(
            &self
                .cli
                .run_binary_command(
                    "sui",
                    &["keytool", "generate", scheme.as_str(), "--json"],
                    super::SUI_KEYS_DIR,
                )?
                .stdout,
        )?;
        let key_path = scheme.key_file_path(&generated.sui_address);
        let private_key = fs::read_to_string(key_path)?.trim().to_string();
        Ok(SuiKeypair {
            address: generated.sui_address,
            private_key,
        })
    }
}
