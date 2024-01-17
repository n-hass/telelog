use chrono::{DateTime, TimeZone, Local, LocalResult};
use lazy_static::lazy_static;

use systemd::journal as sysjournal;

use crate::journal::LogEntry;

use regex::Regex;
lazy_static!(
	static ref RE: Regex = Regex::new("\x1b\\[[0-9;]*m").unwrap();
);

// do the bare minimum to clean the message - remove ANSI tty colouring and trailing newlines
fn clean_message(message: &str) -> String {
	RE.replace_all(message, "").trim_end_matches('\n').to_string()
}

pub fn parse_message(entry: Result<Option<sysjournal::JournalRecord>,systemd::Error>) -> Option<LogEntry> {
	match entry {
		Ok(Some(entry)) => 
		{
			let timestamp = match entry.get("_SOURCE_REALTIME_TIMESTAMP") {
				Some(t) => {
					let t = t.parse::<u64>().unwrap();
					let t = t / 1000000; // convert from ns to seconds
					let t = t as i64;
					let t: DateTime<Local> = match Local.timestamp_opt(t, 0) {
						LocalResult::Single(t) => t,
						LocalResult::None => panic!("Invalid timestamp"),
						LocalResult::Ambiguous(_, _) => panic!("Ambiguous timestamp"),
					};
					t
				},
				None => {
					// just make a timestamp from the current time
					chrono::offset::Local::now()
				},
			};

			let identifier = match entry.get("SYSLOG_IDENTIFIER") {
				Some(i) => i.to_owned(),
				None => "unknown".to_owned(),
			};

			let message = match entry.get("MESSAGE") {
				Some(m) => clean_message(m),
				None => "unknown".to_owned(),
			};

			let priority = match entry.get("PRIORITY") {
				Some(p) => p.parse::<u8>().unwrap(),
				None => 7,
			};

			return Some(LogEntry {
				timestamp: timestamp,
				identifier: identifier,
				message: message,
				priority: priority,
			});
		},
		Ok(None) => {
			return None;
		},
		Err(e) => {
			println!("[parser] Error: {}", e);
			return None;
		},
	};
}
