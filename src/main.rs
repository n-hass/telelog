use systemd::journal::Journal as SysJournal;

mod journal;
use journal::open_journal_tail;

mod parser;
use parser::print_a_message;

mod filter;

mod config;
use config::read_config;

fn process_batch(j: &mut SysJournal) {
	print_a_message(j.next_entry());

	loop {
		let next = j.next_entry();
		match next {
			Ok(Some(_)) => {
				print_a_message(next);
			},
			Ok(None) => {
				break;
			},
			Err(e) => {
				println!("[main loop] Error: {}", e);
				break;
			},
		}
	}
}

fn main() {

	let settings = match read_config() {
		Ok(settings) => settings,
		Err(e) => {
			println!("Error reading config: {}", e);
			return;
		}
	};

	let mut j = open_journal_tail();
	
	filter::init(&settings, &mut j);

	loop {
		match j.wait(None) {
			Ok(_) => process_batch(&mut j),
			Err(_) => println!("Timeout"),
		}
	}
}
