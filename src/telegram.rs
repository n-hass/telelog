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
	println!("[telegram] initialised");
}

pub async fn send_log_entry(entry: LogEntry) {
	let mut buffer = LOG_BUFFER.lock().await;
	buffer.push(entry.clone());

	// if this is a critical entry, flush the buffer immediately
	if entry.priority <= 2 {
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
		let flush_seconds = match settings.telegram.flush_seconds {
			Some(seconds) => seconds,
			None => {
				println!("[telegram] flush_seconds not set, defaulting to 5 seconds");
				5
			},
		};
		drop(settings); // release the lock
		
		tokio::spawn(async move {
			tokio::time::sleep(std::time::Duration::from_secs(flush_seconds as u64)).await;
			flush_log_buffer().await;
		});
	}
}

fn colour_translate(priority: u8) -> String {
	// match priority {
	// 	0 => "#FF3333".to_owned(),
	// 	1 => "#FF6600".to_owned(),
	// 	2 => "#800080".to_owned(),
	// 	3 => "#B22222".to_owned(),
	// 	4 => "#FFD700".to_owned(),
	// 	5 => "#87CEEB".to_owned(),
	// 	6 => "#4169E1".to_owned(),
	// 	7 => "#CDD1D3".to_owned(),
	// 	_ => "#000000".to_owned(),
	// }
	match priority {
		0 => "☢️".to_owned(),
		1 => "‼️".to_owned(),
		2 => "🟣".to_owned(),
		3 => "⭕️".to_owned(),
		4 => "🟡".to_owned(),
		5 => "🔵".to_owned(),
		6 => "⚫️".to_owned(),
		7 => "⚪️".to_owned(),
		_ => "⚪️".to_owned(),
	}
}

async fn flush_log_buffer() {
	let mut buffer = LOG_BUFFER.lock().await;
	// parse the buffer to form the telegram message
	let mut message = String::from("<code>\n");
	for entry in buffer.borrow_mut().iter() {
		let colour = colour_translate(entry.priority);
		message.push_str(&format!("{}[{}] {}: {}\n", colour, entry.timestamp.format("%b %d %H:%M:%S"), entry.identifier, entry.message));
	}
	message.push_str("</code>");
	buffer.clear();
	drop(buffer); // release the lock

	// send the message to telegram
	let settings = APP_SETTINGS_COPY.lock().await;
	let chat_id = (settings.telegram.chat_id).to_owned();
	let api_key = (settings.telegram.api_key.as_ref().unwrap()).to_owned();
	drop(settings);

	let client = reqwest::Client::new();
	match client.post(&format!("https://api.telegram.org/bot{}/sendMessage", api_key))
		.form(&[("chat_id", chat_id), ("text", message), ("parse_mode", "HTML".to_string())])
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
