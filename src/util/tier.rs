use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum RetrievalTier {
    Expedited,
    Standard,
    Bulk,
}

#[derive(Error, Debug)]
pub enum RetrievalTierParseError {
    #[error("Invalid retrieval tier: {0}")]
    InvalidTier(String),
}

impl From<&RetrievalTier> for String {
    fn from(tier: &RetrievalTier) -> Self {
        match tier {
            RetrievalTier::Expedited => "Expedited".to_string(),
            RetrievalTier::Standard => "Standard".to_string(),
            RetrievalTier::Bulk => "Bulk".to_string(),
        }
    }
}

impl TryFrom<&str> for RetrievalTier {
    type Error = RetrievalTierParseError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "expedited" => Ok(RetrievalTier::Expedited),
            "standard" => Ok(RetrievalTier::Standard),
            "bulk" => Ok(RetrievalTier::Bulk),
            _ => Err(RetrievalTierParseError::InvalidTier(s.to_string())),
        }
    }
}

pub fn parse_retrieval_tier(s: &str) -> Result<RetrievalTier, RetrievalTierParseError> {
    RetrievalTier::try_from(s)
}
