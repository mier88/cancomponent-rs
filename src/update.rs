use crate::can_id::CanId;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal_ota::Ota;
use esp_storage::FlashStorage;

static UPDATE: Mutex<CriticalSectionRawMutex, Option<Update>> = Mutex::new(None);

pub async fn init() {
    let mut update_guard = UPDATE.lock().await;

    if update_guard.is_none() {
        let update = Update { ota: None };
        *update_guard = Some(update);
    }
}

pub async fn update(
) -> embassy_sync::mutex::MappedMutexGuard<'static, CriticalSectionRawMutex, Update> {
    let guard = UPDATE.lock().await;
    embassy_sync::mutex::MutexGuard::map(guard, |opt| opt.as_mut().expect("Update not initialized"))
}

pub struct Update {
    ota: Option<Ota<FlashStorage>>,
}

impl Update {
    pub async fn start(&mut self, _id: CanId, data: &[u8], _remote_request: bool) {
        if data.len() < 8 {
            // Ungültige Nachricht
            return;
        }

        let size = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let crc = u32::from_le_bytes(data[4..8].try_into().unwrap());

        match Ota::new(FlashStorage::new()) {
            Ok(mut ota) => {
                if ota.ota_begin(size, crc).is_ok() {
                    self.ota = Some(ota);
                } else {
                    // ota_begin fehlgeschlagen
                    self.ota = None;
                }
            }
            Err(_) => {
                // OTA-Initialisierung fehlgeschlagen
                self.ota = None;
            }
        }
    }

    pub async fn write(&mut self, _id: CanId, data: &[u8], _remote_request: bool) {
        if let Some(ota) = self.ota.as_mut() {
            match ota.ota_write_chunk(data) {
                Ok(true) => {
                    // Letzter Chunk – flush und reboot
                    if ota.ota_flush(true, true).is_ok() {
                        esp_hal::system::software_reset();
                    }
                }
                Ok(false) => {
                    // Weiter schreiben
                }
                Err(_) => {
                    // Fehler beim Schreiben
                    self.ota = None;
                }
            }
        } else {
            // OTA nicht gestartet
        }
    }

    pub async fn progress(&mut self, _id: CanId, _data: &[u8], _remote_request: bool) {}
    pub async fn select(&mut self, _id: CanId, _data: &[u8], _remote_request: bool) {}
    pub async fn erase(&mut self, _id: CanId, _data: &[u8], _remote_request: bool) {}
    pub async fn read(&mut self, _id: CanId, _data: &[u8], _remote_request: bool) {}
    pub async fn verify(&mut self, _id: CanId, _data: &[u8], _remote_request: bool) {}
}
