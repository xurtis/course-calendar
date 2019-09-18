//! Events that occur for a particular course

use chrono::{offset::FixedOffset, DateTime, Duration};
use serde::{de, Deserialize, Deserializer};
use url::Url;

use std::fmt;

/// All of the events for a particular course
#[derive(Debug, Clone, Deserialize)]
pub struct Course {
    #[serde(rename = "session", default)]
    repeat_sessions: Vec<RepeatSession>,
    #[serde(rename = "week", default)]
    weeks: Vec<Week>,
    #[serde(rename = "assignment", default)]
    assignments: Vec<Assignment>,
}

/// A week with interactive sessions
#[derive(Debug, Clone, Deserialize)]
struct Week {
    #[serde(deserialize_with = "deserialize_datetime")]
    start: DateTime<FixedOffset>,
    #[serde(rename = "session", default)]
    sessions: Vec<Session>,
}

/// An interactive session such as a lecture, tutorial, lab, or seminar
#[derive(Debug, Clone, Deserialize)]
struct Session {
    kind: String,
    title: Option<String>,
    #[serde(default)]
    presenters: Vec<String>,
    location: Option<String>,
    #[serde(deserialize_with = "deserialize_datetime")]
    time: DateTime<FixedOffset>,
    #[serde(deserialize_with = "deserialize_duration")]
    duration: Duration,
}

/// An interactive session that repeats in multiple weeks
#[derive(Debug, Clone, Deserialize)]
struct RepeatSession {
    kind: String,
    title: Option<String>,
    #[serde(default)]
    presenters: Vec<String>,
    location: Option<String>,
    #[serde(deserialize_with = "deserialize_datetime")]
    first: DateTime<FixedOffset>,
    #[serde(deserialize_with = "deserialize_duration")]
    duration: Duration,
    weeks: Vec<u64>,
}

/// An assignment with presentations and submissions
#[derive(Debug, Clone, Deserialize)]
struct Assignment {
    name: String,
    #[serde(deserialize_with = "deserialize_url")]
    link: Url,
    description: Option<String>,
    value: Option<f64>,
    #[serde(rename = "submission", default)]
    submissions: Vec<Submission>,
    #[serde(rename = "presentation", default)]
    presentations: Vec<Presentation>,
}

/// A submission deadline for an assignment
#[derive(Debug, Clone, Deserialize)]
struct Submission {
    name: String,
    #[serde(deserialize_with = "deserialize_datetime")]
    time: DateTime<FixedOffset>,
}

/// A presentation within a particular session
#[derive(Debug, Clone, Deserialize)]
struct Presentation {
    name: String,
    session: String,
    weeks: Vec<u64>,
}

struct DateTimeVisitor;

impl<'de> de::Visitor<'de> for DateTimeVisitor {
    type Value = DateTime<FixedOffset>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A TOML Datetime")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        DateTime::parse_from_rfc3339(value).map_err(E::custom)
    }

    fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let value = map.next_value::<&'de str>()?;
        DateTime::parse_from_rfc3339(value).map_err(<A::Error as de::Error>::custom)
    }
}

fn deserialize_datetime<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<DateTime<FixedOffset>, D::Error> {
    deserializer.deserialize_map(DateTimeVisitor)
}

struct DurationVisitor;

impl<'de> de::Visitor<'de> for DurationVisitor {
    type Value = Duration;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A duration in seconds")
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Duration::seconds(value))
    }
}

fn deserialize_duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
    deserializer.deserialize_i64(DurationVisitor)
}

struct UrlVisitor;

impl<'de> de::Visitor<'de> for UrlVisitor {
    type Value = Url;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A URL")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Url::parse(value).map_err(E::custom)
    }
}

fn deserialize_url<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Url, D::Error> {
    deserializer.deserialize_str(UrlVisitor)
}
