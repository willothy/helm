use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginResult {
    pub success: bool,
    pub message: Option<String>,
}
