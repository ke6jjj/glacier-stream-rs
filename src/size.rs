#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SizeUnit {
    B,
    KB,
    MB,
    GB,
    TB,
}

#[derive(Debug, Clone)]
pub struct SizeSpec {
    size: f64,
    unit: SizeUnit,
}

#[derive(thiserror::Error, Debug)]
pub enum SizeParseError {
    #[error("Invalid size specification: {0}")]
    InvalidFormat(String),
}

impl std::fmt::Display for SizeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.size,
            match self.unit {
                SizeUnit::B => "b",
                SizeUnit::KB => "k",
                SizeUnit::MB => "m",
                SizeUnit::GB => "g",
                SizeUnit::TB => "t",
            }
        )
    }
}

impl std::str::FromStr for SizeSpec {
    type Err = SizeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| SizeParseError::InvalidFormat(s.into()))
    }
}

impl SizeSpec {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let (num_part, unit_part) = s
            .chars()
            .partition::<String, _>(|c| c.is_ascii_digit() || *c == '.');
        let size = num_part.parse::<f64>().ok()?;
        let unit = match unit_part.trim().to_uppercase().as_str() {
            "B" => SizeUnit::B,
            "K" => SizeUnit::KB,
            "M" => SizeUnit::MB,
            "G" => SizeUnit::GB,
            "T" => SizeUnit::TB,
            _ => return None,
        };
        Some(SizeSpec { size, unit })
    }

    pub fn to_bytes(&self) -> u64 {
        let multiplier = match self.unit {
            SizeUnit::B => 1.0,
            SizeUnit::KB => 1024.0,
            SizeUnit::MB => 1024.0 * 1024.0,
            SizeUnit::GB => 1024.0 * 1024.0 * 1024.0,
            SizeUnit::TB => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        };
        (self.size * multiplier) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_spec_parse() {
        let spec = SizeSpec::parse("1.5g").unwrap();
        assert_eq!(spec.size, 1.5);
        assert_eq!(spec.unit, SizeUnit::GB);
        assert_eq!(spec.to_bytes(), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
    }

    #[test]
    fn test_size_spec_parse_error() {
        assert!(SizeSpec::parse("invalid").is_none());
    }
}
