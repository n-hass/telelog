use std::sync::Mutex;

use crate::config::AppSettings;
use crate::journal::LogEntry;
use systemd::Journal;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static!(
	static ref ALLOW_FILTERS: Mutex<Vec<FieldFilter>> = Mutex::new(vec![]);
	static ref DENY_FILTERS: Mutex<Vec<FieldFilter>> = Mutex::new(vec![]);
);

struct FieldFilter {
	field: String,
	re: Option<Regex>,
}

pub fn init(settings: &AppSettings, journal: &mut Journal) {
	let filters = &settings.filters;
	match &filters.priority {
		Some(list) => {
			let journald_field = "PRIORITY";
			for filter in list {
				let mut field = filter.value.clone();
				
				if &filter.filter_type == "pattern" {
					field = match field.to_lowercase().as_str() {
						"emerg" => "0".to_string(),
						"alert" => "1".to_string(),
						"crit" => "2".to_string(),
						"err" => "3".to_string(),
						"warning" => "4".to_string(),
						"notice" => "5".to_string(),
						"info" => "6".to_string(),
						"debug" => "7".to_string(),
						&_ => "7".to_string(),
					}
				}

				let field_int = field.parse::<u8>().unwrap();
				
				for i in 0..=field_int {
					let field = i.to_string();
					match journal.match_add(journald_field, field.as_bytes()) {
						Ok(_) => {},
						Err(e) => println!("Error adding priority filter: {}", e)
					}
				}
				
			}
		},
		None => {},
	}

	match &filters.syslog_identifier {
		Some(list) => {
			let journald_field = "SYSLOG_IDENTIFIER";
			for filter in list {

				let action = if filter.action.as_ref().is_some_and(|a| a.to_lowercase() == "deny") {
					false
				} else {
					true
				};

				if filter.filter_type == "match" {
					match journal.match_add(journald_field, filter.value.as_bytes()) {
						Ok(_) => {},
						Err(e) => println!("Error adding {} filter: {}",journald_field, e)
					}
				} 

				if filter.filter_type == "pattern" {
					match Regex::new(&filter.value) {
						Ok(re) => {
							if action {
								let mut allow = ALLOW_FILTERS.lock().unwrap();
								allow.push( 
									FieldFilter {
										field: journald_field.to_owned(),
										re: Some(re),
									}
								);
							} else {
								let mut deny = DENY_FILTERS.lock().unwrap();
								deny.push( 
									FieldFilter {
										field: journald_field.to_owned(),
										re: Some(re),
									}
								);
							}
						},
						Err(e) => println!("Error compiling regex for '{}': {}", journald_field , e),
					}
				}
			}
		},
		None => {},
	}

	match &filters.message {
		Some(list) => {
			for filter in list {
				let journald_field = "MESSAGE";
				let action: bool = if filter.action.as_ref().is_some_and(|a| a.to_lowercase() == "deny") {
					false
				} else {
					true
				};

				if filter.filter_type == "match" {
					match journal.match_add(journald_field, filter.value.as_bytes()) {
						Ok(_) => {},
						Err(e) => println!("Error adding message filter: {}", e)
					}
				} 

				if filter.filter_type == "pattern" {
					match Regex::new(&filter.value) {
						Ok(re) => {
							if action {
								let mut allow = ALLOW_FILTERS.lock().unwrap();
								allow.push( 
									FieldFilter {
										field: journald_field.to_owned(),
										re: Some(re),
									}
								);
							} else {
								let mut deny = DENY_FILTERS.lock().unwrap();
								deny.push( 
									FieldFilter {
										field: journald_field.to_owned(),
										re: Some(re),
									}
								);
							}
						},
						Err(e) => println!("Error compiling regex for '{}': {}",journald_field, e),
					}
				}
			}
		},
		None => {},
	}

}

pub fn filter_log_entry(entry: &LogEntry) -> bool {
	let allow_filters = ALLOW_FILTERS.lock().unwrap();
	let deny_filters = DENY_FILTERS.lock().unwrap();
	
	
	for filter in deny_filters.iter() {
		let filter_field = filter.field.as_str();
		if filter.re.is_some() {
			let re = filter.re.as_ref().unwrap();
			let entry_field_value = match entry.get_copy(filter_field) {
				Ok(field) => field,
				Err(e) => {
					println!("Error getting field '{}': {}", filter_field, e);
					continue;
				}
			};

			if re.is_match(&entry_field_value) {
				return true;
			}
		}
	}
	
	for filter in allow_filters.iter() {
		let filter_field = filter.field.as_str();
		if filter.re.is_some() {
			let re = filter.re.as_ref().unwrap();
			let entry_field_value = match entry.get_copy(filter_field) {
				Ok(field) => field,
				Err(e) => {
					println!("Error getting field '{}': {}", filter_field, e);
					continue;
				}
			};

			if re.is_match(&entry_field_value) {
				return false;
			}
		}
	}
	
	false
}