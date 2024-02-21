use std::{collections::HashMap, fmt, path::PathBuf};
use serde::{de::{self, Error, MapAccess, Visitor}, Deserialize, Deserializer};

use clap::{arg, command, value_parser};

#[derive(Debug, Deserialize)]
pub struct AppSettings {
	pub telegram: TelegramSettings,
	#[serde(rename = "match", deserialize_with = "deserialize_rule_group")]
    pub match_rules: Option<HashMap<u32, Vec<Rule>>>,
	#[serde(rename = "deny", deserialize_with = "deserialize_rule_group")]
    pub deny_rules: Option<HashMap<u32, Vec<Rule>>>,
	#[serde(rename = "allow", deserialize_with = "deserialize_rule_group")]
    pub allow_rules: Option<HashMap<u32, Vec<Rule>>>,
}

impl Default for AppSettings {
	fn default() -> Self {
		AppSettings {
			telegram: TelegramSettings {
				chat_id: "".to_string(),
				api_key: None,
				flush_seconds: None,
			},
			match_rules: Some(HashMap::new()),
			deny_rules: Some(HashMap::new()),
			allow_rules: Some(HashMap::new()),
		}
	}
}

#[derive(Debug, Deserialize)]
pub struct TelegramSettings {
	pub chat_id: String,
	pub api_key: Option<String>,
	pub flush_seconds: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    pub field: String,
    pub value: RuleValue,
    #[serde(rename = "rule", default = "Rule::default_rule")]
    pub logic: RuleLogic,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RuleValue {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize, Clone, PartialEq, Copy)]
pub enum RuleLogic {
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "all")]
    All,
}

impl Rule {
    fn default_rule() -> RuleLogic {
        RuleLogic::Any
    }
}

fn deserialize_rule_group<'de, D>(deserializer: D) -> Result<Option<HashMap<u32, Vec<Rule>>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct RuleGroupVisitor;

    impl<'de> Visitor<'de> for RuleGroupVisitor {
        type Value = Option<HashMap<u32, Vec<Rule>>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map with string keys and list of rules as values")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut map = HashMap::new();
            while let Some(key) = access.next_key::<String>()? {
                let parsed_key = (&key).parse::<u32>().map_err(de::Error::custom)?;
                let rules = match access.next_value()? {
                    Some(value) => match value {
                        toml::Value::Array(seq) => {
                            seq.into_iter().map(|val: toml::Value| {
                                Rule::deserialize(val).map_err(de::Error::custom)
                            }).collect::<Result<Vec<_>, _>>()?
                        },
                        _ => vec![Rule::deserialize(value).map_err(de::Error::custom)?],
                    },
                    None => continue,
                };
                map.insert(parsed_key, rules);
            }
            Ok(Some(map))
        }
    }

    deserializer.deserialize_option(RuleGroupVisitor)
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

pub fn read_config(filepath: &str) -> Result<AppSettings, toml::de::Error> {

	let config_str = std::fs::read_to_string(filepath).unwrap();
	let config: Result<AppSettings, toml::de::Error> = toml::from_str(&config_str);
	if config.is_err() {
		return config;
	}

	let mut settings: AppSettings = config.unwrap();

	if settings.telegram.api_key.is_none() {
		match get_environment_variable("TELEGRAM_API_KEY") {
			Some(api_key) => {
				settings.telegram.api_key = Some(api_key);
			},
			None => {
				return Err(toml::de::Error::missing_field("[config] No API key set in file as telegram.api_key, and no TELEGRAM_API_KEY environment variable"))
			}
		}
	}

	if settings.telegram.flush_seconds.is_none() {
		settings.telegram.flush_seconds = Some(5);
	}

	return Ok(settings);
}