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

#[derive(Debug, Clone)]
pub struct LogEntry {
	pub timestamp: DateTime<Local>,
	pub identifier: String,
	pub message: String,
	pub priority: u8,
}

impl LogEntry {
	pub fn get_copy(&self, field_string: &str) -> Result<String, String> {
		match field_string {
			"timestamp" => Ok(self.timestamp.to_string()),
			"identifier" => Ok(self.identifier.clone()),
			"SYSLOG_IDENTIFIER" => Ok(self.identifier.clone()),
			"message" => Ok(self.message.clone()),
			"MESSAGE" => Ok(self.message.clone()),
			"priority" => Ok(self.priority.to_string()),
			_ => Err(format!("[LogEntry get] Field {} not found", field_string)),
		}
	}
}