use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestaurantInfo {
    pub id: String,
    pub position: (f64, f64),
}
