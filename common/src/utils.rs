use crate::constants::COORDINATE_SCALE;
use crate::constants::SUCCESS_PROBABILITY;
use rand::random;

pub fn get_rand_f32_tuple() -> (f32, f32) {
    (
        (rand::random::<f32>() * COORDINATE_SCALE).round(),
        (rand::random::<f32>() * COORDINATE_SCALE).round(),
    )
}

pub fn random_bool_by_probability() -> bool {
    let rand_value: f32 = random();
    rand_value < SUCCESS_PROBABILITY
}

pub fn random_bool_by_given_probability(probability: f32) -> bool {
    let rand_value: f32 = random();
    println!("Random bool with probability: {}", probability);
    println!("Result is: {}", rand_value < probability);

    rand_value < probability
}

pub fn calculate_distance(point1: (f32, f32), point2: (f32, f32)) -> f32 {
    let dx = (point1.0 - point2.0).abs();
    let dy = (point1.1 - point2.1).abs();
    dx + dy
}
