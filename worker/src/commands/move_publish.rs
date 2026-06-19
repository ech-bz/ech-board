use crate::Ctx;
use crate::config::load_job_config;
use crate::sui_cli::SuiCli;
use crate::sui_cli::client::{ObjectChange, SuiMoveType};
use anyhow::Context;
use ech_board_common::MovePublishConfig;
use ech_board_common::keys::MOVE_ORIGINAL_ID;
use ech_board_common::keys::MOVE_REF;
use ech_k8s::StoreExt;
use k8s_openapi::api::core::v1::ConfigMap;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub(crate) async fn run(ctx: &Ctx, config_path: &Path) -> anyhow::Result<()> {
    let config: MovePublishConfig = load_job_config(config_path)?;
    let sui = SuiCli::new(&config.rpc_url)?;
    let work_dir = tempfile::tempdir()?;

    let repo_dir = work_dir.path().join("repo");
    let status = Command::new("git")
        .arg("init")
        .arg(&repo_dir)
        .status()
        .context("git init failed")?;
    anyhow::ensure!(status.success(), "git init exited with {status}");

    let status = Command::new("git")
        .args(["remote", "add", "origin", &config.repo])
        .current_dir(&repo_dir)
        .status()
        .context("git remote add failed")?;
    anyhow::ensure!(status.success(), "git remote add exited with {status}");

    let status = Command::new("git")
        .args([
            "fetch",
            "--depth=1",
            "--filter=blob:none",
            "origin",
            &config.git_ref,
        ])
        .current_dir(&repo_dir)
        .status()
        .context("git fetch failed")?;
    anyhow::ensure!(status.success(), "git fetch exited with {status}");

    let status = Command::new("git")
        .args(["sparse-checkout", "set", &config.package_path])
        .current_dir(&repo_dir)
        .status()
        .context("git sparse-checkout failed")?;
    anyhow::ensure!(status.success(), "git sparse-checkout exited with {status}");

    let status = Command::new("git")
        .args(["checkout", "FETCH_HEAD"])
        .current_dir(&repo_dir)
        .status()
        .context("git checkout failed")?;
    anyhow::ensure!(status.success(), "git checkout exited with {status}");

    let package_dir = repo_dir.join(&config.package_path);

    let chain_id = sui.client().chain_identifier()?;
    tracing::info!(chain_id, "resolved chain identifier");
    std::fs::write(
        package_dir.join("Move.toml"),
        format!(
            "{}\n[environments]\nlocalnet = \"{chain_id}\"\n",
            std::fs::read_to_string(package_dir.join("Move.toml"))
                .context("failed to read Move.toml")?
        ),
    )
    .context("failed to append environments to Move.toml")?;

    sui.move_()
        .build(&package_dir)
        .context("sui move build failed")?;

    std::fs::write(
        crate::sui_cli::SUI_KEYSTORE_PATH,
        format!("[\"{}\"]", config.publisher_key_base64),
    )
    .context("failed to write keystore")?;

    let publisher_address = sui
        .client()
        .active_address()
        .context("sui client active address failed")?;

    let package = sui
        .client()
        .objects(&publisher_address)
        .context("sui client objects failed")?
        .iter()
        .find_map(|obj| {
            let move_ = &obj.data.move_;
            if !matches!(
                &move_.type_,
                SuiMoveType::Struct { other } if other.module == "package" && other.name == "UpgradeCap"
            ) {
                return None;
            }
            if move_.contents.len() < 64 {
                return None;
            }
            let cap_id = format!("0x{}", bytes_to_hex(&move_.contents[..32]));
            let pkg_id = format!("0x{}", bytes_to_hex(&move_.contents[32..64]));
            Some((cap_id, pkg_id))
        });

    let original_id = match &package {
        Some((upgrade_cap_id, forum_package_id)) => {
            let version = sui
                .client()
                .upgrade_cap_content_version(upgrade_cap_id)
                .context("failed to get upgrade cap version")?;
            let original_id = sui
                .client()
                .package_original_id(forum_package_id)
                .context("failed to get original package id")?;

            let published_toml = format!(
                concat!(
                    "[published.localnet]\n",
                    "build-config = {{ flavor = \"sui\", edition = \"2024\" }}\n",
                    "toolchain-version = \"1.72.5\"\n",
                    "chain-id = \"{chain_id}\"\n",
                    "published-at = \"{forum_package_id}\"\n",
                    "original-id = \"{original_id}\"\n",
                    "upgrade-capability = \"{upgrade_cap_id}\"\n",
                    "version = {version}\n",
                ),
                chain_id = chain_id,
                forum_package_id = forum_package_id,
                original_id = original_id,
                upgrade_cap_id = upgrade_cap_id,
                version = version,
            );
            std::fs::write(package_dir.join("Published.toml"), published_toml)
                .context("failed to write Published.toml")?;

            sui.client()
                .upgrade(&package_dir)
                .context("sui client upgrade failed")?;

            tracing::info!(
                network = %config.worker.network_name,
                forum_package_id,
                upgrade_cap_id,
                git_ref = %config.git_ref,
                "move package upgraded"
            );

            original_id
        }
        None => {
            let forum_package_id = sui
                .client()
                .publish(&package_dir)
                .context("sui client publish failed")?
                .object_changes
                .iter()
                .find_map(|change| match change {
                    ObjectChange::Published { package_id, .. } => Some(package_id.as_str()),
                    _ => None,
                })
                .context("published package not found in publish output")?
                .to_string();

            tracing::info!(
                network = %config.worker.network_name,
                forum_package_id,
                "move package published"
            );

            forum_package_id
        }
    };

    ctx.k8s
        .namespaced::<ConfigMap>(&config.worker.namespace)
        .store_put(
            &config.config_map_name,
            BTreeMap::from([
                (MOVE_REF.to_string(), config.git_ref.clone()),
                (MOVE_ORIGINAL_ID.to_string(), original_id),
            ]),
            None,
        )
        .await?;

    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}
