#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Duration;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::timer::timg::TimerGroup;
use esp_hal_embassy::main;
use raffstore::can::Can;
use raffstore::relais::{relais_task, Relais};

#[main]
async fn main(spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    let can = Can::new(
        peripherals.TWAI0,
        peripherals.GPIO14,
        peripherals.GPIO13,
        &spawner,
    );

    can.start().await;

    let relais = Relais::new(peripherals.I2C0, peripherals.GPIO21, peripherals.GPIO19);

    spawner.spawn(relais_task(relais)).unwrap();

    loop {
        // let frame = block!(twai.receive()).unwrap();
        // println!("Bla");
        Timer::after(Duration::from_millis(3_000)).await;
    }
}

// // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
