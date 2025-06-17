use crate::can_id::CanId;
use crate::can_message_type::CanMessageType;
use crate::device::device;
use crate::update::update;
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

pub async fn init<RX: PeripheralInput, TX: PeripheralOutput>(
    twai: esp_hal::peripherals::TWAI0,
    rx: impl Peripheral<P = RX> + 'static,
    tx: impl Peripheral<P = TX> + 'static,
    spawner: &Spawner,
) {
    const TC: TimingConfig = TimingConfig {
        baud_rate_prescaler: 80,
        sync_jump_width: 3,
        tseg_1: 15,
        tseg_2: 4,
        triple_sample: false,
    };

    const TWAI_BAUDRATE: twai::BaudRate = twai::BaudRate::Custom(TC);

    let twai_config = twai::TwaiConfiguration::new(twai, rx, tx, TWAI_BAUDRATE, TwaiMode::Normal);

    let twai = twai_config.into_async().start();

    let (rx, tx) = twai.split();

    let rx = TWAI_RX.init(rx);
    let tx = TWAI_TX.init(tx);

    spawner.spawn(can_recieve_task(rx)).unwrap();

    spawner.spawn(can_send_task(tx)).unwrap();

    let id = CanId::new(0x08u8, 0xAAu8, CanMessageType::Available);
    let message =
        EspTwaiFrame::new(<CanId as Into<esp_hal::twai::ExtendedId>>::into(id), &[1u8]).unwrap();
    CAN_CHANNEL.send(message).await;
}

// Typ für die CAN-Handler-Funktion
pub async fn dispatch(frame: &EspTwaiFrame) {
    let id = match frame.id() {
        embedded_can::Id::Extended(id) => CanId::try_from(id).unwrap(), // Nur das letzte Byte relevant
        embedded_can::Id::Standard(id) => {
            println!("WARN: Ignoring standard ID: {:?}", id);
            return;
        }
    };
    match id.msg_type {
        CanMessageType::Relais => {
            crate::relais::relais_handler(id, frame.data(), frame.is_remote_frame()).await
        }
        CanMessageType::Uptime => {
            device()
                .await
                .uptime(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::RequestParameter => {
            device()
                .await
                .request_parameter(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::DeviceUid0 => {
            device()
                .await
                .uid0(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::DeviceUid1 => {
            device()
                .await
                .uid1(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::CustomString => {
            device()
                .await
                .custom_string(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::DeviceIdType => {
            device()
                .await
                .id_type(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::Baudrate => {
            device()
                .await
                .baudrate(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::HwRev => {
            device()
                .await
                .hardware_revision(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::Restart => {
            device()
                .await
                .restart(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::FlashStart => {
            update()
                .await
                .start(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::FlashProgress => {
            update()
                .await
                .progress(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::FlashSelect => {
            update()
                .await
                .select(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::FlashRead => {
            update()
                .await
                .read(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::FlashWrite => {
            update()
                .await
                .write(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::FlashVerify => {
            update()
                .await
                .verify(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::FlashErase => {
            update()
                .await
                .erase(id, frame.data(), frame.is_remote_frame())
                .await
        }
        CanMessageType::UpdateSilence => silence(frame).await,
        _ => unknown_handler(frame).await,
    }
}

async fn silence(_frame: &EspTwaiFrame) {}

async fn unknown_handler(frame: &EspTwaiFrame) {
    let id = match frame.id() {
        embedded_can::Id::Extended(id) => id.as_raw(), // Nur das letzte Byte relevant
        embedded_can::Id::Standard(id) => {
            println!("WARN: Ignoring standard ID: {:?}", id);
            return;
        }
    };
    println!("Unknown msg ID {}, payload: {:?}", id, frame.data());
}

pub async fn send_can_message(msg_id: CanMessageType, data: &[u8], rtr: bool) {
    let device_type = 0x12u8; // <- kommt bei dir dynamisch aus Config
    let device_id = 0x34u8; // <- dito

    let id: esp_hal::twai::ExtendedId = CanId::new(device_id, device_type, msg_id).into();

    let frame = if rtr {
        EspTwaiFrame::new_remote(id, data.len() as usize).unwrap()
    } else {
        EspTwaiFrame::new(id, data).unwrap()
    };

    CAN_CHANNEL.send(frame).await
}

// === CAN Task ===

#[embassy_executor::task]
pub async fn can_recieve_task(rx: &'static mut TwaiRx<'static, Async>) {
    println!("can_recieve_task started");
    loop {
        if let Ok(frame) = rx.receive_async().await {
            dispatch(&frame).await;
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
