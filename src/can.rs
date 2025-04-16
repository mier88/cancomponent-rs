use esp_hal::twai::{self, TwaiMode, TimingConfig, Twai};
use esp_hal::Async;
use esp_hal::gpio::interconnect::{PeripheralOutput, PeripheralInput};
use esp_hal::peripheral::Peripheral;
use crate::relais::{RelaisMsg, RELAIS_CHANNEL, RelaisCommand};
use embedded_can::Frame;
use esp_println::println;
use embassy_time::{Duration,Timer};

pub struct Can<'a> {
    twai: Twai<'a, Async>,
}

impl<'a> Can<'a> {
    pub fn new <RX: PeripheralInput, TX: PeripheralOutput> (twai: esp_hal::peripherals::TWAI0, rx: impl Peripheral<P = RX> + 'a, tx: impl Peripheral<P = TX> + 'a,) -> Can<'a> {
         
        const TC: TimingConfig = TimingConfig {
            baud_rate_prescaler: 80,
            sync_jump_width: 3,
            tseg_1: 15,
            tseg_2: 4,
            triple_sample: false,
        };
    
        const TWAI_BAUDRATE: twai::BaudRate = twai::BaudRate::Custom(TC);
    
        let twai_config = twai::TwaiConfiguration::new_no_transceiver(
            twai,
            rx,
            tx,
            TWAI_BAUDRATE,
            TwaiMode::Normal,
        );
    
        let twai = twai_config
        .into_async()
        .start();

         Can{twai}
     }
     pub fn start (&self){}
     pub fn send (&self){}
}

// === CAN Task ===



#[embassy_executor::task]
pub async fn can_task(mut can: Can<'static>) {
    println!("can_task started");
    loop {
        if let Ok(frame) = can.twai.receive_async().await {
            let data = frame.data();
            println!("data:{data:?}");
            if let Some((num, on, time)) = RelaisMsg::parse(data) {
                RELAIS_CHANNEL
                    .send(RelaisCommand::Set {
                        num,
                        on,
                        duration_ms: time,
                    })
                    .await;
            }
        }
    }
}