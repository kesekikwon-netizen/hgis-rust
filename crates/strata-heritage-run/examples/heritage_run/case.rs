//! The cases this example drives. `KaHeritageBrowser.cpp:596-940`.
//!
//! The fixture always accepts the same pair; a case that must fail logs in with a different
//! password. Passing the wrong password to *both* the fixture and the run makes the fixture
//! accept it, which is how the first version of this example tested nothing.

use strata_net::heritage_auto::HeritageStage;

/// The account the fixture accepts, in every case.
pub const FIXTURE_ACCOUNT: (&str, &str) = ("user1", "pass1");

/// What a case expects.
pub struct Case {
    pub name: &'static str,
    /// The id the run logs in with.
    pub run_id: &'static str,
    /// The password the run logs in with.
    pub run_password: &'static str,
    /// The city both the fixture and the run use.
    pub city: (&'static str, &'static str),
    /// Where the run must be when the case is over.
    pub expect: HeritageStage,
}

/// The cases, in the order they run.
#[must_use]
pub fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "right password reaches the download page",
            run_id: FIXTURE_ACCOUNT.0,
            run_password: FIXTURE_ACCOUNT.1,
            city: ("경상북도", "경주시"),
            expect: HeritageStage::DownloadAll,
        },
        Case {
            name: "wrong password stops and says so",
            run_id: FIXTURE_ACCOUNT.0,
            run_password: "not-the-password",
            city: ("경상북도", "경주시"),
            expect: HeritageStage::Stopped(String::new()),
        },
        Case {
            name: "a kind with a ZIP runs to the end of the six kinds",
            run_id: FIXTURE_ACCOUNT.0,
            run_password: FIXTURE_ACCOUNT.1,
            city: ("경상북도", "경주시"),
            // The fixture answers every kind with the same ZIP, so all six arrive and the run
            // finishes. This is the only case that proves a file reaches the disk.
            expect: HeritageStage::Done,
        },
    ]
}

/// Whether the case needs the fixture to answer with a ZIP.
#[must_use]
pub fn wants_zip(case: &Case) -> bool {
    matches!(case.expect, HeritageStage::Done)
}

/// A minimal but real ZIP: one entry, stored (not deflated), so `looks_like_zip` accepts it and
/// nothing has to be compressed for the test.
///
/// The site answers with a ZIP of shapefiles; the run only needs the file to be a ZIP here,
/// because what the app does with the contents is `heritage_reuse`'s job (HER9).
pub fn sample_zip() -> Result<Vec<u8>, String> {
    // One stored entry named `a.txt` holding `hi`. Every field is hand-written so the bytes are
    // a ZIP a reader accepts, without pulling a compression crate in for the example.
    let name = b"a.txt";
    let data = b"hi";
    let crc = crc32(data);
    let mut out = Vec::new();
    // Local file header.
    out.extend_from_slice(b"PK\x03\x04");
    out.extend_from_slice(&20u16.to_le_bytes()); // version needed
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&0u16.to_le_bytes()); // method: stored
    out.extend_from_slice(&0u16.to_le_bytes()); // mod time
    out.extend_from_slice(&0u16.to_le_bytes()); // mod date
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // extra length
    out.extend_from_slice(name);
    out.extend_from_slice(data);
    let central_at = out.len();
    // Central directory header.
    out.extend_from_slice(b"PK\x01\x02");
    out.extend_from_slice(&20u16.to_le_bytes()); // version made by
    out.extend_from_slice(&20u16.to_le_bytes()); // version needed
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&0u16.to_le_bytes()); // method
    out.extend_from_slice(&0u16.to_le_bytes()); // mod time
    out.extend_from_slice(&0u16.to_le_bytes()); // mod date
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // extra
    out.extend_from_slice(&0u16.to_le_bytes()); // comment
    out.extend_from_slice(&0u16.to_le_bytes()); // disk
    out.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
    out.extend_from_slice(&0u32.to_le_bytes()); // external attrs
    out.extend_from_slice(&0u32.to_le_bytes()); // local header offset
    out.extend_from_slice(name);
    let central_len = out.len() - central_at;
    // End of central directory.
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&0u16.to_le_bytes()); // this disk
    out.extend_from_slice(&0u16.to_le_bytes()); // disk with central
    out.extend_from_slice(&1u16.to_le_bytes()); // entries on this disk
    out.extend_from_slice(&1u16.to_le_bytes()); // entries total
    out.extend_from_slice(&(central_len as u32).to_le_bytes());
    out.extend_from_slice(&(central_at as u32).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment length
    Ok(out)
}

/// CRC-32 (the ZIP polynomial), as the format's specification defines it.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Whether `got` satisfies `expect`, treating any stop as equal to a stop.
#[must_use]
pub fn matches(got: &HeritageStage, expect: &HeritageStage) -> bool {
    match (got, expect) {
        (HeritageStage::Stopped(_), HeritageStage::Stopped(_)) => true,
        (a, b) => a == b,
    }
}
