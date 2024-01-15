use systemd::journal;
use chrono::{NaiveDateTime};
use lazy_static::lazy_static;

lazy_static!(
	// system timezone
	static ref LOCAL_TZ_OFFSET: i64 = chrono::offset::Local::now().offset().local_minus_utc() as i64;
);

fn print_a_message(entry: Result<Option<journal::JournalRecord>,systemd::Error>) {
	match entry {
		Ok(Some(entry)) => 
		{
			let timestamp = match entry.get("_SOURCE_REALTIME_TIMESTAMP") {
				Some(t) => {
					let t = t.parse::<u64>().unwrap();
					let t = t / 1000000; // convert from ns to seconds
					let t = t as i64;
					let t = match NaiveDateTime::from_timestamp_opt(t + (*LOCAL_TZ_OFFSET), 0) {
						Some(t) => t,
						None => {
							// now time
							chrono::offset::Local::now().naive_local()
						},
					};
					let t = t.format("%b %d %H:%M:%S");
					t.to_string()
				},
				None => {
					// do it in format Jan 15 18:36:55
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
			// println!("dump: {:#?}", entry);
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


fn main() {
	let mut j = journal::OpenOptions::default().open().expect("Could not open journal");
	
	j.seek_tail().expect("Failed to seek to tail");

	println!("Seeked to tail, waiting for next entry ...\n");
	j.wait(None).expect("Failed to wait for last entry");
	j.previous().expect("Failed to position cursor for following tail");
	
	loop { 
		match j.wait(None) {
			Ok(_) => {
				
				print_a_message(j.next_entry());
				loop {
					let next = j.next_entry();
					match next {
						Ok(Some(_)) => {
							// print!("another: ");
							print_a_message(next);
						},
						Ok(None) => {
							// println!("Last of batch reached");
							break;
						},
						Err(e) => {
							println!("Error in multi batch: {}", e);
							break;
						},
					}
				}
			},
			Err(_) => println!("Timeout"),
		}
	}


}
