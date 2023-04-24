use std::fs::File;

use csv::Writer;

const FIXED_FIELDS: &'static [&'static str] = &[
    "block_number",
    "transaction_hash",
    "transaction_from",  // tx from
    "transaction_to",    // tx to
    "transaction_value", // tx value
    "contract",
    // "transaction_log_index",
];

pub struct CsvOutput {
    pub filename: String,
    pub writer: Writer<File>,
    pub headers: Vec<String>,
}

impl CsvOutput {
    pub fn new(filename: &str) -> Self {
        let file = File::create(filename).unwrap();
        let writer = csv::Writer::from_writer(file);
        Self {
            filename: filename.to_owned(),
            // file,
            writer,
            headers: Vec::from(FIXED_FIELDS).iter().map(|&s| s.into()).collect(),
        }
    }

    pub fn add_headers(&mut self, headers: Vec<String>) -> &Self {
        self.headers.extend(headers);
        self
    }

    pub fn write_headers(&mut self) -> &Self {
        self.writer.write_record(self.headers.clone()).unwrap();
        self.writer.flush().unwrap();
        self
    }

    pub fn write(&mut self, record: Vec<String>) -> &Self {
        self.writer.write_record(record).unwrap();
        self.writer.flush().unwrap();
        self
    }
}
