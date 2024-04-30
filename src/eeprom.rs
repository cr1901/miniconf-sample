use embedded_storage::{nor_flash::ErrorType, ReadStorage, Storage};
use embedded_storage_async::nor_flash::{NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash};

/// Fake 24x EEPROM emulation. It's two buffers b/c my actual use case is
/// two 512-byte devices.
#[derive(Debug)]
pub struct EepromEmu {
    buf0: [u8; 512],
    buf1: [u8; 512],
}

impl EepromEmu {
    pub fn new() -> Self {
        Self {
            buf0: [0xff; 512],
            buf1: [0xff; 512],
        }
    }
}

#[derive(Debug)]
pub struct NotEnoughSpace {}

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

/// Fake emulation of NOR flash for an EEPROM
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
    // We can do up to page writes, but ERASE_SIZE must be at least 3 times larger.
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
