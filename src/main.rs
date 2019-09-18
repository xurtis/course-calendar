//! Generate an ical file from the specification of course events.

use failure::{format_err, Error};
use toml;
use ics::{ICalendar, Event, properties};
use uuid::Uuid;
use chrono::{DateTime, Duration, offset::Utc};

mod course;

use std::env::args;
use std::fs::File;
use std::io::{BufReader, Read, stdout};

fn main() -> Result<(), Error> {
    let path = args()
        .skip(1)
        .next()
        .ok_or(format_err!("Expects course as argument"))?;
    let mut course_toml = String::new();
    BufReader::new(File::open(&path)?).read_to_string(&mut course_toml)?;

    let mut course: course::Course = toml::from_str(&course_toml)?;
    course.generate_repeats()?;

    let mut calendar = ICalendar::new("2.0", "ics-rs");
    calendar.push(properties::Name::new(course.name()));
    calendar.push(properties::CalScale::new("GREGORIAN"));

    for event in course.events() {
        let mut cal_event = Event::new(new_uuid(), time_format(Utc::now()));

        let summary = format!("{} {}", course.code().to_owned(), event.title());
        cal_event.push(properties::Summary::new(summary));
        cal_event.push(properties::DtStart::new(time_format(event.start())));
        cal_event.push(properties::DtEnd::new(time_format(event.end())));
        //cal_event.push(properties::Duration::new(duration_format(event.duration())));
        if let Some(location) = event.location() {
            cal_event.push(properties::Location::new(location));
        }
        for presenter in event.presenters() {
            cal_event.push(properties::Contact::new(presenter));
        }
        if let Some(link) = event.link() {
            cal_event.push(properties::URL::new(link.as_str()));
        }
        if let Some(description) = event.description() {
            let description = description.split('\n').collect::<Vec<_>>();
            let description = description.join("\\n");
            cal_event.push(properties::Description::new(description));
        }

        calendar.add_event(cal_event);
    }

    calendar.write(stdout())?;

    Ok(())
}

fn new_uuid() -> String {
    let mut buffer = Uuid::encode_buffer();
    Uuid::new_v4().to_hyphenated().encode_lower(&mut buffer).to_owned()
}

fn time_format<O>(time: DateTime<O>) -> String
where
    O: chrono::TimeZone,
    DateTime<Utc>: From<DateTime<O>>,
{
    let utc_time: DateTime<Utc> = time.into();
    utc_time.format("%Y%m%dT%H%M%SZ").to_string()
}

fn duration_format(duration: Duration) -> String {
    let days = duration.num_days();
    let consumed = Duration::days(days);
    let hours = (duration - consumed).num_hours();
    let consumed = consumed + Duration::hours(hours);
    let minutes = (duration - consumed).num_minutes();
    let consumed = consumed + Duration::minutes(minutes);
    let seconds = (duration - consumed).num_seconds();
    format!("P{}DT{}H{}M{}S", days, hours, minutes, seconds)
}
