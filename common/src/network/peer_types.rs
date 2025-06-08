#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerType {
    ClientType,
    RestaurantType,
    DeliveryType,
    CoordinatorType,
    GatewayType,
}
