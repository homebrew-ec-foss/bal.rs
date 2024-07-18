use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use csv::ReaderBuilder;

pub fn get_uris(file_path: String) -> Result<Vec<hyper::Uri>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut uris: Vec<hyper::Uri> = vec![];

    let mut csv_reader = ReaderBuilder::new().from_reader(reader);

    for result in csv_reader.records() {
        let record = result?;
        let uri = record[0].parse()?;
        uris.push(uri);
    }

    Ok(uris)
}