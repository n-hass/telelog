use systemd::journal;

fn print_a_message(entry: Result<Option<journal::JournalRecord>,systemd::Error>) {
	match entry {
		Ok(Some(entry)) => 
		{
			// println!("{} {} {}", entry.get("_SOURCE_REALTIME_TIMESTAMP").unwrap(), entry.get("_COMM").unwrap(), entry.get("MESSAGE").unwrap());
			println!("{}: {}", entry.get("_COMM").unwrap(), entry.get("MESSAGE").unwrap());
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
