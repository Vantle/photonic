use crate::failure::Failure;

pub fn decode(path: &str, byte: Vec<u8>) -> Result<String, Failure> {
    String::from_utf8(byte).map_err(|_| Failure::Encoding {
        path: path.to_owned(),
    })
}
