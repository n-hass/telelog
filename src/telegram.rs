use lazy_static::lazy_static;
use signal_hook::{consts::{SIGTERM,SIGINT}, iterator::Signals};
use tokio::time::sleep;
use tokio::sync::Mutex as AsyncMutex;
use reqwest;
use std::time::Duration;
use serde::Deserialize;
use serde_json::Error as JsonError;
use std::sync::OnceLock;

use crate::journal::LogEntry;
use crate::config::AppSettings;

#[derive(Debug)]
struct TelegramContext {
	chat_id: String,
	api_key: String,
	flush_seconds: u16,
}

lazy_static!(
	static ref LOG_ENTRY_BUFFER: AsyncMutex<Vec<LogEntry>> = AsyncMutex::new(Vec::new());
	static ref PROCESSED_MESSAGE_BUFFER: AsyncMutex<Vec<String>> = AsyncMutex::new(Vec::new());
	static ref SEND_LOCK: AsyncMutex<()> = AsyncMutex::new(());
	static ref REQUEST_CLIENT: reqwest::Client = reqwest::Client::new();
);
static TELEGRAM_CONTEXT: OnceLock<TelegramContext> = OnceLock::new();

#[derive(Deserialize)]
struct ErrorResponse {
    // ok: Option<bool>,
    // error_code: Option<u16>,
    // description: Option<String>,
    parameters: Option<RetryParameters>,
}

#[derive(Deserialize)]
struct RetryParameters {
    retry_after: Option<u64>,
}

pub async fn init(settings: AppSettings) {
	TELEGRAM_CONTEXT.set(TelegramContext {
		chat_id: settings.telegram.chat_id,
		api_key: settings.telegram.api_key.unwrap(),
		flush_seconds: settings.telegram.flush_seconds.unwrap_or(5),
	}).expect("Initialisation only occurs once");

	tokio::task::spawn(async move {
		let mut signals = Signals::new(&[SIGTERM, SIGINT]).unwrap();
		for signal in signals.forever() {
			match signal {
				SIGTERM | SIGINT => {
					println!("[telegram] Received stop signal");
					flush_log_buffer().await;
					std::process::exit(0);
				},
				_ => {},
			};
		}
	});

	println!("[telegram] initialised");
}

pub async fn send_log_entry(entry: LogEntry) {
	let mut buffer = LOG_ENTRY_BUFFER.lock().await;
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
		let flush_seconds = match TELEGRAM_CONTEXT.get() {
			Some(context) => context.flush_seconds,
			None => 5,
		};

		tokio::spawn(async move {
			sleep(Duration::from_secs(flush_seconds as u64)).await;
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

fn generate_messages(buffer: &Vec<LogEntry>) -> Vec<String> {
	let mut message_list: Vec<String> = vec![];
	let mut current_message = String::from("<code>\n");

	for entry in buffer {
		let new_entry_string = format!("{}[{}] {}: {}\n", colour_translate(entry.priority), entry.timestamp.format("%b %d %H:%M:%S"), entry.identifier, entry.message);
		
		if current_message.len() + new_entry_string.len() >= 4088 {
			current_message.push_str("</code>");
			message_list.push(current_message);
			current_message = String::from("<code>\n");
		}

		current_message.push_str(&new_entry_string);
	}
	if current_message != "<code>\n" {
		current_message.push_str("</code>");
		message_list.push(current_message);
	}
	return message_list
}

async fn send_telegram_message(message: &String, api_key: &String, chat_id: &String) -> Result<reqwest::Response, reqwest::Error> {
	let _guard = SEND_LOCK.lock().await;
	let response = REQUEST_CLIENT.post(&format!("https://api.telegram.org/bot{}/sendMessage", api_key))
		.form(&[("chat_id", chat_id), ("text", message), ("parse_mode", &"HTML".to_string())])
		.send()
		.await;

	tokio::spawn(async move {
		sleep(Duration::from_secs(1)).await;
		drop(_guard);
	});

	return response;
}

async fn flush_log_buffer() {
	let mut buffer = LOG_ENTRY_BUFFER.lock().await;
	let message_list = generate_messages(&buffer);
	buffer.clear();
	drop(buffer); // release the lock

	if message_list.len() == 0 {
		println!("[telegram] flush was ran, but buffer was empty");
		return
	}

	let (api_key, chat_id) = match TELEGRAM_CONTEXT.get() {
		Some(context) => (context.api_key.clone(), context.chat_id.clone()),
		None => {
			println!("[telegram] flush was ran, but context was empty");
			return
		}
	};

	let mut old_unsent_messages = PROCESSED_MESSAGE_BUFFER.lock().await;
	let mut new_unsent_messages: Vec<String> = Vec::new();

	// try sending all the messages
	for buffer in [&old_unsent_messages, &message_list] {
		for message in buffer {
			let result = send_telegram_message(message, &api_key, &chat_id).await;

			if let Err(e) = result {
				println!("[telegram] Failed: {}", e);
				new_unsent_messages.push(message.to_string());
				continue
			}

			let response = result.unwrap();

			if !response.status().is_success() {
				let status = response.status();
				let text = response.text().await.unwrap();
			
				// Error handling specifics
				match status.as_u16() {
					429 => {
						match serde_json::from_str(&text) as Result<ErrorResponse, JsonError> {
							Ok(error_response) => {
								if let Some(parameters) = error_response.parameters {
									if let Some(retry_after) = parameters.retry_after {
										println!("[telegram] API response 429: pausing messages for {} seconds", retry_after);
										tokio::spawn(async move {
											let _guard = SEND_LOCK.lock().await;
											sleep(Duration::from_secs(retry_after)).await;
											drop(_guard);
										});
									}
								}
							}
							Err(e) => {
								println!("[telegram] Failed to parse 429 response: {}", e);
							}
						}

						new_unsent_messages.push(message.to_string());
					},
					_ => {
						println!("[telegram] API response {}: {:?}", status, text);
						// new_unsent_messages.push(message.to_string());
					}
				}
			}
		}
	}

	*old_unsent_messages = new_unsent_messages;

	drop(old_unsent_messages);
}
