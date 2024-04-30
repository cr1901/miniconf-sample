use sequential_storage;
use serde::{Deserialize, Serialize};

use core::num::NonZeroUsize;
use heapless::String;
use miniconf::{Packed, Tree};

#[derive(Tree, Default, Serialize, Deserialize)]
pub struct Msg(pub String<80>);

#[derive(Tree, Default, Serialize, Deserialize)]
pub struct Settings {
    pub lcd_size: [u8; 2],
    #[tree(depth = 3)]
    pub msgs: [Option<Msg>; 12],
}

const fn const_packed(u: usize) -> Packed {
    Packed::from_lsb(match NonZeroUsize::new(u) {
        Some(p) => p,
        None => panic!("non-zero key required"),
    })
}

#[allow(unused)]
pub const SIZE: Packed = const_packed(0b1_0);
#[allow(unused)]
pub const MSG_0: Packed = const_packed(0b1_1_0000_0);
#[allow(unused)]
pub const MSG_1: Packed = const_packed(0b1_1_0001_0);

// Taken straight from: https://github.com/quartiq/stabilizer/blob/fix/861/menu-clear/src/settings.rs
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsKey(pub u8);

impl sequential_storage::map::Key for SettingsKey {
    fn serialize_into(
        &self,
        buffer: &mut [u8],
    ) -> Result<usize, sequential_storage::map::SerializationError> {
        Ok(postcard::to_slice(self, buffer)
            .map_err(|_| sequential_storage::map::SerializationError::BufferTooSmall)?
            .len())
    }

    fn deserialize_from(
        buffer: &[u8],
    ) -> Result<(Self, usize), sequential_storage::map::SerializationError> {
        let original_length = buffer.len();
        let (result, remainder) = postcard::take_from_bytes(buffer)
            .map_err(|_| sequential_storage::map::SerializationError::BufferTooSmall)?;
        Ok((result, original_length - remainder.len()))
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct SettingsItem<'b>(pub &'b [u8]);

impl<'a, 'b> sequential_storage::map::Value<'a> for SettingsItem<'b>
where
    'a: 'b,
{
    fn serialize_into(
        &self,
        buffer: &mut [u8],
    ) -> Result<usize, sequential_storage::map::SerializationError> {
        Ok(postcard::to_slice(self, buffer)
            .map_err(|_| sequential_storage::map::SerializationError::BufferTooSmall)?
            .len())
    }

    fn deserialize_from(
        buffer: &'a [u8],
    ) -> Result<Self, sequential_storage::map::SerializationError> {
        postcard::from_bytes(buffer)
            .map_err(|_| sequential_storage::map::SerializationError::BufferTooSmall)
    }
}
