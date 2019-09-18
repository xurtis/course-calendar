//! Events that occur for a particular course

use chrono::{offset::FixedOffset, DateTime, Duration};
use serde::{de, Deserialize, Deserializer};
use url::Url;

use failure::{Error, format_err};

use std::fmt;

/// All of the events for a particular course
#[derive(Debug, Clone, Deserialize)]
pub struct Course {
    code: String,
    name: String,
    #[serde(deserialize_with = "deserialize_url")]
    link: Url,
    #[serde(rename = "week", default)]
    weeks: Vec<Week>,
    #[serde(rename = "assignment", default)]
    assignments: Vec<Assignment>,
    #[serde(rename = "session", default)]
    repeat_sessions: Vec<RepeatSession>,
}

impl Course {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    /// Generate all repeated sessions in the course
    pub fn generate_repeats(&mut self) -> Result<(), Error> {
        let mut sessions = Vec::new();

        for session in &self.repeat_sessions {
            let first_week = if let Some(first) = session.weeks.get(0) {
                self.weeks.get(*first).ok_or(format_err!("Requested repeat of {} session in non-existent week {}", session.kind, first))?.start
            } else {
                continue;
            };

            for week_no in &session.weeks {
                let week = self.weeks.get(*week_no).ok_or(format_err!("Tried to schedule repeat of {} session in non-existent week {}", session.kind, week_no))?;
                let duplicate = session.duplicate(first_week, week.start);
                sessions.push((*week_no, duplicate));
            }
        }

        for (week, session) in sessions.drain(..) {
            self.weeks[week].sessions.push(session);
        }

        Ok(())
    }

    /// Generate an iterator over the events in chronological order
    pub fn events(&self) -> impl Iterator<Item = Event> {
        let mut events = Vec::new();

        for week in &self.weeks {
            for session in &week.sessions {
                events.push(session.into());
            }
        }

        for assignment in &self.assignments {
            events.extend(assignment.events(&self));
        }

        events.sort();
        events.into_iter()
    }
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
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Session {
    #[serde(deserialize_with = "deserialize_datetime")]
    time: DateTime<FixedOffset>,
    title: Option<String>,
    location: Option<String>,
    #[serde(default)]
    presenters: Vec<String>,
    kind: String,
    #[serde(deserialize_with = "deserialize_duration")]
    duration: Duration,
}

impl Session {
    fn location(&self) -> Option<&str> {
        self.location.as_ref().map(|s| s.as_str())
    }

    fn presenters(&self) -> Vec<&str> {
        self.presenters.iter().map(|s| s.as_str()).collect::<Vec<_>>()
    }
}

/// An interactive session that repeats in multiple weeks
#[derive(Debug, Clone, Deserialize)]
struct RepeatSession {
    #[serde(deserialize_with = "deserialize_datetime")]
    first: DateTime<FixedOffset>,
    title: Option<String>,
    location: Option<String>,
    #[serde(default)]
    presenters: Vec<String>,
    kind: String,
    #[serde(deserialize_with = "deserialize_duration")]
    duration: Duration,
    weeks: Vec<usize>,
}

impl RepeatSession {
    fn duplicate(&self, first_week: DateTime<FixedOffset>, week_start: DateTime<FixedOffset>) -> Session {
        let offset = self.first - first_week;

        Session {
            kind: self.kind.clone(),
            title: self.title.clone(),
            presenters: self.presenters.clone(),
            location: self.location.clone(),
            time: week_start + offset,
            duration: self.duration,
        }
    }
}

/// An assignment with presentations and submissions
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Assignment {
    name: String,
    description: Option<String>,
    #[serde(deserialize_with = "deserialize_url")]
    link: Url,
    value: Option<u64>,
    #[serde(rename = "submission", default)]
    submissions: Vec<Submission>,
    #[serde(rename = "presentation", default)]
    presentations: Vec<Presentation>,
}

impl Assignment {
    fn events<'c>(&'c self, course: &'c Course) -> impl Iterator<Item = Event<'c>> {
        let presentations = self.presentations.iter()
            .flat_map(|p| p.weeks.iter().map(move |w| (*w, p)))
            .flat_map(move |(w, p)| course.weeks.get(w).map(|w| (w, p)))
            .flat_map(move |(w, p)| w.sessions.iter().map(move |s| (s, p)));

        AssignmentEvents {
            assignment: &self,
            submissions: self.submissions.iter(),
            presentations,
        }
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_str())
    }
}

struct AssignmentEvents<'c, S, P> {
    assignment: &'c Assignment,
    submissions: S,
    presentations: P,
}

impl<'c, S, P> Iterator for AssignmentEvents<'c, S, P>
where
    S: Iterator<Item = &'c Submission>,
    P: Iterator<Item = (&'c Session, &'c Presentation)>,
{
    type Item = Event<'c>;

    fn next(&mut self) -> Option<Self::Item>{
        loop {
            if let Some(submission) = self.submissions.next() {
                let event = Event {
                    start: submission.time,
                    base: EventBase::Submission(self.assignment, submission),
                };
                break Some(event);
            } else if let Some((session, presentation)) = self.presentations.next() {
                if session.kind != presentation.session {
                    continue;
                }

                let event = Event {
                    start: session.time,
                    base: EventBase::Presentation(self.assignment, presentation, session),
                };
                break Some(event);
            } else {
                break None;
            }
        }
    }
}

/// A submission deadline for an assignment
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Submission {
    #[serde(deserialize_with = "deserialize_datetime")]
    time: DateTime<FixedOffset>,
    name: String,
    description: Option<String>,
}

impl Submission {
    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_str())
    }
}

/// A presentation within a particular session
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Presentation {
    name: String,
    session: String,
    description: Option<String>,
    weeks: Vec<usize>,
}

impl Presentation {
    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Event<'c> {
    start: DateTime<FixedOffset>,
    base: EventBase<'c>,
}

impl<'c> Event<'c> {
    pub fn start(&self) -> DateTime<FixedOffset> {
        self.start
    }

    pub fn duration(&self) -> Duration {
        use EventBase::*;
        match self.base {
            Session(s) => s.duration,
            Submission(_, _) => Duration::minutes(5),
            Presentation(_, _, s) => s.duration,
        }
    }

    pub fn end(&self) -> DateTime<FixedOffset> {
        self.start() + self.duration()
    }

    pub fn title(&self) -> String {
        match self.base {
            EventBase::Session(Session { title: Some(title), kind, .. }) => format!("{} ({})", title, kind),
            EventBase::Session(Session { kind, .. }) => format!("({})", kind),
            EventBase::Submission(a, s) => format!("{}: {} (submission)", a.name, s.name),
            EventBase::Presentation(a, p, _) => format!("{}: {} (presentation)", a.name, p.name),
        }
    }

    pub fn location(&self) -> Option<&str> {
        match self.base {
            EventBase::Session(s) => s.location(),
            EventBase::Submission(_, _) => None,
            EventBase::Presentation(_, _, s) => s.location(),
        }
    }

    pub fn presenters(&self) -> impl Iterator<Item = &str> {
        match self.base {
            EventBase::Session(s) => s.presenters().into_iter(),
            EventBase::Submission(_, _) => Vec::new().into_iter(),
            EventBase::Presentation(_, _, s) => s.presenters().into_iter(),
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self.base {
            EventBase::Session(_) => None,
            EventBase::Submission(_, s @Submission { description: Some(_), .. }) => s.description(),
            EventBase::Submission(a, _) => a.description(),
            EventBase::Presentation(_, p @Presentation { description: Some(_), .. }, _) => p.description(),
            EventBase::Presentation(a, _, _) => a.description(),
        }
    }

    pub fn link(&self) -> Option<&Url> {
        match self.base {
            EventBase::Session(_) => None,
            EventBase::Submission(a, _) => Some(&a.link),
            EventBase::Presentation(a, _, _) => Some(&a.link),
        }
    }
}

impl<'c> From<&'c Session> for Event<'c> {
    fn from(session: &'c Session) -> Self {
        Event {
            start: session.time,
            base: EventBase::Session(session),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum EventBase<'c> {
    Session(&'c Session),
    Submission(&'c Assignment, &'c Submission),
    Presentation(&'c Assignment, &'c Presentation, &'c Session),
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
