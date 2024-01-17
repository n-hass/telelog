use lazy_static::lazy_static;
use std::borrow::BorrowMut;
use tokio;
use tokio::sync::Mutex as AsyncMutex;
use reqwest;

use crate::journal::LogEntry;
use crate::config::AppSettings;
lazy_static!(
	static ref LOG_BUFFER: AsyncMutex<Vec<LogEntry>> = AsyncMutex::new(vec![]);
	static ref APP_SETTINGS_COPY: AsyncMutex<AppSettings> = AsyncMutex::new(AppSettings::default());
);

pub async fn init(settings: AppSettings) {
	let mut app_settings_copy = APP_SETTINGS_COPY.lock().await;
	*app_settings_copy = settings;
}

pub async fn send_log_entry(entry: LogEntry) {
	let mut buffer = LOG_BUFFER.lock().await;
	buffer.push(entry.clone());

	// if this is a critical entry, flush the buffer immediately
	if entry.priority <= 3 {
		drop(buffer); // release the lock
		tokio::spawn(async move {
			flush_log_buffer().await;
		});
		return
	}

	// otherwise schedule a flush if this is the first message in the buffer
	if buffer.len() == 1 {
		drop(buffer); // release the lock
		let settings = APP_SETTINGS_COPY.lock().await;
		let flush_seconds = settings.telegram.flush_seconds.unwrap();
		drop(settings); // release the lock
		
		tokio::spawn(async move {
			tokio::time::sleep(std::time::Duration::from_secs(flush_seconds as u64)).await;
			flush_log_buffer().await;
		});
	}
}

async fn flush_log_buffer() {
	let mut buffer = LOG_BUFFER.lock().await;
	// parse the buffer to form the telegram message
	let mut message = String::from("```\n");
	for entry in buffer.borrow_mut().iter() {
		message.push_str(&format!("[{}] {}: {}\n", entry.timestamp.format("%b %d %H:%M:%S"), entry.identifier, entry.message));
	}
	message.push_str("```");
	buffer.clear();
	drop(buffer); // release the lock

	// send the message to telegram
	let settings = APP_SETTINGS_COPY.lock().await;
	let chat_id = (settings.telegram.chat_id).to_owned();
	let api_key = (settings.telegram.api_key.as_ref().unwrap()).to_owned();
	drop(settings);

	let client = reqwest::Client::new();
	match client.post(&format!("https://api.telegram.org/bot{}/sendMessage", api_key))
		.form(&[("chat_id", chat_id), ("text", message), ("parse_mode", "Markdown".to_string())])
		.send()
		.await {
		Ok(response) => {
			if response.status().is_success() {
				return
			}
			println!("[telegram] API Error: {}", response.status());
		},
		Err(e) => {
			println!("[telegram] Failed: {}", e);
		}
	}

}
