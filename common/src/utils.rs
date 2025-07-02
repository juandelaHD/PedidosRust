use crate::constants::COORDINATE_SCALE;
//use crate::constants::SUCCESS_PROBABILITY;
use rand::random;
use std::io::{self, Write};

pub fn get_rand_f32_tuple() -> (f32, f32) {
    (
        (rand::random::<f32>() * COORDINATE_SCALE).round(),
        (rand::random::<f32>() * COORDINATE_SCALE).round(),
    )
}

pub fn random_bool_by_given_probability(probability: f32) -> bool {
    let rand_value: f32 = random();
    rand_value < probability
}

pub fn calculate_distance(point1: (f32, f32), point2: (f32, f32)) -> f32 {
    let dx = (point1.0 - point2.0).abs();
    let dy = (point1.1 - point2.1).abs();
    dx + dy
}

pub fn print_welcome_message() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();

    println!("===========================================");
    println!("🦀 Welcome to PedidosRust! 🛵💨");
    println!("-------------------------------------------");
    println!("Your fast, secure, and efficient ordering system.");
    println!("===========================================\n");
}
