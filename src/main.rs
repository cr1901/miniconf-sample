use embassy_futures::block_on;
use embedded_storage::{nor_flash::ErrorType, ReadStorage, Storage};
use embedded_storage_async::nor_flash::{NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash};
use miniconf::{Postcard, Traversal, TreeDeserialize, TreeSerialize};
use postcard::ser_flavors::Flavor;
use sequential_storage::{self, cache::NoCache};
use serde::{Deserialize, Serialize};

use core::num::NonZeroUsize;
use heapless::String;
use miniconf::{Packed, Tree, TreeKey};

#[derive(Tree, Default, Serialize, Deserialize)]
pub struct Msg(String<80>);

#[derive(Tree, Default, Serialize, Deserialize)]
pub struct Settings {
    lcd_size: [u8; 2],
    #[tree(depth = 3)]
    msgs: [Option<Msg>; 2],
}

const fn const_packed(u: usize) -> Packed {
    Packed::from_lsb(match NonZeroUsize::new(u) {
        Some(p) => p,
        None => panic!("non-zero key required"),
    })
}

pub const SIZE: Packed = const_packed(0b1_0);
pub const MSG_0: Packed = const_packed(0b1_1_0000_0);
pub const MSG_1: Packed = const_packed(0b1_1_0001_0);

#[derive(Debug)]
struct EepromEmu {
    buf0: [u8; 512],
    buf1: [u8; 512],
}

impl EepromEmu {
    fn new() -> Self {
        Self {
            buf0: [0xff; 512],
            buf1: [0xff; 512],
        }
    }
}

#[derive(Debug)]
struct NotEnoughSpace {}

impl NorFlashError for NotEnoughSpace {
    fn kind(&self) -> embedded_storage::nor_flash::NorFlashErrorKind {
        NorFlashErrorKind::Other
    }
}

impl ReadStorage for EepromEmu {
    type Error = NotEnoughSpace;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let len = bytes.len();

        if len > (self.capacity() - offset as usize) {
            return Err(NotEnoughSpace {});
        }

        if offset < 512 {
            if offset as usize + len > 512 {
                let buf0_part = (512 - offset) as usize;

                bytes[0..buf0_part].copy_from_slice(&self.buf0[(offset as usize)..]);
                bytes[buf0_part..].copy_from_slice(&self.buf1[0..(len - buf0_part)]);
            } else {
                let beg = offset as usize;
                let end = beg + len;
                bytes.copy_from_slice(&self.buf0[beg..end]);
            }
        }

        if offset >= 512 {
            let beg = offset as usize - 512;
            let end = beg + len;
            bytes.copy_from_slice(&self.buf1[beg..end]);
        }

        Ok(())
    }

    fn capacity(&self) -> usize {
        1024
    }
}

impl Storage for EepromEmu {
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let len = bytes.len();

        if len > (self.capacity() - offset as usize) {
            return Err(NotEnoughSpace {});
        }

        if offset < 512 {
            if offset as usize + len > 512 {
                let buf0_part = (512 - offset) as usize;

                self.buf0[(offset as usize)..].copy_from_slice(&bytes[0..buf0_part]);
                self.buf1[0..(len - buf0_part)].copy_from_slice(&bytes[buf0_part..]);
            } else {
                let beg = offset as usize;
                let end = beg + len;
                self.buf0[beg..end].copy_from_slice(bytes);
            }
        }

        if offset >= 512 {
            let beg = offset as usize - 512;
            let end = beg + len;
            self.buf1[beg..end].copy_from_slice(bytes);
        }

        Ok(())
    }
}

impl ErrorType for EepromEmu {
    type Error = NotEnoughSpace;
}

impl ReadNorFlash for EepromEmu {
    const READ_SIZE: usize = 1;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let ret = <Self as ReadStorage>::read(self, offset, bytes);
        println!("reading {}, {:?}, {:?}", offset, bytes, ret);
        ret
    }

    fn capacity(&self) -> usize {
        1024
    }
}

impl NorFlash for EepromEmu {
    const WRITE_SIZE: usize = 1;
    const ERASE_SIZE: usize = 16;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        println!("erasing {}, {}", from, to);

        for offs in from..to {
            <Self as Storage>::write(self, offs, &[0xff])?;
        }

        Ok(())
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        println!("writing {}, {:?}", offset, bytes);
        <Self as Storage>::write(self, offset, bytes)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SettingsKey(u8);

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
pub struct SettingsItem<'b>(&'b [u8]);

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

fn main() {
    use postcard::ser_flavors as ser;
    use postcard::Serializer;

    let mut buf = [0; 128];
    // let mut byte_ser = Serializer::from_flavor ser::Slice::new(&mut buf)
    let mut emu = EepromEmu::new();

    let mut s = Settings::default();

    assert_eq!(SIZE, Settings::packed(["lcd_size"]).unwrap().0);
    // assert_eq!(MSG_0, Settings::packed(["msgs", "0", "0"]).unwrap().0);
    // assert_eq!(MSG_1, Settings::packed(["msgs", "1", "0"]).unwrap().0);

    s.lcd_size = [20, 4];
    s.msgs[0] = Some(Msg(TryFrom::try_from("Th").unwrap()));

    let mut buf2 = [0; 128];
    for p in Settings::iter_packed() {
        let mut ser = Serializer {
            output: ser::Slice::new(&mut buf),
        };

        if let Err(miniconf::Error::Traversal(Traversal::Absent(_))) = s.serialize_by_key(p.unwrap(), &mut ser) {
            continue;
        }

        s.serialize_by_key(p.unwrap(), &mut ser).unwrap();
        let len = ser.output.finalize().unwrap().len();
        println!("{}", len);

        println!("{:#b}, {:?}", p.unwrap().into_lsb().get(), &buf[0..len]);

        block_on(sequential_storage::map::store_item(
            &mut emu,
            0..1024,
            &mut NoCache::new(),
            &mut buf2,
            SettingsKey(p.unwrap().get() as u8),
            &SettingsItem(&buf[0..len]),
        )).unwrap();

        println!("{:?}", emu);
    }

    // println!("{:?}", buf);
    // println!("{:?}", buf2);
    // println!("{:?}", emu);
}
