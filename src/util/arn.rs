use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AWSResource {
    resource_type: String,
    account_id: String,
    region: String,
    id: String,
}

#[derive(Debug, Error)]
pub enum AWSResourceParseError {
    #[error("Invalid ARN format")]
    InvalidFormat,
}

impl TryFrom<&str> for AWSResource {
    type Error = AWSResourceParseError;

    fn try_from(arn: &str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = arn.split(':').collect();
        if parts.len() != 6 || parts[0] != "arn" || parts[1] != "aws" {
            return Err(AWSResourceParseError::InvalidFormat);
        }
        Ok(AWSResource {
            resource_type: parts[2].to_string(),
            account_id: parts[4].to_string(),
            region: parts[3].to_string(),
            id: parts[5].to_string(),
        })
    }
}

impl AWSResource {
    pub fn resource_type(&self) -> &str {
        &self.resource_type
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

pub fn parse_aws_resource(arn: &str) -> Result<AWSResource, AWSResourceParseError> {
    AWSResource::try_from(arn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arn_parsing() {
        let arn_str = "arn:aws:glacier:us-east-2:123456789012:vaults/video-archives";
        let resource = AWSResource::try_from(arn_str).expect("Failed to parse ARN");
        assert_eq!(resource.resource_type, "glacier");
        assert_eq!(resource.account_id, "123456789012");
        assert_eq!(resource.region, "us-east-2");
        assert_eq!(resource.id, "vaults/video-archives");
    }
}