use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NewDeliveryOffer {
    pub order_id: String,
    pub restaurant_location: (f32, f32),
    pub delivery_location: (f32, f32),
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OfferDecision {
    pub order_id: String,
    pub accepted: bool,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct TrackLocation {
    pub current_position: (f32, f32),
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PickupOrder {
    pub order_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliverOrder {
    pub order_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CancelOrder {
    pub order_id: String,
}
