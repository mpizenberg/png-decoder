use std::convert::TryFrom;

#[derive(Debug, Copy, Clone)]
pub enum ColorType {
    Gray,
    RGB,
    PLTE,
    GrayAlpha,
    RGBA,
}

impl TryFrom<u8> for ColorType {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ColorType::Gray),
            2 => Ok(ColorType::RGB),
            3 => Ok(ColorType::PLTE),
            4 => Ok(ColorType::GrayAlpha),
            6 => Ok(ColorType::RGBA),
            _ => Err(format!("Color type {} is not valid", value)),
        }
    }
}

