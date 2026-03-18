use thiserror::Error;
use super::arn::AWSResource;

#[derive(Debug, Clone)]
pub struct GlacierVaultSpec {
    account_id: String,
    region: String,
    vault_name: String,
}

#[derive(Debug, Error)]
pub enum GlacierVaultARNError {
    #[error("Not a Glacier vault ARN")]
    NotGlacierVault,
    #[error("Malformed vault identifier")]
    MalformedVaultIdentifier,
}

impl GlacierVaultSpec {
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn vault_name(&self) -> &str {
        &self.vault_name
    }
}

impl TryFrom<&AWSResource> for GlacierVaultSpec {
    type Error = GlacierVaultARNError;

    fn try_from(resource: &AWSResource) -> Result<Self, Self::Error> {
        if resource.resource_type() != "glacier" {
            return Err(GlacierVaultARNError::NotGlacierVault);
        }
        if !resource.id().starts_with("vaults/") {
            return Err(GlacierVaultARNError::MalformedVaultIdentifier);
        }
        let vault_name = resource.id().trim_start_matches("vaults/").to_string();
        Ok(GlacierVaultSpec {
            account_id: resource.account_id().to_owned(),
            region: resource.region().to_owned(),
            vault_name,
        })
    }
}

impl TryFrom<&str> for GlacierVaultSpec {
    type Error = GlacierVaultARNError;

    fn try_from(arn: &str) -> Result<Self, Self::Error> {
        let resource = AWSResource::try_from(arn).map_err(|_| GlacierVaultARNError::NotGlacierVault)?;
        Self::try_from(&resource)
    }
}

pub fn parse_glacier_vault_arn(arn: &str) -> Result<GlacierVaultSpec, GlacierVaultARNError> {
    GlacierVaultSpec::try_from(arn)
}