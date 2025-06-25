use crate::can::send_can_message;
use crate::can_id::CanId;
use crate::can_message_type::CanMessageType;
use crate::device_message::IdTypeMsg;
use crate::error::{Component, ErrorCode, ErrorReport, Severity};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
use esp_hal::efuse::Efuse;

static DEVICE: Mutex<CriticalSectionRawMutex, Option<Device>> = Mutex::new(None);

pub async fn init() {
    let mut device_guard = DEVICE.lock().await;

    if device_guard.is_none() {
        let mac = Efuse::read_base_mac_address();
        let mac = u64::from_be_bytes([0, 0, mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]]);
        let device = Device {
            custom_string: [0; 9],
            id: 0,
            dtype: 1,
            baudrate: 0,
            hardware_revision: 0,
            uid0: 0,
            uid1: 0,
            mac,
            boot_time: Instant::now(),
        };
        *device_guard = Some(device);
    }
}

pub async fn device(
) -> embassy_sync::mutex::MappedMutexGuard<'static, CriticalSectionRawMutex, Device> {
    let guard = DEVICE.lock().await;
    embassy_sync::mutex::MutexGuard::map(guard, |opt| opt.as_mut().expect("Device not initialized"))
}

pub struct Device {
    custom_string: [u8; 9],
    id: u8,
    dtype: u8,
    baudrate: u8,
    hardware_revision: u8,
    uid0: u64,
    uid1: u64,
    mac: u64,
    boot_time: Instant,
}

impl Device {
    pub async fn uptime(&mut self, id: CanId, _data: &[u8], remote_request: bool) {
        if remote_request {
            let now = Instant::now();
            let uptime = now.duration_since(self.boot_time);
            let uptime_minutes = uptime.as_secs() / 60;

            let bytes = (uptime_minutes as u32).to_le_bytes(); // 4 bytes

            send_can_message(id.msg_type, &bytes, false).await;
        }
    }

    pub async fn request_parameter(&mut self, id: CanId, data: &[u8], _remote_request: bool) {
        self.uptime(id, data, true).await;
        self.uid0(id, data, true).await;
        self.uid1(id, data, true).await;
        self.custom_string(id, data, true).await;
        self.hardware_revision(id, data, true).await;
        self.application_version(id, data, true).await;
    }
    pub async fn id_type(&mut self, id: CanId, data: &[u8], remote_request: bool) {
        if remote_request {
            if self.uid0 == self.mac && self.uid1 == self.mac {
                if data.len() == 2 {
                    self.id = data[0];
                    self.dtype = data[1];
                } else {
                    ErrorReport::send(
                        Component::Device,
                        ErrorCode::InvalidData,
                        Severity::Warning,
                        0,
                        &[id.msg_type as u8, data.len() as u8, 0u8],
                    )
                    .await;
                }
                // ignore else, thats than not for this device
            }
        } else {
            let (id, dtype) = IdTypeMsg::parse(data).unwrap();
            self.id = id;
            self.dtype = dtype;
        }
    }

    pub async fn uid0(&mut self, id: CanId, data: &[u8], remote_request: bool) {
        if remote_request {
            let txdata = self.mac.to_le_bytes();
            send_can_message(id.msg_type, &txdata, false).await;
        } else {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(data);
            self.uid0 = u64::from_le_bytes(buf);
        }
    }

    pub async fn uid1(&mut self, id: CanId, data: &[u8], remote_request: bool) {
        if remote_request {
            let txdata = self.mac.to_le_bytes();
            send_can_message(id.msg_type, &txdata, false).await;
        } else {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(data);
            self.uid0 = u64::from_le_bytes(buf);
        }
    }

    pub async fn baudrate(&mut self, id: CanId, data: &[u8], remote_request: bool) {
        if remote_request {
            let mut txdata = [0u8; 1];
            txdata[0] = self.baudrate;
            send_can_message(id.msg_type, &txdata, false).await;
        } else {
            if data.len() == 1 {
                self.baudrate = data[0];
                esp_hal::system::software_reset();
            } else {
                ErrorReport::send(
                    Component::Device,
                    ErrorCode::InvalidData,
                    Severity::Warning,
                    0,
                    &[id.msg_type as u8, data.len() as u8, 0u8],
                )
                .await;
            }
        }
    }
    pub async fn custom_string(&mut self, _id: CanId, data: &[u8], remote_request: bool) {
        if remote_request {
            // RTR-Frame: Aktuellen String senden
            let string = self.custom_string;

            let len = string.iter().position(|&c| c == 0).unwrap_or(8);
            let mut data = [0u8; 8];
            data[..len].copy_from_slice(&string[..len]);
            send_can_message(CanMessageType::CustomString, &data, false).await;
        } else {
            // Neuer String setzen
            let mut string = self.custom_string;

            let len = data.len().min(8) as usize;
            string[..len].copy_from_slice(&data[..len]);
            if len < 9 {
                string[len] = 0;
            }
        }
    }
    pub async fn hardware_revision(&mut self, id: CanId, data: &[u8], remote_request: bool) {
        if remote_request {
            let mut txdata = [0u8; 1];
            txdata[0] = self.hardware_revision;
            send_can_message(id.msg_type, &txdata, false).await;
        } else {
            if data.len() == 1 {
                self.hardware_revision = data[0];
                esp_hal::system::software_reset();
            } else {
                ErrorReport::send(
                    Component::Device,
                    ErrorCode::InvalidData,
                    Severity::Warning,
                    0,
                    &[id.msg_type as u8, data.len() as u8, 0u8],
                )
                .await;
            }
        }
    }
    pub async fn application_version(&mut self, id: CanId, _data: &[u8], _remote_request: bool) {
        let version = env!("VERGEN_GIT_DESCRIBE");
        let version_bytes = version.as_bytes();

        let mut buf = [0u8; 8];
        buf[..version_bytes.len().min(8)]
            .copy_from_slice(&version_bytes[..version_bytes.len().min(8)]);

        send_can_message(id.msg_type, &buf, false).await;
    }
    pub async fn restart(&mut self, _id: CanId, _data: &[u8], remote_request: bool) {
        if !remote_request {
            esp_hal::system::software_reset();
        }
    }
}
