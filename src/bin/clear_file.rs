//! Used to test file I/O functions in Rust.

use std::io::Write;
use std::{fs, io};

fn main() -> io::Result<()> {
    let mut file = io::BufWriter::new(fs::File::create("test.txt")?);

    println!("going to write: \"hi there\\n\"");
    file.write_all(b"hi there\n")?;

    println!("going to write: \"hi there\\nmy name is Joey\\n\"");
    file.write_all(b"hi there\nmy name is Joey\n")?;

    Ok(())
}
