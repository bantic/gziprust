use std::env;

pub struct Config {
  pub filename: String,
  pub debug: bool,
}

impl Config {
  pub fn new(mut args: env::Args) -> Result<Config, &'static str> {
    // eat 1st arg
    args.next();

    let filename = match args.next() {
      Some(filename) => filename,
      None => return Err("Needs filename"),
    };

    let debug = args.next().is_some();

    Ok(Config { filename, debug })
  }
}
