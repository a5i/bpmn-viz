//! Fuzzes the BPMN importer with arbitrary bytes.
//! Run: cargo +nightly fuzz run xml_parse -- -max_len=65536

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Must never panic; parse results (Ok or Err) are both fine.
    let _ = bpmn_xml::from_xml(data);
});
