use super::super::SuiCli;
use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct DumpValidatorsOutput {
    pub(crate) address: String,
    pub(crate) peer_id: String,
}

pub(crate) fn run(cli: &SuiCli, genesis: &Path) -> anyhow::Result<Vec<DumpValidatorsOutput>> {
    let output = cli.run_binary_command(
        "sui-tool",
        &[
            "dump-validators",
            "--genesis",
            genesis.to_str().unwrap_or_default(),
        ],
        super::super::SUI_CONFIG_DIR,
    )?;
    let stdout = String::from_utf8(output.stdout)?;

    let p2p_re = Regex::new(r#"p2p_address: "([^"]+)""#)?;
    let pubkey_re = Regex::new(r"network_pubkey_bytes: \[([^\]]+)\]")?;

    let addresses: Vec<String> = p2p_re
        .captures_iter(&stdout)
        .map(|cap| cap[1].to_string())
        .collect();

    let peer_ids: Vec<String> = pubkey_re
        .captures_iter(&stdout)
        .map(|cap| {
            cap[1]
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| {
                    s.parse::<u8>()
                        .map_err(|e| anyhow::anyhow!("bad network_pubkey_bytes byte: {e}"))
                })
                .collect::<anyhow::Result<Vec<u8>>>()
                .map(|bytes| bytes.iter().map(|b| format!("{:02x}", b)).collect())
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    anyhow::ensure!(
        !addresses.is_empty() && addresses.len() == peer_ids.len(),
        "mismatched p2p_address ({}) and network_pubkey_bytes ({})",
        addresses.len(),
        peer_ids.len(),
    );

    Ok(addresses
        .into_iter()
        .zip(peer_ids)
        .map(|(address, peer_id)| DumpValidatorsOutput { address, peer_id })
        .collect())
}
