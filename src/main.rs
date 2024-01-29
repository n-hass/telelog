use std::path::PathBuf;

use systemd::journal::Journal as SysJournal;
use systemd::journal as sysjournal;

mod journal;
use journal::{open_journal_tail, LogEntry};

mod config;
use config::{read_config, AppSettings, parse_cli_args};

mod parser;
use parser::parse_message;

mod filter;
use filter::filter_log_entry;
use tokio::sync::mpsc;

mod telegram;

async fn process_entry(entry: sysjournal::JournalRecord, telegram_tx: &mpsc::Sender<LogEntry>) {
	match parse_message(entry) {
		Some(entry) => {
			if filter_log_entry(&entry) {
				return
			}
			match telegram_tx.send(entry).await {
				Ok(_) => {},
				Err(e) => println!("[process_entry] Error in message channel: {}", e),
			};
		},
		None => {},
	}
}

async fn process_batch(j: &mut SysJournal, telegram_tx: &mpsc::Sender<LogEntry>) {
	if let Ok(Some(entry)) = j.next_entry() {
		process_entry(entry, telegram_tx).await;
	}

	loop {
		if let Ok(Some(next)) = j.next_entry() {
			process_entry(next, telegram_tx).await;
		} else {
			break;
		}
	}
}

fn init(settings: AppSettings) -> (SysJournal, mpsc::Sender<LogEntry>) {
	let mut j = open_journal_tail();
	filter::init(&settings, &mut j);
	let (tx, rx) = tokio::sync::mpsc::channel::<LogEntry>(40);
	telegram::init(settings, rx);
	(j, tx)
}

#[tokio::main]
async fn main() {

	let args = parse_cli_args();

	let config_path = match args.get_one::<PathBuf>("config") {
		Some(path) => path.to_str().unwrap(),
		None => {
			println!("[main] Config file not specified, using '/etc/telelog.toml'");
			"/etc/telelog.toml"
		},
	};

	let settings = match read_config(config_path) {
		Ok(settings) => settings,
		Err(e) => {
			println!("[main] Error reading config: {}", e);
			return;
		}
	};

	let (mut j, tx) = init(settings);

	loop {
		match j.wait(None) {
			Ok(_) => process_batch(&mut j, &tx).await,
			Err(_) => println!("[main] Timeout"),
		}
	}
}
