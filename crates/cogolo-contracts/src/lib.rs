//! Core contract types for Cogolo.

/// Returns the crate purpose as a stable placeholder.
#[must_use]
pub fn crate_name() -> &'static str {
    "cogolo-contracts"
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_name() {
        assert_eq!(super::crate_name(), "cogolo-contracts");
    }
}
