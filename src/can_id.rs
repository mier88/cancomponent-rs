use crate::can_message_type::CanMessageType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanId {
    pub is_ng: bool,              // 1 Bit
    pub group: u8,                // 6 Bit
    pub device_type: u8,          // 6 Bit
    pub device_id: u8,            // 8 Bit
    pub msg_type: CanMessageType, // 8 Bit
}

impl CanId {
    pub fn new(device_type: u8, device_id: u8, msg_type: CanMessageType) -> Self {
        Self {
            is_ng: true,
            group: 0,
            device_type: device_type & 0x3F, // nur 6 Bit
            device_id,
            msg_type,
        }
    }
}

impl From<CanId> for u32 {
    fn from(id: CanId) -> Self {
        ((id.is_ng as u32) << 28)
            | ((id.group as u32 & 0x3F) << 22)
            | ((id.device_type as u32 & 0x3F) << 16)
            | ((id.device_id as u32) << 8)
            | id.msg_type as u32 & 0xFF
    }
}

impl From<CanId> for esp_hal::twai::ExtendedId {
    fn from(id: CanId) -> Self {
        esp_hal::twai::ExtendedId::new(id.into()).unwrap()
    }
}

impl TryFrom<u32> for CanId {
    type Error = ();

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        let msg_type = CanMessageType::try_from((raw & 0xFF) as u8)?;
        Ok(Self {
            is_ng: ((raw >> 28) & 0x1) != 0,
            group: ((raw >> 22) & 0x3F) as u8,
            device_type: ((raw >> 16) & 0x3F) as u8,
            device_id: ((raw >> 8) & 0xFF) as u8,
            msg_type,
        })
    }
}

impl TryFrom<embedded_can::ExtendedId> for CanId {
    type Error = ();

    fn try_from(id: embedded_can::ExtendedId) -> Result<Self, Self::Error> {
        Self::try_from(id.as_raw())
    }
}

impl core::fmt::Display for CanId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "NG:{} Group:{} Type:{} ID:{} Msg:{:?}",
            self.is_ng, self.group, self.device_type, self.device_id, self.msg_type
        )
    }
}
