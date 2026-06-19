pub(crate) mod client;
mod genesis;
mod keytool;
pub(crate) mod move_;
pub mod tool;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

pub(crate) use genesis::GenesisOutput;

pub(crate) const SUI_CONFIG_DIR: &str = "/tmp/sui";
pub(crate) const SUI_KEYS_DIR: &str = "/tmp/sui/keys";
pub(crate) const SUI_MOVE_HOME: &str = "/tmp/sui/move";
pub(crate) const SUI_KEYSTORE_PATH: &str = "/tmp/sui/sui.keystore";

#[derive(Debug, serde::Serialize)]
pub(crate) struct SuiClientConfig {
    pub(crate) keystore: SuiKeystoreConfig,
    pub(crate) envs: Vec<SuiEnvConfig>,
    pub(crate) active_env: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct SuiEnvConfig {
    pub(crate) alias: String,
    pub(crate) rpc: String,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct SuiKeystoreConfig {
    #[serde(rename = "File")]
    pub(crate) file: String,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum KeyScheme {
    Ed25519,
    Bls12381,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SuiKeytoolOutput {
    pub(crate) sui_address: String,
}

impl KeyScheme {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ed25519 => "ed25519",
            Self::Bls12381 => "bls12381",
        }
    }

    pub(crate) fn key_file_path(self, addr: &str) -> PathBuf {
        match self {
            Self::Bls12381 => PathBuf::from(SUI_KEYS_DIR).join(format!("bls-{addr}.key")),
            Self::Ed25519 => PathBuf::from(SUI_KEYS_DIR).join(format!("{addr}.key")),
        }
    }
}

impl std::fmt::Display for KeyScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub(crate) struct SuiCli;

impl SuiCli {
    pub(crate) fn new(rpc_url: &str) -> anyhow::Result<Self> {
        if PathBuf::from(SUI_CONFIG_DIR).exists() {
            fs::remove_dir_all(SUI_CONFIG_DIR)?;
        }
        if PathBuf::from(SUI_KEYS_DIR).exists() {
            fs::remove_dir_all(SUI_KEYS_DIR)?;
        }
        fs::create_dir_all(SUI_CONFIG_DIR)?;
        fs::create_dir_all(SUI_KEYS_DIR)?;
        fs::create_dir_all(SUI_MOVE_HOME)?;
        let client_config = SuiClientConfig {
            keystore: SuiKeystoreConfig {
                file: SUI_KEYSTORE_PATH.into(),
            },
            envs: vec![SuiEnvConfig {
                alias: "localnet".into(),
                rpc: rpc_url.to_string(),
            }],
            active_env: Some("localnet".into()),
        };
        fs::write(
            PathBuf::from(SUI_CONFIG_DIR).join("client.yaml"),
            serde_saphyr::to_string(&client_config)?,
        )?;
        Ok(Self)
    }

    pub(crate) fn keytool(&self) -> keytool::SuiKeytool<'_> {
        keytool::SuiKeytool::new(self)
    }

    pub(crate) fn tool(&self) -> tool::SuiTool<'_> {
        tool::SuiTool::new(self)
    }

    pub(crate) fn client(&self) -> client::SuiClient<'_> {
        client::SuiClient::new(self)
    }

    pub(crate) fn genesis_from_file(
        &self,
        config_path: impl AsRef<Path>,
        work_dir: impl AsRef<Path>,
        force: bool,
    ) -> anyhow::Result<GenesisOutput> {
        genesis::run_from_file(self, config_path.as_ref(), work_dir.as_ref(), force)
    }

    pub(crate) fn move_(&self) -> move_::SuiMove<'_> {
        move_::SuiMove::new(self)
    }

    fn run_binary_command(
        &self,
        binary: &str,
        args: &[&str],
        current_dir: impl AsRef<std::path::Path>,
    ) -> anyhow::Result<std::process::Output> {
        let output = ProcessCommand::new(binary)
            .args(args)
            .current_dir(current_dir)
            .env("SUI_CONFIG_DIR", SUI_CONFIG_DIR)
            .env("SUI_KEYS_DIR", SUI_KEYS_DIR)
            .env("MOVE_HOME", SUI_MOVE_HOME)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "{binary} {} failed:\nstdout: {}\nstderr: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(output)
    }
}
