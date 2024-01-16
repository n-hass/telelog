use chrono::{DateTime,Local};
use systemd::{journal, Journal};

pub fn open_journal_tail() -> Journal {
	let mut j = journal::OpenOptions::default().open().expect("Could not open journal");
	
	j.seek_tail().expect("Failed to seek to tail");

	println!("Seeked to tail, waiting for next entry ...\n");
	j.wait(None).expect("Failed to wait for last entry");
	j.previous().expect("Failed to position cursor for following tail");
	j
}

#[derive(Debug, Clone)]
pub struct LogEntry {
	pub timestamp: DateTime<Local>,
	pub identifier: String,
	pub message: String,
	pub priority: u8,
}