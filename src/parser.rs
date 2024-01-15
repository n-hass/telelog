use chrono::{DateTime, TimeZone, Local, LocalResult};
use lazy_static::lazy_static;

use systemd::journal as sysjournal;

lazy_static!(
	// system timezone
	static ref LOCAL_TZ_OFFSET: i64 = chrono::offset::Local::now().offset().local_minus_utc() as i64;
);

pub fn print_a_message(entry: Result<Option<sysjournal::JournalRecord>,systemd::Error>) {
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
					let t = t.format("%b %d %H:%M:%S");
					t.to_string()
				},
				None => {
					// just make a timestamp from the current time
					chrono::offset::Local::now().format("%b %d %H:%M:%S").to_string()
				},
			};

			let identifier = match entry.get("SYSLOG_IDENTIFIER") {
				Some(i) => i,
				None => "unknown",
			};

			let message = match entry.get("MESSAGE") {
				Some(m) => m,
				None => "unknown",
			};

			println!("[{}] {}: {}", timestamp, identifier, message);
		},
		Ok(None) => {
			println!("No entry");
			return;
		},
		Err(e) => {
			println!("Error: {}", e);
			return;
		},
	};
		
}