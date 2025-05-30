#![no_std]

use los_api::println;

extern "C" fn module_init() {
    println!("Hello, world! From kernel module!");
}
