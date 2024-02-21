use core::time;
use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime,Local};
use systemd::{journal, Journal};

pub fn open_journal_tail() -> Journal {
	let mut j = journal::OpenOptions::default().open().expect("Could not open journal");
	
	j.seek_tail().expect("[open_journal] Failed to seek to tail");

	println!("[open_journal] Seeked to tail ...\n");
	j.wait(None).expect("[open_journal] Failed to wait for last entry");
	j.previous().expect("[open_journal] Failed to position cursor for following tail");
	j
}

#[derive(Debug, Clone, Default)]
pub struct LogEntry {
	pub priority: u8,
	pub timestamp: DateTime<Local>,
	pub identifier: String,
	pub message: String,
	raw_fields: BTreeMap<String, String>,
}

impl LogEntry {
	pub fn new(priority: u8, timestamp: DateTime<Local>, identifier: String, message: String, raw_fields: BTreeMap<String, String>) -> Self {
		LogEntry {
			priority: priority,
			timestamp: timestamp,
			identifier: identifier,
			message: message,
			raw_fields: raw_fields,
		}
	}

	pub fn get_field(&self, field_string: &str) -> Result<String, String> {
		match field_string {
			"PRIORITY" => Ok(self.priority.to_string()),
			"TIMESTAMP" => Ok(self.timestamp.to_string()),
			"_SOURCE_REALTIME_TIMESTAMP" => Ok(self.timestamp.to_string()),
			"IDENTIFIER" => Ok(self.identifier.to_owned()),
			"MESSAGE" => Ok(self.message.to_owned()),
			_ => self.raw_fields.get(field_string).map(|s| s.to_owned()).ok_or_else(|| format!("[LogEntry get] Field {} not found", field_string))
		}
	}
}