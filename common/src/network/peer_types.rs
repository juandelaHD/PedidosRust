#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerType {
    ClientType,
    RestaurantType,
    DeliveryType,
    CoordinatorType,
    GatewayType,
}

impl PeerType {
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
