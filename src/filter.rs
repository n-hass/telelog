use std::sync::Mutex;

use crate::{config::AppSettings};
use crate::journal::LogEntry;
use systemd::Journal;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static!(
	static ref FILTERS: Mutex<Vec<FieldFilter>> = Mutex::new(vec![]);
);

struct FieldFilter {
	field: String,
	val: String,
	re: Option<Regex>,
	action: bool
}

pub fn init(settings: &AppSettings, journal: &mut Journal) {
	let filters = &settings.filters;
	match &filters.priority {
		Some(list) => {
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
					match journal.match_add("PRIORITY", field.as_bytes()) {
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
			for filter in list {

				let action = match &filter.action {
					Some(a) => {
						match a.to_lowercase().as_str() {
							"allow" => true,
							"deny" => false,
							_ => true,
						}
					},
					None => true,
				};

				if filter.filter_type == "match" {
					match journal.match_add("SYSLOG_IDENTIFIER", filter.value.as_bytes()) {
						Ok(_) => {},
						Err(e) => println!("Error adding syslog_identifier filter: {}", e)
					}
				} 

				if filter.filter_type == "pattern" {
					match Regex::new(&filter.value) {
						Ok(re) => {
							let mut filters = FILTERS.lock().unwrap();
							filters.push( 
								FieldFilter {
									field: "SYSLOG_IDENTIFIER".to_owned(),
									val: filter.value.clone(),
									re: Some(re),
									action: action,
								}
							);
						},
						Err(e) => println!("Error compiling regex: {}", e),
					}
				}
			}
		},
		None => {},
	}

	match &filters.message {
		Some(list) => {
			for filter in list {
				
				let action = match &filter.action {
					Some(a) => {
						match a.to_lowercase().as_str() {
							"allow" => true,
							"deny" => false,
							_ => true,
						}
					},
					None => true,
				};

				if filter.filter_type == "match" {
					match journal.match_add("MESSAGE", filter.value.as_bytes()) {
						Ok(_) => {},
						Err(e) => println!("Error adding message filter: {}", e)
					}
				} 

				if filter.filter_type == "pattern" {
					match Regex::new(&filter.value) {
						Ok(re) => {
							let mut filters = FILTERS.lock().unwrap();
							filters.push( 
								FieldFilter {
									field: "MESSAGE".to_owned(),
									val: filter.value.clone(),
									re: Some(re),
									action: action,
								}
							);
						},
						Err(e) => println!("Error compiling regex: {}", e),
					}
				}
			}
		},
		None => {},
	}

}

pub fn filter_log_entry(entry: &LogEntry) -> bool {
	let filters = FILTERS.lock().unwrap();
	let (true_filters, false_filters): (Vec<_>, Vec<_>) = filters.iter().partition(|filter| filter.action);
	
	for filter in false_filters {
		match filter.field.as_str() {
			"SYSLOG_IDENTIFIER" => {
				if filter.re.is_some() {
					let re = filter.re.as_ref().unwrap();
					if re.is_match(&entry.identifier) {
						return true;
					}
				}
			}
			"MESSAGE" => {
				if filter.re.is_some() {
					let re = filter.re.as_ref().unwrap();
					if re.is_match(&entry.message) {
						return true;
					}
				}
			}
			_ => {},
		}
	}
	
	for filter in true_filters {
		match filter.field.as_str() {
			"SYSLOG_IDENTIFIER" => {
				if filter.re.is_some() {
					let re = filter.re.as_ref().unwrap();
					if re.is_match(&entry.identifier) {
						return false;
					}
				}
			}
			"MESSAGE" => {
				if filter.re.is_some() {
					let re = filter.re.as_ref().unwrap();
					if re.is_match(&entry.message) {
						return false;
					}
				}
			}
			_ => {},
		}
	}
	
	false
}