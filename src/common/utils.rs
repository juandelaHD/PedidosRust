use rand::random;
use crate::common::constants::SUCCESS_PROBABILITY;
use crate::common::constants::COORDINATE_SCALE;

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
