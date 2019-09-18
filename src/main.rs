//! Generate an ical file from the specification of course events.

use failure::{format_err, Error};
use toml;

mod course;

use std::env::args;
use std::fs::File;
use std::io::{BufReader, Read};

fn main() -> Result<(), Error> {
    let path = args()
        .skip(1)
        .next()
        .ok_or(format_err!("Expects course as argument"))?;
    let mut course_toml = String::new();
    BufReader::new(File::open(&path)?).read_to_string(&mut course_toml)?;

    let mut course: course::Course = toml::from_str(&course_toml)?;
    course.generate_repeats()?;

    println!("{:#?}", course);

    Ok(())
}
