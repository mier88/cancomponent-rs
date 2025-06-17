use crate::can_id::CanId;
use crate::relais_message::RelaisMsg;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::i2c::master::{Config, I2c};
use esp_hal::peripheral::Peripheral;
use esp_hal::Async;
use esp_println::println;

const BANK1: u8 = 0x26;
const BANK2: u8 = 0x27;

static RELAIS_CHANNEL: Channel<CriticalSectionRawMutex, RelaisCommand, 8> = Channel::new();

pub async fn relais_handler(_id: CanId, data: &[u8], _remote_request: bool) {
    if let Some((num, on, time)) = RelaisMsg::parse(data) {
        RELAIS_CHANNEL
            .send(RelaisCommand::Set {
                num,
                on,
                duration_ms: time,
            })
            .await;
    } else {
        println!("RelaisHandler: invalid frame data: {:?}", data);
    }
}

#[derive(Debug)]
pub enum RelaisCommand {
    Set {
        num: usize,
        on: bool,
        duration_ms: Option<u64>,
    },
}

pub struct Relais {
    i2c: I2c<'static, Async>,
    expanders: [u8; 2],
}

impl Relais {
    pub fn init<SDA: PeripheralOutput + 'static, SCL: PeripheralOutput + 'static>(
        i2c0: esp_hal::peripherals::I2C0,
        sda: impl Peripheral<P = SDA> + 'static,
        scl: impl Peripheral<P = SCL> + 'static,
        spawner: &Spawner,
    ) {
        let mut i2c = I2c::new(i2c0, Config::default())
            .unwrap()
            .with_sda(sda)
            .with_scl(scl)
            .into_async();

        i2c.write(BANK1, &[0x3, 0x0]).ok();
        i2c.write(BANK2, &[0x3, 0x0]).ok();
        i2c.write(BANK1, &[0x1, 0x0]).ok();
        i2c.write(BANK2, &[0x1, 0x0]).ok();

        let relais = Relais {
            expanders: [0, 0],
            i2c,
        };

        spawner.spawn(relais_task(relais)).unwrap();
    }
    /// Each entry: (expander index, bit position)
    const MAPPING: [(usize, u8); 12] = [
        (0, 3),
        (0, 2),
        (0, 1),
        (0, 7),
        (0, 6),
        (0, 5),
        (0, 4),
        (1, 11 - 8),
        (1, 10 - 8),
        (1, 9 - 8),
        (1, 15 - 8),
        (1, 14 - 8),
    ];

    pub fn set(&mut self, num: usize, onoff: bool) {
        if let Some(&(expander, bit)) = Self::MAPPING.get(num) {
            let mask = 1 << bit;
            if onoff {
                self.expanders[expander] |= mask;
            } else {
                self.expanders[expander] &= !mask;
            }

            self.i2c.write(BANK1, &[0x1, self.expanders[0]]).ok();
            self.i2c.write(BANK2, &[0x1, self.expanders[1]]).ok();
        }
    }
}

#[embassy_executor::task]
async fn relais_task(mut relais: Relais) {
    println!("relais_task started");
    loop {
        let cmd = RELAIS_CHANNEL.receive().await;
        println!("relais_task:{cmd:?}");
        match cmd {
            RelaisCommand::Set {
                num,
                on,
                duration_ms,
            } => {
                relais.set(num, on);
                if let Some(ms) = duration_ms {
                    embassy_time::Timer::after_millis(ms).await;
                    relais.set(num, false);
                }
            }
        }
    }
}
