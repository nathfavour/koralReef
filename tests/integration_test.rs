// Placeholder for integration tests
// In a real scenario, this would mock the Solana RPC or use a local test validator

#[test]
fn test_config_load() {
    // This test assumes config.toml.example exists
    let content = std::fs::read_to_string("config.toml.example").unwrap();
    let config: koralreef::config::Config = toml::from_str(&content).unwrap();
    assert_eq!(config.settings.scan_interval_hours, 6);
}
