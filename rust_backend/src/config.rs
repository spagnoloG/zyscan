use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub db_connection: String,
    pub db_name: String,
    pub scan_folders: Vec<String>,
}

// Default config
impl ::std::default::Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            db_connection: "mongodb://localhost:27017".to_string(),
            db_name: "zyscan".to_string(),
            scan_folders: vec!["/home/gasperspagnolo/Nextcloud/InstantUpload/Camera/".to_string()],
        }
    }
}
