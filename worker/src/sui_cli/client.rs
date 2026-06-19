use super::SuiCli;
use anyhow::Context;
use serde::Deserialize;
use std::path::Path;

pub(crate) struct SuiClient<'a> {
    cli: &'a SuiCli,
}

impl<'a> SuiClient<'a> {
    pub(crate) fn new(cli: &'a SuiCli) -> Self {
        Self { cli }
    }

    pub(crate) fn chain_identifier(&self) -> anyhow::Result<String> {
        let output = self
            .cli
            .run_binary_command("sui", &["client", "chain-identifier"], "/tmp")?;
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    pub(crate) fn publish(&self, package_dir: &Path) -> anyhow::Result<PublishOutput> {
        let output =
            self.cli
                .run_binary_command("sui", &["client", "publish", "--json"], package_dir)?;
        serde_json::from_slice(&output.stdout).context("failed to parse publish output")
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) struct PublishOutput {
    pub(crate) object_changes: Vec<ObjectChange>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) enum ObjectChange {
    #[serde(rename_all = "camelCase")]
    Published {
        package_id: String,
        version: String,
        digest: String,
        modules: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    Created {
        object_type: String,
        object_id: String,
        version: String,
        digest: String,
    },
    #[serde(rename_all = "camelCase")]
    Mutated {
        object_type: String,
        object_id: String,
        version: String,
        previous_version: String,
        digest: String,
    },
}
