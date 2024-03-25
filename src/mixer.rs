use std::borrow::Borrow;

use super::padder::Padder;

macro_rules! mix_rule_inner {
    ($content:expr,$range:expr,$var_index:ident,$var_this:ident,$var_other:ident,$delta:expr,$body:block) => {
        for $var_index in $range {
            let $var_other = $content[(($var_index as isize) + $delta) as usize];
            let $var_this = &mut $content[$var_index];
            $body
        }
    };
    (guarded,$guard:expr,$content:expr,$range:expr,$var_index:ident,$var_this:ident,$var_other:ident,$delta:expr,$body:block) => {
        if $content.len() > $guard {
            mix_rule_inner!($content,$range,$var_index,$var_this,$var_other,$delta,$body)
        }
    };
}

macro_rules! mix_rule {
    ($content:expr,h2t,$var_index:ident,$var_this:ident,$var_prev:ident,$body:block) => {
        mix_rule_inner!($content,1..($content.len()),$var_index,$var_this,$var_prev,-1,$body)
    };
    ($content:expr,h2tr,$var_index:ident,$var_this:ident,$var_prev:ident,$body:block) => {
        mix_rule_inner!($content,(1..($content.len())).rev(),$var_index,$var_this,$var_prev,-1,$body)
    };
    ($content:expr,t2h,$var_index:ident,$var_this:ident,$var_next:ident,$body:block) => {
        mix_rule_inner!($content,(0..($content.len()-1)).rev(),$var_index,$var_this,$var_next,1,$body)
    };
    ($content:expr,t2hr,$var_index:ident,$var_this:ident,$var_next:ident,$body:block) => {
        mix_rule_inner!($content,0..($content.len()-1),$var_index,$var_this,$var_next,1,$body)
    };
    ($content:expr,u2d,$width:expr,$var_index:ident,$var_this:ident,$var_prev:ident,$body:block) => {
        mix_rule_inner!(guarded,$width,$content,$width..($content.len()),$var_index,$var_this,$var_prev,-$width,$body)
    };
    ($content:expr,u2dr,$width:expr,$var_index:ident,$var_this:ident,$var_prev:ident,$body:block) => {
        mix_rule_inner!(guarded,$width,$content,($width..($content.len())).rev(),$var_index,$var_this,$var_prev,-$width,$body)
    };
    ($content:expr,d2u,$width:expr,$var_index:ident,$var_this:ident,$var_next:ident,$body:block) => {
        mix_rule_inner!(guarded,$width,$content,(0..($content.len()-$width)).rev(),$var_index,$var_this,$var_next,$width,$body)
    };
    ($content:expr,d2ur,$width:expr,$var_index:ident,$var_this:ident,$var_next:ident,$body:block) => {
        mix_rule_inner!(guarded,$width,$content,0..($content.len()-$width),$var_index,$var_this,$var_next,$width,$body)
    };
    ($content:expr,byte,$var_index:ident,$var_this:ident,$body:block) => {
        for $var_index in 0..($content.len()) {
            let $var_this = &mut $content[$var_index];
            $body
        }
    };
    ($content:expr,block,$width:expr,$var_index:ident,$var_block:ident,$body:expr) => {
        for $var_index in (0..($content.len())).step_by($width) {
            let $var_block: &mut [u8; $width] = (&mut $content[$var_index..$var_index+$width]).try_into().unwrap();
            $body
        }
    };
}

/// The `Mixer` struct represents a mixer that operates on an owned byte slice.
pub struct Mixer {
    content: Box<[u8]>,
}

impl Mixer {
    /// Creates a new `Mixer` from a byte slice, copying the input data.
    ///
    /// # Arguments
    ///
    /// * `input` - A byte slice to be copied into the `Mixer`.
    ///
    /// # Returns
    ///
    /// * An `Option` containing a new `Mixer` if the length of the input data is a multiple of `Padder::ALIGNMENT`.
    /// * `None` if the length of the input data is not a multiple of `Padder::ALIGNMENT`.
    pub fn new_with_copy(input: impl AsRef<[u8]>) -> Option<Self> {
        let input = input.as_ref();

        if input.len() % Padder::ALIGNMENT != 0 {
            None
        } else {
            Self::new(input.as_ref().to_vec())
        }
    }

    /// Creates a new `Mixer` from a boxed slice of bytes.
    ///
    /// # Arguments
    ///
    /// * `input` - A boxed slice of bytes to be used as the content of the `Mixer`.
    ///
    /// # Returns
    ///
    /// * An `Option` containing a new `Mixer` if the length of the input data is a multiple of `Padder::ALIGNMENT`.
    /// * `None` if the length of the input data is not a multiple of `Padder::ALIGNMENT`.
    pub fn new(input: impl Into<Box<[u8]>>) -> Option<Self> {
        let input = input.into();

        if input.len() % Padder::ALIGNMENT != 0 {
            None
        } else {
            Some(Self { content: input, })
        }
    }

    /// Creates a new `Mixer` from a `Padder`.
    ///
    /// # Arguments
    ///
    /// * `padder` - A `Padder` to be converted into a `Mixer`.
    ///
    /// # Returns
    ///
    /// * A new `Mixer` containing the content of the `Padder`.
    pub fn new_from_padder(padder: Padder) -> Self {
        Self::new(padder).unwrap()
    }

    /// Performs a left rotation on a 3-byte block by a specified number of bits.
    ///
    /// # Arguments
    ///
    /// * `content` - A mutable reference to a 3-byte block to be rotated.
    /// * `shift` - The number of bits to rotate the block to the left.
    fn block_be_rotl(content: &mut [u8; 3], shift: usize) {
        let remain = 8 - shift;
        let carry = content[0] >> remain;
        content[0] = (content[0] << shift) | (content[1] >> remain);
        content[1] = (content[1] << shift) | (content[2] >> remain);
        content[2] = (content[2] << shift) | carry;
    }

    /// Performs a right rotation on a 3-byte block by a specified number of bits.
    ///
    /// # Arguments
    ///
    /// * `content` - A mutable reference to a 3-byte block to be rotated.
    /// * `shift` - The number of bits to rotate the block to the right.
    fn block_be_rotr(content: &mut [u8; 3], shift: usize) {
        let remain = 8 - shift;
        let carry = content[2] & ((1 << shift) - 1);
        content[2] = (content[2] >> shift) | (content[1] << remain);
        content[1] = (content[1] >> shift) | (content[0] << remain);
        content[0] = (content[0] >> shift) | (carry << remain);
    }

    /// Reverses the bits of the middle byte in a 3-byte block and swaps the first and last bytes.
    ///
    /// # Arguments
    ///
    /// * `content` - A mutable reference to a 3-byte block to be modified.
    fn middle_shift(content: &mut [u8; 3]) {
        content[1] = content[1].reverse_bits();

        let temp = content[0];
        content[0] = content[2];
        content[2] = temp;
    }

    /// Applies a series of mixing rules to the content of the `Mixer`.
    ///
    /// The rules include various operations such as prefix sums, xors, shifts, and additions and subtractions of indices.
    /// The rules are applied in a specific order such that another call to `mix` will reverse the effects of the first call.
    pub fn mix(&mut self) {
        // step   1: head-to-tail prefix sum
        // step   2: 3-width up-to-down prefix xor
        // step   3: add index to each byte
        // shift  1: shl by (row_id_0_based * 2 + 1) % 8 bits in 3-byte big-endian blocks
        // step   4: 6-width down-to-up prefix sum
        // step   5: tail-to-head prefix xor
        // shift  2:
        // step  5r: tail-to-head inverse prefix xor
        // step  4r: 6-width down-to-up inverse prefix sum
        // shift 1r: shr by (row_id_0_based * 2 + 1) % 8 bits in 3-byte big-endian blocks
        // step  3r: subtract index from each byte
        // step  2r: 3-width up-to-down inverse prefix xor
        // step  1r: head-to-tail inverse prefix sum

        mix_rule!(self.content,     h2t, i, this, prev, { *this = this.wrapping_add(prev); });
        mix_rule!(self.content,     u2d, 3, i, this, prev, { *this ^= prev; });
        mix_rule!(self.content,    byte, i, this, { *this = this.wrapping_add(i as u8); });
        mix_rule!(self.content,   block, 3, i, this, { Self::block_be_rotl(this, (i / 3 * 2 + 1) & 0x7); });
        mix_rule!(self.content,     d2u, 6, i, this, next, { *this = this.wrapping_add(next); });
        mix_rule!(self.content,     t2h, i, this, next, { *this ^= next; });
        mix_rule!(self.content,   block, 3, i, this, { Self::middle_shift(this); });
        mix_rule!(self.content,    t2hr, i, this, prev, { *this ^= prev; });
        mix_rule!(self.content,    d2ur, 6, i, this, prev, { *this = this.wrapping_sub(prev); });
        mix_rule!(self.content,   block, 3, i, this, { Self::block_be_rotr(this, (i / 3 * 2 + 1) & 0x7); });
        mix_rule!(self.content,    byte, i, this, { *this = this.wrapping_sub(i as u8); });
        mix_rule!(self.content,    u2dr, 3, i, this, prev, { *this ^= prev; });
        mix_rule!(self.content,    h2tr, i, this, prev, { *this = this.wrapping_sub(prev); });
    }

    /// Returns a slice of the content of the `Mixer`.
    ///
    /// # Returns
    ///
    /// * A byte slice representing the content of the `Mixer`.
    pub fn as_slice(&self) -> &[u8] {
        &self.content
    }

    /// Returns a mutable slice of the content of the `Mixer`.
    ///
    /// # Returns
    ///
    /// * A mutable byte slice representing the content of the `Mixer`.
    ///
    /// # Safety
    ///
    /// This method is marked as unsafe because it allows mutable access to the content of the `Mixer`.
    pub unsafe fn as_mut(&mut self) -> &mut [u8] {
        &mut self.content
    }
}

/// Implementation of the `Borrow` trait for the `Mixer` struct.
impl Borrow<[u8]> for Mixer {
    /// Returns a slice of the content of the `Mixer`.
    ///
    /// # Returns
    ///
    /// * A byte slice representing the content of the `Mixer`.
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

/// Implementation of the `AsRef` trait for the `Mixer` struct.
impl AsRef<[u8]> for Mixer {
    /// Returns a slice of the content of the `Mixer`.
    ///
    /// # Returns
    ///
    /// * A byte slice representing the content of the `Mixer`.
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

/// Implementation of the `Into` trait for the `Mixer` struct.
impl Into<Box<[u8]>> for Mixer {
    /// Consumes the `Mixer` and returns the content as a boxed slice.
    ///
    /// # Returns
    ///
    /// * A boxed slice representing the entire content of the `Mixer`.
    fn into(self) -> Box<[u8]> {
        self.content
    }
}