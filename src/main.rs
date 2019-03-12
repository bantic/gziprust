use gziprust;
use gziprust::{run, Config};
use std::{env, process};

fn main() {
  let config = Config::new(env::args()).unwrap_or_else(|err| {
    eprintln!("Error reading args: {}", err);
    process::exit(1);
  });

  if let Err(e) = run(config) {
    eprint!("Error: {}", e);
    process::exit(1);
  }
}
