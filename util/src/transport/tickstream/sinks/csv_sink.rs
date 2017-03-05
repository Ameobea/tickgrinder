//! Saves data to a CSV flatfile.

// TODO: Enable compression/decompression and transferring to/from archives

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use csv::Writer;

use trading::tick::GenTick;
use transport::tickstream::GenTickSink;
use transport::textlog::debug;
use rustc_serialize::{Encodable, Decodable};

// TODO: Non `CommandServer`-Based Logging

pub struct CsvSink<> {
    writer: Writer<File>,
}

/// A tick sink that writes data to a CSV file.  As long as the data is able to be split up into columns and serialized, this sink should be able to handle it.
/// Requires that the setting `output_path` be supplied in the settings `HashMap`.
impl<T> GenTickSink<T> for CsvSink where T:Encodable, T:Decodable {
    fn new(settings: HashMap<String, String>) -> Result<Self, String> {
        let output_path = match settings.get("output_path") {
            Some(p) => p,
            None => { return Err(String::from("You must supply an `output_path` argument in the input `HashMap`~")) },
        };

        Ok(CsvSink {
            writer: Writer::from_file(Path::new(output_path)).map_err(debug)?,
        })
    }

    fn tick(&mut self, t: GenTick<T>) {
        if let Err(e) = self.writer.encode((t.timestamp, t.data)) {
            println!("Error while writing line to file: {:?}", e);
        }
    }
}
