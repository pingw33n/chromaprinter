use byteorder::{LE, ReadBytesExt};
use std::io::{Cursor, ErrorKind};

pub fn read_audio_raw(bytes: &[u8]) -> Vec<i16> {
    assert_eq!(bytes.len() % 2, 0);
    let mut r = Vec::with_capacity(bytes.len() / 2);
    let mut rd = Cursor::new(bytes);
    loop {
        match rd.read_i16::<LE>() {
            Ok(v) => r.push(v),
            Err(e) => if e.kind() == ErrorKind::UnexpectedEof {
                break;
            } else {
                panic!("{:?}", e);
            }
        }
    }
    r
}