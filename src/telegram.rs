use lazy_static::lazy_static;
use signal_hook::{consts::{SIGTERM,SIGINT}, iterator::Signals};
use tokio::time::sleep;
use tokio::sync::{Mutex as AsyncMutex, mpsc, Notify};
use reqwest;
use std::time::Duration;
use serde::Deserialize;
use serde_json::Error as JsonError;
use std::sync::OnceLock;

use crate::{helpers::*, journal::LogEntry};
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
	static ref RETRY_FLAG: Notify = Notify::new();
	static ref RETRY_COUNT: AsyncMutex<u64> = AsyncMutex::new(1);
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

pub fn init(settings: AppSettings, mut rx: mpsc::Receiver<LogEntry>) {
	let flush_seconds = settings.telegram.flush_seconds.unwrap_or(5);
	TELEGRAM_CONTEXT.set(TelegramContext {
		chat_id: settings.telegram.chat_id,
		api_key: settings.telegram.api_key.unwrap(),
		flush_seconds: flush_seconds,
	}).expect("Initialisation only occurs once");

	// task to handle incoming signals
	tokio::spawn(async move {
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
	
	// spawn a task to process messages as they are sent from the main task
	tokio::spawn(async move {
		while let Some(entry) = rx.recv().await {
			let mut buffer = LOG_ENTRY_BUFFER.lock().await;
			buffer.push(entry.clone());
			if entry.priority <= 2 {
				drop(buffer); // release the lock
				// if this is a critical entry, flush the buffer immediately
				tokio::spawn(async move {
					flush_log_buffer().await;
				});
				continue
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
	});

	// task to handle retrying failed flushes
	tokio::spawn( async move {
		loop {
			RETRY_FLAG.notified().await;
			let retry_count = RETRY_COUNT.lock().await;
			if *retry_count > 5 {
				// TODO: write to disk maybe?
			};
			sleep(Duration::from_secs(*retry_count * 2 * flush_seconds as u64)).await;
			tokio::spawn(async move {
				flush_log_buffer().await;
			});
		}
	});

	println!("[telegram] initialised");
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
		eprintln!("[telegram] flush was ran, but buffer was empty");
		return
	}

	let (api_key, chat_id) = match TELEGRAM_CONTEXT.get() {
		Some(context) => (context.api_key.clone(), context.chat_id.clone()),
		None => {
			eprintln!("[telegram] flush was ran, but context was empty");
			return
		}
	};

	let mut old_unsent_messages = PROCESSED_MESSAGE_BUFFER.lock().await;
	let mut failed_unsent_messages: Vec<String> = Vec::new();

	let all_unsent_messages = flatten_messages([&old_unsent_messages, &message_list]);

	// try sending all the messages
	for message in all_unsent_messages {
		let result = send_telegram_message(&message, &api_key, &chat_id).await;

		if let Err(e) = result {
			eprintln!("[telegram] Failed: {}", e);
			failed_unsent_messages.push(message.to_string());
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
									eprintln!("[telegram] API response 429: pausing messages for {} seconds", retry_after);
									tokio::spawn(async move {
										let _guard = SEND_LOCK.lock().await;
										sleep(Duration::from_secs(retry_after)).await;
										drop(_guard);
									});
								}
							}
						}
						Err(e) => {
							eprintln!("[telegram] Failed to parse 429 response: {}", e);
						}
					}

					failed_unsent_messages.push(message.to_string());
				},
				_ => {
					println!("[telegram] API response {}: {:?}", status, text);
					failed_unsent_messages.push(message.to_string());
				}
			}
		}
	}
	
	let mut retry_count = RETRY_COUNT.lock().await;
	if failed_unsent_messages.len() > 0 {
		*retry_count *= 2;
		RETRY_FLAG.notify_one();
	} else {
		*retry_count = 1;
	}

	*old_unsent_messages = failed_unsent_messages;
}
