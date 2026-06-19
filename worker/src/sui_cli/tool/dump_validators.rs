use super::super::SuiCli;
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
            "--concise",
        ],
        super::super::SUI_CONFIG_DIR,
    )?;
    let stdout = String::from_utf8(output.stdout)?;
    let validators = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(parse_validator)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if validators.is_empty() {
        return Err(anyhow::anyhow!(
            "sui-tool dump-validators returned no validators"
        ));
    }
    Ok(validators)
}

fn parse_validator(line: &str) -> anyhow::Result<DumpValidatorsOutput> {
    let mut parts = line.split_whitespace();
    let _ordinal = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing ordinal in dump-validators line: {line}"))?;
    let _name = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing validator name in dump-validators line: {line}"))?;
    let _key = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing validator key in dump-validators line: {line}"))?;
    let address = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing address in dump-validators line: {line}"))?;
    let peer_id = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing peer-id in dump-validators line: {line}"))?;
    if parts.next().is_some() {
        return Err(anyhow::anyhow!(
            "unexpected dump-validators line format: {line}"
        ));
    }

    let address = address
        .strip_prefix("Multiaddr(")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(address)
        .to_string();

    Ok(DumpValidatorsOutput {
        address,
        peer_id: peer_id.to_string(),
    })
}
