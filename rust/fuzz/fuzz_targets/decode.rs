#![no_main]

use cast_codec::CastFile;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    if let Ok(decoded) = CastFile::decode(bytes) {
        let encoded = decoded.encode().expect("decoded Cast data must remain encodable");
        CastFile::decode(&encoded).expect("encoded Cast data must remain decodable");
    }
});
