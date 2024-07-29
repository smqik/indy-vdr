use std::path::PathBuf;

#[cfg(test)]
use std::env;

pub struct EnvironmentUtils {}

impl EnvironmentUtils {
    pub fn indy_home_path() -> PathBuf {
        // TODO: FIXME: Provide better handling for the unknown home path case!!!
        let mut path = dirs::home_dir().unwrap_or(PathBuf::from("/home/indy"));
        path.push(if cfg!(target_os = "ios") {
            "Documents/.indy_client"
        } else {
            ".indy_client"
        });
        path
    }

    pub fn wallet_home_path() -> PathBuf {
        let mut path = EnvironmentUtils::indy_home_path();
        path.push("wallet");
        path
    }

    pub fn wallet_path(wallet_name: &str) -> PathBuf {
        let mut path = EnvironmentUtils::wallet_home_path();
        path.push(wallet_name);
        path
    }

    pub fn wallets_path() -> PathBuf {
        let mut path = EnvironmentUtils::indy_home_path();
        path.push("wallets");
        path
    }

    pub fn wallet_config_path(id: &str) -> PathBuf {
        let mut path = Self::wallets_path();
        path.push(id);
        path.set_extension("json");
        path
    }

    pub fn pool_home_path() -> PathBuf {
        let mut path = EnvironmentUtils::indy_home_path();
        path.push("pool");
        path
    }

    pub fn pool_path(pool_name: &str) -> PathBuf {
        let mut path = EnvironmentUtils::pool_home_path();
        path.push(pool_name);
        path
    }

    pub fn pool_transactions_path(pool_name: &str) -> PathBuf {
        let mut path = EnvironmentUtils::pool_home_path();
        path.push(pool_name);
        path.push(pool_name);
        path.set_extension("txn");
        path
    }

    pub fn pool_config_path(id: &str) -> PathBuf {
        let mut path = Self::pool_home_path();
        path.push(id);
        path.push("config");
        path.set_extension("json");
        path
    }

    #[cfg(test)]
    pub fn tmp_path() -> PathBuf {
        let mut path = env::temp_dir();
        path.push("indy_client");
        path
    }

    #[cfg(test)]
    pub fn tmp_file_path(file_name: &str) -> PathBuf {
        let mut path = EnvironmentUtils::tmp_path();
        path.push(file_name);
        path
    }

    pub fn history_file_path() -> PathBuf {
        let mut path = EnvironmentUtils::indy_home_path();
        path.push("history");
        path.push("history.txt");
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indy_home_path_works() {
        let path = EnvironmentUtils::indy_home_path();

        assert!(path.is_absolute());
        assert!(path.has_root());
        assert!(path.to_string_lossy().contains(".indy_client"));
    }

    #[test]
    fn wallet_home_path_works() {
        let path = EnvironmentUtils::wallet_home_path();

        assert!(path.is_absolute());
        assert!(path.has_root());
        assert!(path.to_string_lossy().contains(".indy_client"));
        assert!(path.to_string_lossy().contains("wallet"));
    }

    #[test]
    fn wallet_path_works() {
        let path = EnvironmentUtils::wallet_path("wallet1");

        assert!(path.is_absolute());
        assert!(path.has_root());
        assert!(path.to_string_lossy().contains(".indy_client"));
        assert!(path.to_string_lossy().contains("wallet1"));
    }

    #[test]
    fn pool_home_path_works() {
        let path = EnvironmentUtils::pool_home_path();

        assert!(path.is_absolute());
        assert!(path.has_root());
        assert!(path.to_string_lossy().contains(".indy_client"));
        assert!(path.to_string_lossy().contains("pool"));
    }

    #[test]
    fn pool_path_works() {
        let path = EnvironmentUtils::pool_path("pool1");

        assert!(path.is_absolute());
        assert!(path.has_root());
        assert!(path.to_string_lossy().contains(".indy_client"));
        assert!(path.to_string_lossy().contains("pool1"));
    }

    #[test]
    fn tmp_path_works() {
        let path = EnvironmentUtils::tmp_path();

        assert!(path.is_absolute());
        assert!(path.has_root());
        assert!(path.to_string_lossy().contains("indy_client"));
    }

    #[test]
    fn tmp_file_path_works() {
        let path = EnvironmentUtils::tmp_file_path("test.txt");

        assert!(path.is_absolute());
        assert!(path.has_root());
        assert!(path.to_string_lossy().contains("indy_client"));
        assert!(path.to_string_lossy().contains("test.txt"));
    }
}
