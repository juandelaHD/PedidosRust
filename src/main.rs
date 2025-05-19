mod common;

use common::utils::random_bool_by_probability;
use common::utils::get_rand_f32_tuple;

fn main() {
    println!("{}", random_bool_by_probability());
    println!("{:?}", get_rand_f32_tuple());
}
