use super::models::GameCandidate;

pub trait DiscoveryProvider {
    /// Returns the name of the provider (e.g., "Steam", "GOG")
    fn name(&self) -> &'static str;

    /// Scans the system for installed games
    fn discover_games(&self) -> Result<Vec<GameCandidate>, String>;
}
