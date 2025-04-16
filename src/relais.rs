use esp_hal::i2c::master::{ I2c, Config };
use esp_hal::Async;
use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::peripheral::Peripheral;

const BANK1: u8 = 0x26;
const BANK2: u8 = 0x27;

pub struct Relais<'a> {
    i2c: I2c<'a, Async>,
    expanders: [u8; 2],
}

impl<'a> Relais<'a> {
   pub fn new <SDA: PeripheralOutput, SCL: PeripheralOutput> (i2c0: esp_hal::peripherals::I2C0, sda: impl Peripheral<P = SDA> + 'a, scl: impl Peripheral<P = SCL> + 'a,) -> Relais<'a> {
        
        let mut i2c = I2c::new(
            i2c0,
            Config::default(),
        )
        .unwrap()
        .with_sda(sda)
        .with_scl(scl);

        i2c.write(BANK1, &[0x3,0x0]).ok();
        i2c.write(BANK2, &[0x3,0x0]).ok();
        i2c.write(BANK1, &[0x1,0x0]).ok();
        i2c.write(BANK2, &[0x1,0x0]).ok();

        Relais{expanders:[0,0],
        i2c}
    }
    /// Each entry: (expander index, bit position)
    const MAPPING: [(usize, u8); 12] = [
        (0, 3), (0, 2), (0, 1), (0, 7),
        (0, 6), (0, 5), (0, 4), (1, 11 - 8),
        (1, 10 - 8), (1, 9 - 8), (1, 15 - 8), (1, 14 - 8),
    ];

    pub fn set(&mut self, num: usize, onoff: bool) {
        if let Some(&(expander, bit)) = Self::MAPPING.get(num) {
            let mask = 1 << bit;
            if onoff {
                self.expanders[expander] |= mask;
            } else {
                self.expanders[expander] &= !mask;
            }

            self.i2c.write(BANK1, &[0x1,self.expanders[0]]).ok();
            self.i2c.write(BANK2, &[0x1,self.expanders[1]]).ok();
        }
    }
}
