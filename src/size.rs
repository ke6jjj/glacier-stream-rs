#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SizeUnit {
    B,
    KB,
    MB,
    GB,
    TB,
}

pub struct SizeSpec {
    size: f64,
    unit: SizeUnit,
}

impl SizeSpec {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let (num_part, unit_part) = s
            .chars()
            .partition::<String, _>(|c| c.is_digit(10) || *c == '.');
        let size = num_part.parse::<f64>().ok()?;
        let unit = match unit_part.trim().to_uppercase().as_str() {
            "B" => SizeUnit::B,
            "KB" => SizeUnit::KB,
            "MB" => SizeUnit::MB,
            "GB" => SizeUnit::GB,
            "TB" => SizeUnit::TB,
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
        let spec = SizeSpec::parse("1.5 GB").unwrap();
        assert_eq!(spec.size, 1.5);
        assert_eq!(spec.unit, SizeUnit::GB);
        assert_eq!(spec.to_bytes(),(1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
    }
}