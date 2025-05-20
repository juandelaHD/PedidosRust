pub mod client_messages;
pub mod restaurant_messages;
pub mod delivery_messages;
pub mod admin_messages;
pub mod payment_messages;
pub mod shared_messages;

// Optional: reexport all together for `use common::messages::*`
pub use client_messages::*;
pub use payment_messages::*;
/*
pub use restaurant_messages::*;
pub use delivery_messages::*;
pub use admin_messages::*;
pub use shared_messages::*;
*/