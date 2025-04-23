use crate::relais::{RelaisCommand, RelaisMsg, RELAIS_CHANNEL};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embedded_can::Frame;
use esp_hal::gpio::interconnect::{PeripheralInput, PeripheralOutput};
use esp_hal::peripheral::Peripheral;
use esp_hal::twai::{self, EspTwaiFrame, TimingConfig, TwaiMode, TwaiRx, TwaiTx};
use esp_hal::Async;
use esp_println::println;
use static_cell::StaticCell;

pub static CAN_CHANNEL: Channel<CriticalSectionRawMutex, EspTwaiFrame, 8> = Channel::new();
static TWAI_RX: StaticCell<TwaiRx<'_, Async>> = StaticCell::new();
static TWAI_TX: StaticCell<TwaiTx<'_, Async>> = StaticCell::new();

pub struct Can {}

impl Can {
    pub fn new<RX: PeripheralInput, TX: PeripheralOutput>(
        twai: esp_hal::peripherals::TWAI0,
        rx: impl Peripheral<P = RX> + 'static,
        tx: impl Peripheral<P = TX> + 'static,
        spawner: &Spawner,
    ) -> Can {
        const TC: TimingConfig = TimingConfig {
            baud_rate_prescaler: 80,
            sync_jump_width: 3,
            tseg_1: 15,
            tseg_2: 4,
            triple_sample: false,
        };

        const TWAI_BAUDRATE: twai::BaudRate = twai::BaudRate::Custom(TC);

        let twai_config =
            twai::TwaiConfiguration::new(twai, rx, tx, TWAI_BAUDRATE, TwaiMode::Normal);

        let twai = twai_config.into_async().start();

        let (rx, tx) = twai.split();

        let rx = TWAI_RX.init(rx);
        let tx = TWAI_TX.init(tx);

        spawner.spawn(can_recieve_task(rx)).unwrap();

        spawner.spawn(can_send_task(tx)).unwrap();

        Can {}
    }
    pub async fn start(&self) {
        let id = esp_hal::twai::ExtendedId::new(0x1008AA00).unwrap();
        let message = EspTwaiFrame::new(id, &[1u8]).unwrap();
        CAN_CHANNEL.send(message).await;
    }
}

// === CAN Task ===

#[embassy_executor::task]
pub async fn can_recieve_task(rx: &'static mut TwaiRx<'static, Async>) {
    println!("can_recieve_task started");
    loop {
        if let Ok(frame) = rx.receive_async().await {
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

#[embassy_executor::task]
pub async fn can_send_task(tx: &'static mut TwaiTx<'static, Async>) {
    println!("can_send_task started");
    loop {
        let frame = CAN_CHANNEL.receive().await;
        println!("can_send_task:{frame:?}");
        tx.transmit_async(&frame).await.unwrap();
    }
}
