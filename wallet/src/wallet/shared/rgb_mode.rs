use serde::{Deserialize, Serialize};

/// RGB operation mode
/// 
/// Determines how RGB state is managed and coordinated:
/// - Traditional: Consignment-based (local stash, manual file sharing)
/// - F1r3fly: State-based (F1r3fly node coordination, instant notifications)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RgbMode {
    /// Traditional consignment-based RGB operations
    /// 
    /// Uses:
    /// - RGB runtime for validation and signing
    /// - Local stash for state storage
    /// - Consignment files for state transfer
    /// - Manual file sharing (email/IPFS/upload)
    Traditional,
    
    /// F1r3fly state-based RGB operations
    /// 
    /// Uses:
    /// - RGB runtime for validation and signing (security)
    /// - F1r3fly node for state storage (coordination)
    /// - Bitcoin blockchain for final validation (trust anchor)
    /// - Instant notifications via RSpace
    F1r3fly,
}

impl Default for RgbMode {
    fn default() -> Self {
        RgbMode::Traditional
    }
}

impl std::fmt::Display for RgbMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RgbMode::Traditional => write!(f, "traditional"),
            RgbMode::F1r3fly => write!(f, "f1r3fly"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_traditional() {
        assert_eq!(RgbMode::default(), RgbMode::Traditional);
    }

    #[test]
    fn test_display() {
        assert_eq!(RgbMode::Traditional.to_string(), "traditional");
        assert_eq!(RgbMode::F1r3fly.to_string(), "f1r3fly");
    }

    #[test]
    fn test_serialization() {
        let traditional = RgbMode::Traditional;
        let json = serde_json::to_string(&traditional).unwrap();
        assert_eq!(json, "\"traditional\"");

        let f1r3fly = RgbMode::F1r3fly;
        let json = serde_json::to_string(&f1r3fly).unwrap();
        assert_eq!(json, "\"f1r3fly\"");
    }

    #[test]
    fn test_deserialization() {
        let traditional: RgbMode = serde_json::from_str("\"traditional\"").unwrap();
        assert_eq!(traditional, RgbMode::Traditional);

        let f1r3fly: RgbMode = serde_json::from_str("\"f1r3fly\"").unwrap();
        assert_eq!(f1r3fly, RgbMode::F1r3fly);
    }
}

