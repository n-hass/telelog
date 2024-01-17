use std::path::PathBuf;
use config::Config;
use serde::Deserialize;

use clap::{arg, command, value_parser, ArgAction, Command};

#[derive(Debug, Deserialize)]
pub struct AppSettings {
	pub telegram: TelegramSettings,
	pub filters: FiltersSettings,
}

impl Default for AppSettings {
	fn default() -> Self {
		AppSettings {
			telegram: TelegramSettings {
				chat_id: "".to_string(),
				api_key: None,
				flush_seconds: None,
			},
			filters: FiltersSettings {
				priority: None,
				syslog_identifier: None,
				message: None,
			},
		}
	}
}

#[derive(Debug, Deserialize)]
pub struct TelegramSettings {
	pub chat_id: String,
	pub api_key: Option<String>,
	pub flush_seconds: Option<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FiltersSettings {
	pub priority: Option<Vec<Filter>>,
	pub syslog_identifier: Option<Vec<Filter>>,
	pub message: Option<Vec<Filter>>,
}

#[derive(Debug, Deserialize)]
pub struct Filter {
	#[serde(rename = "type")]
	pub filter_type: String,
	pub value: String,
	pub action: Option<String>,
}


fn get_environment_variable(name: &str) -> Option<String> {
	match std::env::var(name) {
		Ok(value) => Some(value),
		Err(_) => None,
	}
}

pub fn parse_cli_args() -> clap::ArgMatches {
	return command!()
		.arg(
				arg!(
						-c --config <FILE> "Sets a custom config file"
				)
				// We don't have syntax yet for optional options, so manually calling `required`
				.required(false)
				.value_parser(value_parser!(PathBuf)),
		)
		.get_matches()
}

pub fn read_config(filepath: &str) -> Result<AppSettings, config::ConfigError> {
	let builder = Config::builder()
		.add_source(config::File::with_name(filepath));

	match builder.build() {
		Ok(settings) => {
			let settings = settings.try_deserialize::<AppSettings>();
			if settings.is_ok() {
				let mut settings = settings.unwrap();
				
				if settings.telegram.api_key.is_none() {
					match get_environment_variable("TELEGRAM_API_KEY") {
						Some(api_key) => {
							settings.telegram.api_key = Some(api_key);
							return Ok(settings)
						},
						None => {
							return Err(config::ConfigError::Message("[config] No API key set in file as telegram.api_key, and no TELEGRAM_API_KEY environment variable".to_string()))
						}
					}
				}

				if settings.telegram.flush_seconds.is_none() {
					settings.telegram.flush_seconds = Some(5);
				}

				return Ok(settings)
			}
			else { return settings }
		},
		Err(e) => {
			println!("[config] Error reading config: {}", e);
			Err(e)
		}
	}
}