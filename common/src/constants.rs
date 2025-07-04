use tokio::time::Duration;

const DELAY_SECONDS: u64 = 2;
pub const COORDINATE_SCALE: f32 = 10.0;
pub const NEARBY_RADIUS: f32 = 8.0; // blocks
pub const PAYMENT_SUCCESS_PROBABILITY: f32 = 0.95;
pub const RESTAURANT_SUCCESS_PROBABILITY: f32 = 0.9;
pub const DELIVERY_SUCCESS_PROBABILITY: f32 = 0.8;
pub const NUM_COORDINATORS: u16 = 4;
pub const BASE_PORT: u16 = 8080;
pub const TIMEOUT_SECONDS: u64 = 2;
pub const BASE_DELAY_MILLIS: u64 = 1000 * DELAY_SECONDS;
pub const SERVER_IP_ADDRESS: &str = "127.0.0.1";
pub const PAYMENT_GATEWAY_PORT: u16 = BASE_PORT + NUM_COORDINATORS + 1;
pub const INTERVAL_HEARTBEAT: Duration = Duration::from_secs(6);
pub const INTERVAL_STORAGE: Duration = Duration::from_secs(2);
pub const TIMEOUT_HEARTBEAT: Duration = Duration::from_secs(4);
pub const TIMEOUT_LEADER_RESPONSE: Duration = Duration::from_secs(5);
pub const NUMBER_OF_CHEFS: usize = 4;
pub const DEFAULT_TIME_TO_COOK: u64 = 8;
pub const DELAY_SECONDS_TO_START_RECONNECT: Duration = Duration::from_secs(3);
pub const REAP_TIMEOUT: Duration = Duration::from_secs(10);
