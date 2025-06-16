use tokio::time::Duration;

const DELAY_SECONDS: u64 = 2;
pub const COORDINATE_SCALE: f32 = 10.0;
pub const NEARBY_RADIUS: f32 = 5.0;
pub const SUCCESS_PROBABILITY: f32 = 0.75;
pub const NUM_COORDINATORS: u16 = 4;
pub const BASE_PORT: u16 = 8080;
pub const TIMEOUT_SECONDS: u64 = 2;
pub const BASE_DELAY_MILLIS: u64 = 1000 * DELAY_SECONDS;
pub const SERVER_IP_ADDRESS: &str = "127.0.0.1";
pub const PAYMENT_GATEWAY_PORT: u16 = BASE_PORT + NUM_COORDINATORS + 1;
pub const PAYMENT_SUCCESS_PROBABILITY: f32 = 0.6;
pub const INTERVALO_HEARTBEAT: Duration = Duration::from_secs(2);
pub const TIMEOUT_HEARTBEAT: Duration = Duration::from_secs(3);
pub const NUMBER_OF_CHEFS: usize = 4;
