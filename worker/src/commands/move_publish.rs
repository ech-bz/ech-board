use crate::Ctx;
use crate::config::load_job_config;
use crate::sui_cli::SuiCli;
use crate::sui_cli::client::ObjectChange;
use anyhow::Context;
use ech_board_common::MovePublishConfig;
use ech_board_common::keys::MOVE_ORIGINAL_ID;
use ech_k8s::StoreExt;
use k8s_openapi::api::core::v1::Secret;
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

    let original_id = if let Some(original_id) = config.original_id {
        let (version, package_id, upgrade_cap_id) =
            query_package_info(&config.graphql_url, &original_id).await?;

        let published_toml = format!(
            concat!(
                "[published.localnet]\n",
                "build-config = {{ flavor = \"sui\", edition = \"2024\" }}\n",
                "toolchain-version = \"1.72.5\"\n",
                "chain-id = \"{chain_id}\"\n",
                "published-at = \"{package_id}\"\n",
                "original-id = \"{original_id}\"\n",
                "upgrade-capability = \"{upgrade_cap_id}\"\n",
                "version = {version}\n",
            ),
            chain_id = chain_id,
            package_id = package_id,
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
            original_id,
            upgrade_cap_id,
            git_ref = %config.git_ref,
            "move package upgraded"
        );

        original_id.clone()
    } else {
        let output = sui
            .client()
            .publish(&package_dir)
            .context("sui client publish failed")?;

        let package_id = output
            .object_changes
            .iter()
            .find_map(|c| match c {
                ObjectChange::Published { package_id, .. } => Some(package_id.clone()),
                _ => None,
            })
            .context("published package not found in publish output")?;

        tracing::info!(
            network = %config.worker.network_name,
            package_id,
            git_ref = %config.git_ref,
            "move package published"
        );

        package_id
    };

    ctx.k8s
        .namespaced::<Secret>(&config.worker.namespace)
        .store_put(
            &config.output_name,
            BTreeMap::new(),
            BTreeMap::from([(MOVE_ORIGINAL_ID.to_string(), original_id)]),
        )
        .await?;

    Ok(())
}

async fn query_package_info(
    graphql_url: &str,
    original_id: &str,
) -> anyhow::Result<(u64, String, String)> {
    let query = format!(
        r#"{{"query":"{{package(address: \"{original_id}\") {{version address previousTransaction {{effects {{objectChanges {{nodes {{address outputState {{asMoveObject {{contents {{type {{signature}}}}}}}}}}}}}}}}}}}}"}}"#
    );

    let client = reqwest::Client::new();
    let response: serde_json::Value = client
        .post(graphql_url)
        .header("Content-Type", "application/json")
        .body(query)
        .send()
        .await
        .context("graphql request failed")?
        .json()
        .await
        .context("failed to parse graphql response")?;

    let package = response
        .pointer("/data/package")
        .context("graphql response missing data.package")?;

    let version = package["version"]
        .as_u64()
        .context("graphql response missing package.version")?;

    let package_id = package["address"]
        .as_str()
        .context("graphql response missing package.address")?
        .to_string();

    let nodes = package
        .pointer("/previousTransaction/effects/objectChanges/nodes")
        .and_then(|n| n.as_array())
        .context("graphql response missing objectChanges nodes")?;

    let upgrade_cap = nodes
        .iter()
        .find_map(|node| {
            let sig = node.pointer("/outputState/asMoveObject/contents/type/signature")?;
            if sig.pointer("/datatype/type").and_then(|t| t.as_str()) == Some("UpgradeCap") {
                node["address"].as_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .context("upgrade cap not found in graphql object changes")?;

    Ok((version, package_id, upgrade_cap))
}
