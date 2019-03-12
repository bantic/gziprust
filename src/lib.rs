use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

pub mod gzip;

pub struct Config {
  pub filename: String,
}

impl Config {
  pub fn new(mut args: env::Args) -> Result<Config, &'static str> {
    args.next();
    match args.next() {
      Some(filename) => Ok(Config { filename }),
      None => Err("Needs filename"),
    }
  }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
  let mut buf = vec![];
  let mut file = File::open(&config.filename)?;
  let num_read = file.read_to_end(&mut buf)?;
  println!("Read {} from {}:", num_read, &config.filename);
  for b in &buf {
    println!("{}", b);
  }
  Ok(())
}
