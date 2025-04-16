#![no_std]
#![no_main]


use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    main,
};
use esp_println::println;
use nb::block;
use raffstore::relais::Relais;
use raffstore::can::Can;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let delay = Delay::new();

    let mut can = Can::new(peripherals.TWAI0, peripherals.GPIO14, peripherals.GPIO13);

    can.start();

    let mut relais = Relais::new(peripherals.I2C0, peripherals.GPIO21, peripherals.GPIO19);
    relais.set(0,true);
    delay.delay_millis(2000);
    relais.set(1,true);
    delay.delay_millis(2000);
    relais.set(2,true);
    delay.delay_millis(2000);
    relais.set(3,true);
    delay.delay_millis(200);
    relais.set(4,true);
    delay.delay_millis(200);
    relais.set(5,true);
    delay.delay_millis(200);
    relais.set(6,true);
    delay.delay_millis(200);
    relais.set(7,true);
    delay.delay_millis(200);
    relais.set(8,true);
    delay.delay_millis(200);
    relais.set(9,true);
    delay.delay_millis(200);
    relais.set(10,true);
    delay.delay_millis(200);
    relais.set(11,true);
    delay.delay_millis(200);

loop {
    // let frame = block!(twai.receive()).unwrap();

    // println!("Received a frame: {frame:?}");
    // delay.delay_millis(1000);

}
}

// for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
