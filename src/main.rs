use embassy_futures::block_on;
use miniconf::{Traversal, TreeKey, TreeSerialize};
use postcard::ser_flavors::Flavor;
use sequential_storage::{self, cache::NoCache, mock_flash::{MockFlashBase, WriteCountCheck}};

mod eeprom;
mod settings;

use eeprom::*;
use settings::*;

fn main() {
    use postcard::ser_flavors as ser;
    use postcard::Serializer;

    let mut buf = [0; 128];
    // let mut byte_ser = Serializer::from_flavor ser::Slice::new(&mut buf)
    // let mut emu = EepromEmu::new();
    let mut emu = MockFlashBase::<64, 1, 16>::new(WriteCountCheck::Disabled, None, false);

    let mut s = Settings::default();

    // assert_eq!(SIZE, Settings::packed(["lcd_size"]).unwrap().0);
    // assert_eq!(MSG_0, Settings::packed(["msgs", "0", "0"]).unwrap().0);
    // assert_eq!(MSG_1, Settings::packed(["msgs", "1", "0"]).unwrap().0);

    s.lcd_size = [20, 4];
    s.msgs[0] = Some(Msg(TryFrom::try_from("This is a test message").unwrap()));
    s.msgs[1] = Some(Msg(TryFrom::try_from("Here is another test message").unwrap()));

    let mut buf2 = [0; 128];
    for p in Settings::iter_packed() {
        let mut ser = Serializer {
            output: ser::Slice::new(&mut buf),
        };

        if let Err(miniconf::Error::Traversal(Traversal::Absent(_))) = s.serialize_by_key(p.unwrap(), &mut ser) {
            continue;
        }

        let len = ser.output.finalize().unwrap().len();
        println!("{:#b}, {:?}", p.unwrap().into_lsb().get(), &buf[0..len]);

        block_on(sequential_storage::map::store_item(
            &mut emu,
            0..1024,
            &mut NoCache::new(),
            &mut buf2,
            SettingsKey(p.unwrap().get() as u8),
            &SettingsItem(&buf[0..len]),
        )).unwrap();

        // println!("{:?}", emu);
    }

    // println!("{:?}", buf);
    // println!("{:?}", buf2);
    // println!("{:?}", emu);
}
