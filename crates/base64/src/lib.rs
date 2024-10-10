use base64::{engine::general_purpose::STANDARD, Engine as _};

pub fn encode(data: &str) -> String {
    STANDARD.encode(data.as_bytes())
}

pub fn decode(data: &str) -> Result<String, base64::DecodeError> {
    let decoded = STANDARD.decode(data.as_bytes())?;
    Ok(String::from_utf8(decoded).unwrap())
}