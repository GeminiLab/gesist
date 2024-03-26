use std::ops::{Deref, DerefMut};
use std::borrow::Borrow;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::Cursor;
use crc::{Crc, CRC_8_SAE_J1850};

/// This function calculates the size of a leb128 encoded integer.
///
/// # Arguments
///
/// * `input` - A usize value representing the integer to be encoded.
///
/// # Returns
///
/// * A usize value representing the size of the leb128 encoded integer.
///
/// # Example
///
/// ```
/// use gesist::padder::leb128_size;
///
/// let size = leb128_size(300);
/// assert_eq!(size, 2);
///
/// let size = leb128_size(65536);
/// assert_eq!(size, 3);
/// ```
pub const fn leb128_size(input: usize) -> usize {
    match input.checked_ilog2() {
        None => 0,
        Some(bits_minus_one) => (bits_minus_one as usize / 7) + 1
    }
}

/// `Padder` is a structure that represents a padded block of data.
///
/// Padding here includes a prefixing leb128-encoded length field, and a suffixing checksum field.
pub struct Padder {
    leb128_size: usize,
    size: usize,
    content: Box<[u8]>,
}

/// `PadderMutGuard` is a structure that represents a mutable reference to the payload of a `Padder`.
///
/// This structure is used to ensure that the checksum of the `Padder` is recalculated
/// when the `Padder` is mutated.
pub struct PadderMutGuard<'a> {
    padder: &'a mut Padder,
}

#[derive(Clone)]
pub enum PaddingValidationError {
    NotAligned { length: usize },
    BadLengthField,
    UnexpectedPaddedLength { payload_size: usize, expected: usize, actual: usize },
    InvalidChecksum { offset: usize },
}

impl Debug for PaddingValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PaddingValidationError::NotAligned { length } => write!(f, "Length {} is not aligned", length),
            PaddingValidationError::BadLengthField => write!(f, "Bad length field"),
            PaddingValidationError::UnexpectedPaddedLength { payload_size, expected, actual } =>
                write!(f, "Unexpected padded length for payload size {}, {} expected, {} actual", payload_size, expected, actual),
            PaddingValidationError::InvalidChecksum { offset } => write!(f, "Invalid checksum at offset {}", offset),
        }
    }
}

impl Display for PaddingValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(&self, f)
    }
}

impl Error for PaddingValidationError {}

/// Constants and const functions for the `Padder` struct.
impl Padder {
    /// The alignment.
    pub const ALIGNMENT: usize = 3;

    /// This function calculates the size of the padded data block.
    ///
    /// # Arguments
    ///
    /// * `input_size` - The size of the payload.
    ///
    /// # Returns
    ///
    /// * A usize value representing the size of the padded data block.
    ///
    /// # Example
    ///
    /// ```
    /// use gesist::padder::Padder;
    ///
    /// let padded_size = Padder::padded_size(300);
    /// assert_eq!(padded_size, 303);
    ///
    /// let padded_size = Padder::padded_size(200);
    /// assert_eq!(padded_size, 204);
    /// ```
    pub const fn padded_size(input_size: usize) -> usize {
        let size_leb128 = leb128_size(input_size);
        let size_without_checksum = size_leb128 + input_size + 1;
        let size_checksum = (-(size_without_checksum as isize)).rem_euclid(Self::ALIGNMENT as isize) as usize;

        size_without_checksum + size_checksum
    }

    /// The CRC used to calculate the checksum of the payload.
    pub const CRC: Crc<u8> = Crc::<u8>::new(&CRC_8_SAE_J1850);
}
/// Accessors for the `Padder` struct.
impl Padder {
    /// Returns a slice of the payload of the `Padder`.
    fn payload(&self) -> &[u8] {
        &self.content[self.leb128_size..self.leb128_size+self.size]
    }

    /// Returns a mutable slice of the payload of the `Padder`.
    fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.content[self.leb128_size..self.leb128_size+self.size]
    }

    /// Returns a slice of the payload of the `Padder`.
    pub fn as_slice(&self) -> &[u8] {
        self.payload()
    }

    /// Returns a `PadderMutGuard` for the `Padder`.
    ///
    /// This allows for mutation of the payload of the `Padder` while ensuring that the checksum is recalculated when the `Padder` is mutated.
    pub fn as_mut(&mut self) -> PadderMutGuard {
        PadderMutGuard { padder: self }
    }

    /// Returns a slice of the entire content of the `Padder`.
    ///
    /// This includes the leb128 size, the payload, and the checksum.
    /// This method is marked as unsafe because it exposes the raw content of the `Padder`.
    pub unsafe fn raw_slice(&self) -> &[u8] {
        &self.content
    }

    /// Returns a mutable slice of the entire content of the `Padder`.
    ///
    /// This includes the leb128 size, the payload, and the checksum.
    /// This method is marked as unsafe because it exposes the raw content of the `Padder`.
    pub unsafe fn raw_mut(&mut self) -> &mut [u8] {
        &mut self.content
    }

    /// Returns the offset of the payload in the `Padder`.
    ///
    /// # Returns
    ///
    /// * A usize value representing the offset of the payload in the `Padder`.
    pub fn payload_offset(&self) -> usize {
        self.leb128_size
    }

    /// Returns the length of the payload of the `Padder`.
    ///
    /// # Returns
    ///
    /// * A usize value representing the length of the payload of the `Padder`.
    pub fn payload_length(&self) -> usize {
        self.size
    }
}

/// Other methods
impl Padder {
    /// Creates a new `Padder` with a specified size, filled with zeroes.
    ///
    /// # Arguments
    ///
    /// * `size` - The size of the payload.
    ///
    /// # Returns
    ///
    /// * A new `Padder` instance with the specified size, filled with zeroes.
    pub fn new_zeroed(size: usize) -> Self {
        let padded_size = Self::padded_size(size);

        debug_assert!(padded_size > size);
        debug_assert!(padded_size % Self::ALIGNMENT == 0);

        let mut content = vec![0; padded_size].into_boxed_slice();
        let leb128_size = leb128::write::unsigned(&mut content.as_mut(), size as u64).unwrap();

        let mut result = Self {
            leb128_size,
            size,
            content,
        };

        result.recalculate_checksum();
        result
    }

    /// Creates a new `Padder` from a byte slice.
    ///
    /// # Arguments
    ///
    /// * `input` - A byte slice to be copied into the `Padder`.
    ///
    /// # Returns
    ///
    /// * A new `Padder` instance containing a copy of the input byte slice.
    pub fn new(input: impl AsRef<[u8]>) -> Self {
        let input = input.as_ref();
        let mut result = Self::new_zeroed(input.len());
        result.as_mut().copy_from_slice(input);

        result
    }

    /// Checks if the input byte slice is a correctly padded data block.
    ///
    /// # Arguments
    ///
    /// * `raw` - A byte slice to be converted into a `Padder`.
    ///
    /// # Returns
    ///
    /// * A `Result` which is:
    ///     - `Ok` if the input byte slice is a correctly padded data block, containing the `Padder`.
    ///     - `Err` if the input byte slice is not a correctly padded data block, containing a `PadderCheckError`.
    pub fn try_from_raw(raw: impl Into<Box<[u8]>>) -> Result<Self, PaddingValidationError> {
        let content = raw.into();
        let len = content.len();

        if len % Self::ALIGNMENT != 0 {
            return Err(PaddingValidationError::NotAligned { length: len });
        }

        let mut cursor = Cursor::new(content.as_ref());
        let payload_size = leb128::read::unsigned(&mut cursor).map_err(|_| PaddingValidationError::BadLengthField)? as usize;
        let leb128_size = cursor.position() as usize;

        let expected_padded_size = Self::padded_size(payload_size);
        if expected_padded_size != len {
            return Err(PaddingValidationError::UnexpectedPaddedLength { payload_size, expected: expected_padded_size, actual: len });
        }

        let checksum_size = len - payload_size - leb128_size;
        let expected_checksum = Self::CRC.checksum(&content[leb128_size..leb128_size + payload_size]);
        for i in 0..checksum_size {
            let expected = expected_checksum.wrapping_add(i as u8);
            if content[leb128_size + payload_size + i] != expected {
                return Err(PaddingValidationError::InvalidChecksum { offset: leb128_size + payload_size + i });
            }
        }

        Ok(Self {
            leb128_size,
            size: payload_size,
            content,
        })
    }

    /// Recalculates the checksum of the `Padder`.
    ///
    /// If more than one byte should be filled with the checksum, the checksum is incremented by the index of the byte.
    ///
    /// This method is used to ensure that the checksum of the `Padder` is always correct after the `Padder` is mutated.
    pub fn recalculate_checksum(&mut self) {
        let crc = Self::CRC.checksum(self.payload());
        let checksum_count = self.content.len() - self.size - self.leb128_size;

        for i in 0..checksum_count {
            self.content[self.leb128_size + self.size + i] = crc.wrapping_add(i as u8);
        }
    }
}

/// Implementation of the `Borrow` trait for the `Padder` struct.
///
/// It's safe to implement `Borrow` because the same payloads always result in the same `Padder`.
impl Borrow<[u8]> for Padder {
    /// Returns a slice of the payload of the `Padder`.
    ///
    /// # Returns
    ///
    /// * A byte slice representing the payload of the `Padder`.
    fn borrow(&self) -> &[u8] {
        self.payload()
    }
}

/// Implementation of the `AsRef` trait for the `Padder` struct.
impl AsRef<[u8]> for Padder {
    /// Returns a slice of the payload of the `Padder`.
    ///
    /// # Returns
    ///
    /// * A byte slice representing the payload of the `Padder`.
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

/// Implementation of the `Into` trait for the `Padder` struct.
impl Into<Box<[u8]>> for Padder {
    /// Consumes the `Padder` and returns a boxed slice representing the entire content of the `Padder`.
    ///
    /// # Returns
    ///
    /// * A boxed slice representing the entire content of the `Padder`.
    fn into(self) -> Box<[u8]> {
        self.content
    }
}

/// Implementation of the `Deref` trait for the `PadderMutGuard` struct.
///
/// This allows for the `PadderMutGuard` to be used as a byte slice.
impl Deref for PadderMutGuard<'_> {
    type Target = [u8];

    /// Returns a slice of the payload of the `Padder` associated with the `PadderMutGuard`.
    ///
    /// # Returns
    ///
    /// * A byte slice representing the payload of the `Padder`.
    fn deref(&self) -> &Self::Target { self.padder.payload() }
}

/// Implementation of the `DerefMut` trait for the `PadderMutGuard` struct.
///
/// This allows for the `PadderMutGuard` to be used as a mutable byte slice.
impl DerefMut for PadderMutGuard<'_> {
    /// Returns a mutable slice of the payload of the `Padder` associated with the `PadderMutGuard`.
    ///
    /// # Returns
    ///
    /// * A mutable byte slice representing the payload of the `Padder`.
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.padder.payload_mut()
    }
}

/// Implementation of the `Drop` trait for the `PadderMutGuard` struct.
///
/// This ensures that the checksum of the `Padder` associated with the `PadderMutGuard` is
/// recalculated when the `PadderMutGuard` is dropped.
impl Drop for PadderMutGuard<'_> {
    /// When the `PadderMutGuard` is dropped, recalculates the checksum of the `Padder` associated with the `PadderMutGuard`.
    fn drop(&mut self) {
        self.padder.recalculate_checksum();
    }
}
