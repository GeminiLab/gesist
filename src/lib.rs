use std::io::{self, Write};
use base64::Engine;

use mixer::Mixer;
use padder::{Padder, PaddingValidationError};

pub mod padder;
pub mod mixer;

fn do_encode(input: impl AsRef<[u8]>) -> Option<Mixer> {
    let input = input.as_ref();
    if input.is_empty() {
        return None;
    }

    let mut mix = Mixer::new_from_padder(Padder::new(input));
    mix.mix();

    Some(mix)
}

pub fn encode<T: AsRef<[u8]>>(input: T) -> Box<[u8]> {
    do_encode(input).map_or_else(|| [].into(), |m| m.into())
}

pub fn encode_to<T: AsRef<[u8]>, D: Write>(input: T, mut dest: D) -> io::Result<()> {
    do_encode(input).map_or_else(|| Ok(()), |m| dest.write_all(m.as_slice()))
}

pub fn encode_to_base64<T: AsRef<[u8]>>(input: T) -> String {
    do_encode(input).map_or_else(|| String::new(), |m| base64::prelude::BASE64_URL_SAFE.encode(m))
}

fn do_decode(place: impl Into<Box<[u8]>>) -> Result<Padder, PaddingValidationError> {
    let place = place.into();
    let len = place.len();
    let mut mix = Mixer::new(place).ok_or(PaddingValidationError::NotAligned { length: len })?;
    mix.mix();

    Padder::try_from_raw(mix)
}

pub fn decode(input: impl Into<Box<[u8]>>) -> Result<Box<[u8]>, PaddingValidationError> {
    do_decode(input.into()).map(|p| p.as_ref().into())
}

pub fn decode_to(input: impl Into<Box<[u8]>>, mut dest: impl Write) -> Result<io::Result<()>, PaddingValidationError> {
    do_decode(input.into()).map(|p| dest.write_all(p.as_slice()))
}

pub fn decode_from_base64(input: impl AsRef<[u8]>) -> Result<Result<Box<[u8]>, PaddingValidationError>, base64::DecodeError> {
    let bin = base64::prelude::BASE64_URL_SAFE.decode(input)?;
    Ok(decode(bin))
}

pub struct InPlaceDecodeResult {
    pub content: Box<[u8]>,
    pub offset: usize,
    pub length: usize,
}

pub fn decode_in_place(input: Box<[u8]>) -> Result<InPlaceDecodeResult, PaddingValidationError> {
    do_decode(input).map(|p| {
        let offset = p.payload_offset();
        let length = p.payload_length();
        let content = p.into();

        InPlaceDecodeResult {
            content,
            length,
            offset,
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::encode_to_base64;

    #[test]
    fn basic_base64_encode() {
        assert_eq!("PaEU", encode_to_base64(b"a"));
        assert_eq!("pJxd", encode_to_base64(b"b"));
    }
}
