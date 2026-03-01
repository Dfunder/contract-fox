use std::fmt;
use std::process::Command;

/// Configuration for creating and funding a Stellar custom-asset setup.
#[derive(Debug, Clone)]
pub struct TokenSetupConfig {
    pub network: String,
    pub asset_code: String,
    pub initial_supply: i64,
    pub issuing_key_name: String,
    pub distribution_key_name: String,
}

impl Default for TokenSetupConfig {
    fn default() -> Self {
        Self {
            network: "testnet".to_string(),
            asset_code: "AID".to_string(),
            initial_supply: 1_000_000,
            issuing_key_name: "aid_issuing".to_string(),
            distribution_key_name: "aid_distribution".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenSetupResult {
    pub issuing_public_key: String,
    pub distribution_public_key: String,
    pub asset: String,
    pub amount_issued: i64,
}

#[derive(Debug)]
pub enum TokenSetupError {
    CommandFailed {
        command: String,
        stderr: String,
    },
    InvalidCommandOutput {
        context: &'static str,
        output: String,
    },
}

impl fmt::Display for TokenSetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandFailed { command, stderr } => {
                write!(f, "command failed: {command}; stderr: {stderr}")
            }
            Self::InvalidCommandOutput { context, output } => {
                write!(f, "invalid command output while {context}: {output}")
            }
        }
    }
}

impl std::error::Error for TokenSetupError {}

/// Full setup flow:
/// 1. Generates issuing and distribution keypairs.
/// 2. Creates a trustline from distribution -> issuing asset.
/// 3. Issues fixed supply from issuing -> distribution.
pub fn setup_custom_asset(config: &TokenSetupConfig) -> Result<TokenSetupResult, TokenSetupError> {
    generate_keypair(&config.issuing_key_name, &config.network)?;
    generate_keypair(&config.distribution_key_name, &config.network)?;

    let issuing_public_key = get_public_key(&config.issuing_key_name)?;
    let distribution_public_key = get_public_key(&config.distribution_key_name)?;

    let asset = format!("{}:{}", config.asset_code, issuing_public_key);

    create_trustline(
        &config.network,
        &config.distribution_key_name,
        &config.asset_code,
        &issuing_public_key,
    )?;

    issue_asset(
        &config.network,
        &config.issuing_key_name,
        &distribution_public_key,
        &config.asset_code,
        config.initial_supply,
    )?;

    Ok(TokenSetupResult {
        issuing_public_key,
        distribution_public_key,
        asset,
        amount_issued: config.initial_supply,
    })
}

fn generate_keypair(name: &str, network: &str) -> Result<(), TokenSetupError> {
    run_cmd(
        "stellar",
        &[
            "keys",
            "generate",
            name,
            "--network",
            network,
            "--fund",
            "--overwrite",
        ],
        "generating keypair",
    )
    .map(|_| ())
}

fn get_public_key(name: &str) -> Result<String, TokenSetupError> {
    let output = run_cmd("stellar", &["keys", "address", name], "fetching public key")?;
    let trimmed = output.trim();
    if trimmed.starts_with('G') {
        Ok(trimmed.to_string())
    } else {
        Err(TokenSetupError::InvalidCommandOutput {
            context: "parsing account public key",
            output,
        })
    }
}

fn create_trustline(
    network: &str,
    distribution_key_name: &str,
    asset_code: &str,
    issuer: &str,
) -> Result<(), TokenSetupError> {
    run_cmd(
        "stellar",
        &[
            "tx",
            "new",
            "change-trust",
            "--source-account",
            distribution_key_name,
            "--network",
            network,
            "--line",
            &format!("{asset_code}:{issuer}"),
            "--sign",
            "--submit",
        ],
        "creating trustline",
    )
    .map(|_| ())
}

fn issue_asset(
    network: &str,
    issuing_key_name: &str,
    destination: &str,
    asset_code: &str,
    amount: i64,
) -> Result<(), TokenSetupError> {
    run_cmd(
        "stellar",
        &[
            "tx",
            "new",
            "payment",
            "--source-account",
            issuing_key_name,
            "--network",
            network,
            "--destination",
            destination,
            "--asset",
            asset_code,
            "--amount",
            &amount.to_string(),
            "--sign",
            "--submit",
        ],
        "issuing custom asset",
    )
    .map(|_| ())
}

fn run_cmd(bin: &str, args: &[&str], context: &'static str) -> Result<String, TokenSetupError> {
    let output =
        Command::new(bin)
            .args(args)
            .output()
            .map_err(|e| TokenSetupError::CommandFailed {
                command: format!("{bin} {}", args.join(" ")),
                stderr: e.to_string(),
            })?;

    if !output.status.success() {
        return Err(TokenSetupError::CommandFailed {
            command: format!("{bin} {}", args.join(" ")),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        return Err(TokenSetupError::InvalidCommandOutput {
            context,
            output: "<empty stdout>".to_string(),
        });
    }

    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_token_setup_configuration_is_aid_on_testnet() {
        let cfg = TokenSetupConfig::default();
        assert_eq!(cfg.network, "testnet");
        assert_eq!(cfg.asset_code, "AID");
        assert_eq!(cfg.initial_supply, 1_000_000);
    }

    #[test]
    fn asset_descriptor_uses_code_and_issuer() {
        let issuer = "GBRPYHIL2CI3XTFQSYL5X6R5XG7VQ6V5T66M3WQRG4G67NIYQW6T57R3";
        let asset = format!("{}:{}", "AID", issuer);
        assert_eq!(
            asset,
            "AID:GBRPYHIL2CI3XTFQSYL5X6R5XG7VQ6V5T66M3WQRG4G67NIYQW6T57R3"
        );
    }
}
