use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use csv::ReaderBuilder;

pub fn get_ports(file_path: String) -> Result<Vec<u16>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut ports = vec![];

    let mut csv_reader = ReaderBuilder::new().from_reader(reader);

    for result in csv_reader.records() {
        let record = result?;
        let port = record[0].parse()?;
        ports.push(port);
    }

    return Ok(ports);
}