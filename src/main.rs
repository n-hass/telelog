use std::path::PathBuf;

use systemd::journal::Journal as SysJournal;
use systemd::journal as sysjournal;

mod journal;
use journal::open_journal_tail;

mod config;
use config::{read_config, AppSettings, parse_cli_args};

mod parser;
use parser::parse_message;

mod filter;
use filter::filter_log_entry;

mod telegram;

async fn process_entry(entry: Result<Option<sysjournal::JournalRecord>,systemd::Error>) {
	match parse_message(entry) {
		Some(entry) => {
			if filter_log_entry(&entry) {
				return
			}
			// println!("[{}] {}: {}", entry.timestamp.format("%b %d %H:%M:%S"), entry.identifier, entry.message);
			telegram::send_log_entry(entry).await;
		},
		None => {},
	}
}

async fn process_batch(j: &mut SysJournal) {
	let entry = j.next_entry();
	process_entry(entry).await;

	loop {
		let next = j.next_entry();
		match next {
			Ok(Some(_)) => {
				process_entry(next).await;
			},
			Ok(None) => {
				break;
			},
			Err(e) => {
				println!("[process_batch] Error: {}", e);
				break;
			},
		}
	}
}

async fn init(settings: AppSettings) -> SysJournal {
	let mut j = open_journal_tail();
	filter::init(&settings, &mut j);
	telegram::init(settings).await;
	j
}

#[tokio::main]
async fn main() {

	let args = parse_cli_args();

	let mut config_path = "/etc/telelog.toml";
	match args.get_one::<PathBuf>("config") {
		Some(path) => config_path = path.to_str().unwrap(),
		None => println!("[main] Config file not specified, using '/etc/telelog.toml'"),
	}

	let settings = match read_config(config_path) {
		Ok(settings) => settings,
		Err(e) => {
			println!("[main] Error reading config: {}", e);
			return;
		}
	};

	let mut j = init(settings).await;

	loop {
		match j.wait(None) {
			Ok(_) => process_batch(&mut j).await,
			Err(_) => println!("[main] Timeout"),
		}
	}
}
