use crate::result::{Error as EasyError, Result as EasyResult};
use crate::util::client::get_client;
use crate::util::vault::{GlacierVaultSpec, parse_glacier_vault_arn};
use aws_sdk_glacier::types::JobParameters;

/// Initiate an inventory job for a vault.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[arg(value_parser = parse_glacier_vault_arn)]
    /// ARN for the vault.
    /// Example: arn:aws:glacier:us-east-2:123456789012:vaults/video-archives
    arn: GlacierVaultSpec,
}

impl Cmd {
    pub async fn run(&self) -> EasyResult {
        let client = get_client(&self.arn).await;
        let input = JobParameters::builder()
            .r#type("inventory-retrieval")
            .build();
        let output = client.initiate_job().job_parameters(input).send().await?;
        let job_id = output
            .job_id()
            .ok_or(EasyError::msg("No job id returned"))?;
        println!("JobId: {}", job_id);
        Ok(())
    }
}
