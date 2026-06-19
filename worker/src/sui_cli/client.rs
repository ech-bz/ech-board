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

    pub(crate) fn active_address(&self) -> anyhow::Result<String> {
        let output =
            self.cli
                .run_binary_command("sui", &["client", "addresses", "--json"], "/tmp")?;
        let parsed: AddressesOutput =
            serde_json::from_slice(&output.stdout).context("failed to parse addresses output")?;
        Ok(parsed.active_address)
    }

    pub(crate) fn publish(&self, package_dir: &Path) -> anyhow::Result<PublishOutput> {
        let output =
            self.cli
                .run_binary_command("sui", &["client", "publish", "--json"], package_dir)?;
        serde_json::from_slice(&output.stdout).context("failed to parse publish output")
    }

    pub(crate) fn upgrade(&self, package_dir: &Path) -> anyhow::Result<()> {
        self.cli
            .run_binary_command("sui", &["client", "upgrade"], package_dir)
            .map(|_| ())
    }

    pub(crate) fn objects(&self, address: &str) -> anyhow::Result<Vec<SuiObject>> {
        let tmp = tempfile::tempdir()?;
        let output = self.cli.run_binary_command(
            "sui",
            &["client", "objects", address, "--json"],
            tmp.path(),
        )?;
        let stdout = String::from_utf8(output.stdout)?;
        serde_json::from_str(&stdout).context("failed to parse sui client objects output")
    }

    pub(crate) fn upgrade_cap_content_version(
        &self,
        upgrade_cap_id: &str,
    ) -> anyhow::Result<String> {
        let output = self.cli.run_binary_command(
            "sui",
            &["client", "object", upgrade_cap_id, "--json"],
            "/tmp",
        )?;
        let parsed: UpgradeCapObjectOutput = serde_json::from_slice(&output.stdout)
            .context("failed to parse upgrade cap object output")?;
        Ok(parsed.content.version)
    }

    pub(crate) fn package_original_id(&self, pkg_id: &str) -> anyhow::Result<String> {
        let output =
            self.cli
                .run_binary_command("sui", &["client", "object", pkg_id, "--json"], "/tmp")?;
        let parsed: PackageObjectOutput = serde_json::from_slice(&output.stdout)
            .context("failed to parse package object output")?;
        let first = parsed
            .content
            .package
            .type_origin_table
            .first()
            .context("type_origin_table is empty")?;
        Ok(first.package.clone())
    }
}

#[derive(Debug, Deserialize)]
struct AddressesOutput {
    #[serde(rename = "activeAddress")]
    active_address: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PublishOutput {
    pub(crate) object_changes: Vec<ObjectChange>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) enum ObjectChange {
    #[serde(rename = "published")]
    Published {
        #[serde(rename = "packageId")]
        package_id: String,
        version: String,
        digest: String,
        modules: Vec<String>,
    },
    #[serde(rename = "created")]
    Created {
        #[serde(rename = "objectType")]
        object_type: String,
        #[serde(rename = "objectId")]
        object_id: String,
        version: String,
        digest: String,
    },
    #[serde(rename = "mutated")]
    Mutated {
        #[serde(rename = "objectType")]
        object_type: String,
        #[serde(rename = "objectId")]
        object_id: String,
        version: String,
        #[serde(rename = "previousVersion")]
        previous_version: String,
        digest: String,
    },
}

#[derive(Debug, Deserialize)]
pub(crate) struct SuiObject {
    pub(crate) data: SuiObjectData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SuiObjectData {
    #[serde(rename = "Move")]
    pub(crate) move_: SuiMoveObject,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SuiMoveObject {
    pub(crate) type_: SuiMoveType,
    pub(crate) contents: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum SuiMoveType {
    #[allow(dead_code)]
    Str(String),
    Struct {
        #[serde(rename = "Other")]
        other: SuiMoveStructType,
    },
}

#[derive(Debug, Deserialize)]
pub(crate) struct SuiMoveStructType {
    pub(crate) module: String,
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
struct UpgradeCapObjectOutput {
    content: UpgradeCapContent,
}

#[derive(Debug, Deserialize)]
struct UpgradeCapContent {
    version: String,
}

#[derive(Debug, Deserialize)]
struct PackageObjectOutput {
    content: PackageContentWrapper,
}

#[derive(Debug, Deserialize)]
struct PackageContentWrapper {
    #[serde(rename = "Package")]
    package: PackageFields,
}

#[derive(Debug, Deserialize)]
struct PackageFields {
    type_origin_table: Vec<TypeOrigin>,
}

#[derive(Debug, Deserialize)]
struct TypeOrigin {
    package: String,
}
