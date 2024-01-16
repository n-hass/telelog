use config::Config;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppSettings {
	pub telegram: TelegramSettings,
	pub filters: FiltersSettings,
}

#[derive(Debug, Deserialize)]
pub struct TelegramSettings {
	pub chat_id: String,
	pub api_key: String,
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

pub fn read_config() -> Result<AppSettings, config::ConfigError> {
	let builder = Config::builder()
		.add_source(config::File::with_name("/etc/telelog.toml"));

	match builder.build() {
		Ok(settings) => {
			settings.try_deserialize()
		},
		Err(e) => {
			println!("Error reading config: {}", e);
			Err(e)
		}
	}
}