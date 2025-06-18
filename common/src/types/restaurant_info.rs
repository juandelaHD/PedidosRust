use serde::{Deserialize, Serialize};

/// Represents information about a restaurant
///
/// Components:
/// - `id`: A unique identifier for the restaurant.
/// - `position`: A tuple representing the restaurant's position in a 2D space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestaurantInfo {
    pub id: String,
    pub position: (f32, f32),
}
