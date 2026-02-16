use std::collections::HashMap;
use std::sync::Arc;

use crate::metadata::CalculationMetadata;
use crate::traits::Calculation;

/// Holds all registered calculations, keyed by their metadata id.
pub struct CalculationRegistry {
    calculations: HashMap<String, Arc<dyn Calculation>>,
}

impl CalculationRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            calculations: HashMap::new(),
        }
    }

    /// Create a registry pre-populated with all built-in calculations.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(crate::calculations::sma::Sma));
        registry.register(Arc::new(crate::calculations::ema::Ema));
        registry.register(Arc::new(crate::calculations::rsi::Rsi));
        registry
    }

    /// Register a calculation. Overwrites any existing registration with the same id.
    pub fn register(&mut self, calc: Arc<dyn Calculation>) {
        let id = calc.metadata().id.clone();
        self.calculations.insert(id, calc);
    }

    /// Look up a calculation by id.
    pub fn get(&self, id: &str) -> Option<&Arc<dyn Calculation>> {
        self.calculations.get(id)
    }

    /// List all registered calculation ids, sorted alphabetically.
    pub fn list_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.calculations.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Get metadata for all registered calculations, sorted by id.
    pub fn list_metadata(&self) -> Vec<CalculationMetadata> {
        let mut metas: Vec<_> = self.calculations.values().map(|c| c.metadata()).collect();
        metas.sort_by(|a, b| a.id.cmp(&b.id));
        metas
    }

    /// Export a JSON manifest of all registered calculations.
    pub fn export_manifest(&self) -> Result<String, serde_json::Error> {
        let metas = self.list_metadata();
        serde_json::to_string_pretty(&metas)
    }
}

impl Default for CalculationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_has_no_calculations() {
        let registry = CalculationRegistry::new();
        assert!(registry.list_ids().is_empty());
        assert!(registry.list_metadata().is_empty());
    }

    #[test]
    fn with_defaults_registers_all_builtins() {
        let registry = CalculationRegistry::with_defaults();
        let ids = registry.list_ids();
        assert_eq!(ids, vec!["ema", "rsi", "sma"]);
    }

    #[test]
    fn get_returns_none_for_unknown_id() {
        let registry = CalculationRegistry::with_defaults();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn get_returns_some_for_known_id() {
        let registry = CalculationRegistry::with_defaults();
        assert!(registry.get("sma").is_some());
        assert!(registry.get("ema").is_some());
        assert!(registry.get("rsi").is_some());
    }

    #[test]
    fn duplicate_registration_overwrites() {
        let mut registry = CalculationRegistry::new();
        registry.register(Arc::new(crate::calculations::sma::Sma));
        registry.register(Arc::new(crate::calculations::sma::Sma));
        assert_eq!(registry.list_ids().len(), 1);
    }

    #[test]
    fn export_manifest_produces_valid_json() {
        let registry = CalculationRegistry::with_defaults();
        let json = registry.export_manifest().unwrap();
        let parsed: Vec<CalculationMetadata> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].id, "ema");
        assert_eq!(parsed[1].id, "rsi");
        assert_eq!(parsed[2].id, "sma");
    }

    #[test]
    fn list_ids_is_sorted() {
        let registry = CalculationRegistry::with_defaults();
        let ids = registry.list_ids();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }
}
