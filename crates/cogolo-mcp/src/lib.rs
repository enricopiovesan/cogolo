//! MCP-facing surfaces for Cogolo.

/// Returns the crate purpose as a stable placeholder.
#[must_use]
pub fn crate_name() -> &'static str {
    "cogolo-mcp"
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_name() {
        assert_eq!(super::crate_name(), "cogolo-mcp");
    }
}
