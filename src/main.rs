//! Generate an ical file from the specification of course events.

use failure::{format_err, Error};
use toml;

mod course;

use std::env::args;
use std::fs::File;
use std::io::{BufReader, Read};

const DATE_FMT: &'static str = "%k:%M %A, %d %B, %Y";

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

    for event in course.events() {
        let title = format!(
            "{} {}",
            course.code().to_owned(),
            event.title(),
        );
        println!("{}", title);
        let mut bar = String::new();
        for _ in 0..title.len() {
            bar.push('=');
        }
        println!("{}", bar);
        println!("Start: {}", event.start().format(DATE_FMT));
        println!("End: {}", event.end().format(DATE_FMT));
        if let Some(location) = event.location() {
            println!("Location: {}", location);
        }
        for presenter in event.presenters() {
            println!("Presented by: {}", presenter);
        }
        if let Some(link) = event.link() {
            println!("Link: {}", link);
        }
        if let Some(description) = event.description() {
            println!("\n{}", description);
        }
        println!();
    }

    Ok(())
}
