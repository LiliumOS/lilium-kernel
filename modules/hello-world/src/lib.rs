#![no_std]

use los_api::println;

#[unsafe(no_mangle)]
extern "C" fn module_init() {
    println!("Hello, world! From kernel module!");
}
