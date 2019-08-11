use png_decoder::png;
use std::{env, error::Error, fs};

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Err(error) = run(&args) {
        eprintln!("{:?}", error);
    }
}

fn run(args: &[String]) -> Result<(), Box<Error>> {
    let data = fs::read(&args[1])?;
    let _ = png::decode_no_check_verbose_bis(&data)?;
    // let _ = png::decode_no_check_verbose(&data)?;
    // let _ = png::decode_verbose(&data)?;
    println!("All done!");
    Ok(())
}
