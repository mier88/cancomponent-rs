use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::i2c::master::{Config, I2c};
use esp_hal::peripheral::Peripheral;
use esp_hal::Async;
use esp_println::println;

const BANK1: u8 = 0x26;
const BANK2: u8 = 0x27;

#[derive(Debug)]
pub enum RelaisCommand {
    Set {
        num: usize,
        on: bool,
        duration_ms: Option<u64>,
    },
}

// Channel: public so others can send to it
pub static RELAIS_CHANNEL: Channel<CriticalSectionRawMutex, RelaisCommand, 8> = Channel::new();

pub struct Relais<'a> {
    i2c: I2c<'a, Async>,
    expanders: [u8; 2],
}

impl<'a> Relais<'a> {
    pub fn new<SDA: PeripheralOutput, SCL: PeripheralOutput>(
        i2c0: esp_hal::peripherals::I2C0,
        sda: impl Peripheral<P = SDA> + 'a,
        scl: impl Peripheral<P = SCL> + 'a,
    ) -> Relais<'a> {
        let mut i2c = I2c::new(i2c0, Config::default())
            .unwrap()
            .with_sda(sda)
            .with_scl(scl)
            .into_async();

        i2c.write(BANK1, &[0x3, 0x0]).ok();
        i2c.write(BANK2, &[0x3, 0x0]).ok();
        i2c.write(BANK1, &[0x1, 0x0]).ok();
        i2c.write(BANK2, &[0x1, 0x0]).ok();

        Relais {
            expanders: [0, 0],
            i2c,
        }
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

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct RelaisMsg {
    pub number: u8,
    pub state: u8,
    pub time_lo: u8,
    pub time_hi: u8,
    pub time_ext: u8,
    pub bank: u8,
    pub reserved_lo: u8,
    pub reserved_hi: u8,
}

impl RelaisMsg {
    pub fn parse(data: &[u8]) -> Option<(usize, bool, Option<u64>)> {
        if data.len() < 2 {
            return None;
        }

        let number = data[0] as usize;
        let state = data[1] != 0;

        if data.len() >= 5 {
            let time: u64 = (data[2] as u64) | ((data[3] as u64) << 8) | ((data[4] as u64) << 16);
            Some((number, state, Some(time)))
        } else {
            Some((number, state, None))
        }
    }
}

#[embassy_executor::task]
pub async fn relais_task(mut relais: Relais<'static>) {
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
