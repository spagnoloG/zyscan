use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub db_connection: String,
    pub db_name: String,
    pub scan_folders: Vec<String>,
    pub python_venv_path: String,
}

impl Clone for AppConfig {
    fn clone(&self) -> Self {
        AppConfig {
            db_connection: self.db_connection.clone(),
            db_name: self.db_name.clone(),
            scan_folders: self.scan_folders.clone(),
            python_venv_path: self.python_venv_path.clone(),
        }
    }
}

impl ::std::default::Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            db_connection: "mongodb://localhost:27017".to_string(),
            db_name: "zyscan".to_string(),
            scan_folders: vec!["/home/gasperspagnolo/Nextcloud/InstantUpload/Camera/".to_string(),
                                "/home/gasperspagnolo/Nextcloud/InstantUpload/Google Foto/".to_string()],
            python_venv_path: "/home/gasperspagnolo/miniconda3/envs/zyscan/bin/python".to_string(),
        }
    }
}
