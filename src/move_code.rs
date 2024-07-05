//! Define the MoveCode enumeration.
//!
//! This enumaration contains the different codes for the dome motion.

pub enum MoveCode {
    AzimuthPositive,
    AzimuthNegative,
    MainDoorClosing,
    MainDoorOpening,
    DropoutDoorClosing,
    DropoutDoorOpening,
    AzimuthHoming,
    EStop,
}

impl MoveCode {
    pub fn byte_value(&self) -> u8 {
        match self {
            MoveCode::AzimuthPositive => 0x01,
            MoveCode::AzimuthNegative => 0x02,
            MoveCode::MainDoorClosing => 0x04,
            MoveCode::MainDoorOpening => 0x08,
            MoveCode::DropoutDoorClosing => 0x10,
            MoveCode::DropoutDoorOpening => 0x20,
            MoveCode::AzimuthHoming => 0x40,
            MoveCode::EStop => 0x80,
        }
    }
}
