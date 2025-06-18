/// Enum representing the type of a peer in the distributed system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerType {
    /// A client peer.
    ClientType,
    /// A restaurant peer.
    RestaurantType,
    /// A delivery agent peer.
    DeliveryType,
    /// A coordinator peer.
    CoordinatorType,
    /// A payment gateway peer.
    GatewayType,
}

impl PeerType {
    /// Converts a `u8` value to a `PeerType` if possible.
    ///
    /// # Arguments
    /// - `byte`: The byte to convert.
    ///
    /// # Returns
    /// - `Some(PeerType)` if the byte matches a known type, otherwise `None`.
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(PeerType::ClientType),
            1 => Some(PeerType::RestaurantType),
            2 => Some(PeerType::DeliveryType),
            3 => Some(PeerType::CoordinatorType),
            4 => Some(PeerType::GatewayType),
            _ => None,
        }
    }

    /// Converts a `PeerType` to its corresponding `u8` value.
    pub fn to_u8(&self) -> u8 {
        match self {
            PeerType::ClientType => 0,
            PeerType::RestaurantType => 1,
            PeerType::DeliveryType => 2,
            PeerType::CoordinatorType => 3,
            PeerType::GatewayType => 4,
        }
    }
}
