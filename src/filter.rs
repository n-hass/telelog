use std::sync::OnceLock;

use crate::config::{AppSettings, RuleType, RuleValue};
use crate::journal::LogEntry;
use systemd::Journal;
use regex::Regex;

static FILTERS: OnceLock<FilterSet> = OnceLock::new();


#[derive(Debug, PartialEq)]
enum FilterAction {
	Allow,
	Deny,
}

#[derive(Debug)]
struct FieldFilter {
	field: String,
	re: Vec<Regex>,
	logic: RuleType,
	action: FilterAction,
	priority: u32,
}

#[derive(Debug)]
struct FilterSet {
	filters: Vec<FieldFilter>,
}

impl FilterSet {
	pub fn new() -> Self {
		FilterSet {
			filters: Vec::new(),
		}
	}

	pub fn add(&mut self, filter: FieldFilter) {
		// when inserting must maintain priority order from lowest num to highest num
		// if priority is the same, sory by filter.action. true (deny) should come before false (allow)

		if self.filters.is_empty() {
			self.filters.push(filter);
			return;
		}
		let priority = filter.priority;
		let mut insert_index = 0;
		for (i, f) in self.filters.iter().enumerate() {
			if priority < f.priority {
				insert_index = i;
				break;
			} else if priority == f.priority {
				// if this filter is a deny filter, insert it before the other filters of the same priority
				if filter.action == FilterAction::Deny {
					insert_index = i;
					break;
				}

				// if this is an allow filter, find the next allow filter and insert before it
				for (j, f) in self.filters.iter().enumerate().skip(i) {
					if f.action == FilterAction::Allow || priority < f.priority { 
						insert_index = j;
						break;
					}
				}

				break
			} else {
				insert_index = i + 1;
			}
		}

		self.filters.insert(insert_index, filter);
	}

	pub fn get(&self) -> &Vec<FieldFilter> {
		&self.filters
	}
}

pub fn init(settings: &AppSettings, journal: &mut Journal) {
	let mut temp_filters = FilterSet::new();

	if let Some(rule_groups) = &settings.match_rules {
		let mut group_iter = rule_groups.iter().peekable();
		while let Some((_priority,rules)) = group_iter.next() {
			let mut rules_iter = rules.iter().peekable();
			while let Some(rule) = rules_iter.next() {
				let journald_field = rule.field.as_str();

				match &rule.value {
					RuleValue::Single(value) => {
						match journal.match_add(journald_field, value.as_bytes()) {
							Ok(_) => {},
							Err(e) => println!("[filter init] Error adding {} filter: {}", journald_field, e)
						}
					},
					RuleValue::Multiple(values) => {
						let mut values_iter = values.iter().peekable();

						while let Some(value) = values_iter.next() {
							match journal.match_add(journald_field, value.as_bytes()) {
								Ok(_) => {},
								Err(e) => println!("[filter init] Error adding {} filter: {}", journald_field, e)
							}

							if values_iter.peek().is_some() {
								if rule.logic == RuleType::All {
									match journal.match_and() {
										Ok(_) => {},
										Err(e) => println!("[filter init] Error adding match AND filter: {}", e)
									}
								} else {
									match journal.match_or() {
										Ok(_) => {},
										Err(e) => println!("[filter init] Error adding match OR filter: {}", e)
									}
								}
							}
						}
					},
				}

				// When multiple rules are specified in a group, they are always ANDed together
				if rules_iter.peek().is_some() {
					match journal.match_and() {
						Ok(_) => {},
						Err(e) => println!("[filter init] Error adding AND filter: {}", e)
					}
				}
			}

			// When multiple groups are specified, they are always ORed together
			if group_iter.peek().is_some() {
				match journal.match_or() {
					Ok(_) => {},
					Err(e) => println!("[filter init] Error adding OR filter: {}", e)
				}
			}
		}
	}

	if let Some(rule_groups) = &settings.deny_rules {
		for (priority,rules) in rule_groups.iter() {
			for rule in rules.iter() {

				match &rule.value {
					RuleValue::Single(value) => {
						let re = Regex::new(value);
						if re.is_err() {
							println!("[filter init] Error compiling regex for '{}': {}", rule.field, re.err().unwrap());
							continue;
						}
						let re = re.unwrap();

						temp_filters.add(
							FieldFilter {
								field: rule.field.clone(),
								re: vec![re],
								logic: rule.logic.clone(),
								action: FilterAction::Deny,
								priority: *priority,
							}
						);

					},
					RuleValue::Multiple(values) => {
						let mut patterns = Vec::<Regex>::new();
						for value in values.iter() {
							let re = Regex::new(value);
							if re.is_err() {
								println!("[filter init] Error compiling regex for '{}': {}", rule.field, re.err().unwrap());
								continue;
							}
							let re = re.unwrap();
							patterns.push(re);
						}

						temp_filters.add(
							FieldFilter {
								field: rule.field.clone(),
								re: patterns,
								logic: rule.logic.clone(),
								action: FilterAction::Deny,
								priority: *priority,
							}
						);
					},
				}

			}
		}
	}

	if let Some(rule_groups) = &settings.allow_rules {
		for (priority,rules) in rule_groups.iter() {
			for rule in rules.iter() {

				match &rule.value {
					RuleValue::Single(value) => {
						let re = Regex::new(value);
						if re.is_err() {
							println!("[filter init] Error compiling regex for '{}': {}", rule.field, re.err().unwrap());
							continue;
						}
						let re = re.unwrap();

						temp_filters.add(
							FieldFilter {
								field: rule.field.clone(),
								re: vec![re],
								logic: rule.logic.clone(),
								action: FilterAction::Allow,
								priority: *priority,
							}
						);

					},
					RuleValue::Multiple(values) => {
						let mut patterns = Vec::<Regex>::new();
						for value in values.iter() {
							let re = Regex::new(value);
							if re.is_err() {
								println!("[filter init] Error compiling regex for '{}': {}", rule.field, re.err().unwrap());
								continue;
							}
							let re = re.unwrap();
							patterns.push(re);
						}

						temp_filters.add(
							FieldFilter {
								field: rule.field.clone(),
								re: patterns,
								logic: rule.logic.clone(),
								action: FilterAction::Allow,
								priority: *priority,
							}
						);
					},
				}

			}
		}
	}

	FILTERS.set(temp_filters).expect("Initialisation occurs once");
}

/// Returns true if a log should be filtered (ignored), returns false if it should be processed
pub fn filter_log_entry(entry: &LogEntry) -> bool {
	
	
	false
}