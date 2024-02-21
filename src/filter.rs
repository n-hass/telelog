use std::borrow::Borrow;
use std::sync::OnceLock;

use crate::config::{AppSettings, RuleLogic, RuleValue};
use crate::journal::LogEntry;
use systemd::Journal;
use regex::Regex;

static RULESET: OnceLock<RuleSet> = OnceLock::new();


#[derive(Debug, PartialEq)]
enum RuleAction {
	Allow,
	Deny,
}

#[derive(Debug, Clone)]
struct RuleField {
	field: String,
	re: Vec<Regex>,
	logic: RuleLogic,
}

#[derive(Debug)]
struct RuleGroup {
	priority: u32,
	action: RuleAction,
	rules: Vec<RuleField>,
}

#[derive(Debug)]
struct RuleSet {
	filters: Vec<RuleGroup>,
}

impl RuleSet {
	pub fn new() -> Self {
		RuleSet {
			filters: Vec::new(),
		}
	}

	pub fn add(&mut self, filter: RuleGroup) {
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
				if filter.action == RuleAction::Deny {
					insert_index = i;
					break;
				}

				// if this is an allow filter, find the next allow filter and insert before it
				for (j, f) in self.filters.iter().enumerate().skip(i) {
					if f.action == RuleAction::Allow || priority < f.priority { 
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

	// pub fn get(&self) -> &Vec<RuleGroup> {
	// 	&self.filters
	// }
}

pub fn init(settings: &AppSettings, journal: &mut Journal) {
	let mut partial_rule_set = RuleSet::new();

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
								if rule.logic == RuleLogic::All {
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
			
			let new_rule_list: Vec<RuleField> = rules.iter().filter_map(|rule| {
				match &rule.value {
					RuleValue::Single(value) => {
						let re = Regex::new(value);
						if re.is_err() {
							println!("[filter init] Error compiling regex for '{}': {}", rule.field, re.err().unwrap());
							return None;
						}
						let re = re.unwrap();

						Some(RuleField {
							field: rule.field.clone(),
							re: vec![re],
							logic: RuleLogic::Any, // single value rules dont really matter what the logical op is
						})
					},
					RuleValue::Multiple(values) => {

						let mut compiled_list = Vec::<Regex>::new();
						for value in values.iter() {
							let re = Regex::new(value);
							if re.is_err() {
								println!("[filter init] Error compiling regex for '{}': {}", rule.field, re.err().unwrap());
								continue;
							}
							let re = re.unwrap();
							compiled_list.push(re);
						}
						Some(RuleField {
							field: rule.field.clone(),
							re: compiled_list,
							logic: rule.logic.clone(),
						})
					},
				}
			}).collect();

			let new_rule_group = RuleGroup {
				priority: *priority,
				action: RuleAction::Deny,
				rules: new_rule_list,
			};
			partial_rule_set.add(new_rule_group);
		}
	}

	if let Some(rule_groups) = &settings.allow_rules {
		for (priority,rules) in rule_groups.iter() {
			
			let new_rule_list: Vec<RuleField> = rules.iter().filter_map(|rule| {
				match &rule.value {
					RuleValue::Single(value) => {
						let re = Regex::new(value);
						if re.is_err() {
							println!("[filter init] Error compiling regex for '{}': {}", rule.field, re.err().unwrap());
							return None;
						}
						let re = re.unwrap();

						Some(RuleField {
							field: rule.field.clone(),
							re: vec![re],
							logic: RuleLogic::Any, // single value rules dont really matter what the logical op is
						})
					},
					RuleValue::Multiple(values) => {

						let mut compiled_list = Vec::<Regex>::new();
						for value in values.iter() {
							let re = Regex::new(value);
							if re.is_err() {
								println!("[filter init] Error compiling regex for '{}': {}", rule.field, re.err().unwrap());
								continue;
							}
							let re = re.unwrap();
							compiled_list.push(re);
						}
						Some(RuleField {
							field: rule.field.clone(),
							re: compiled_list,
							logic: rule.logic.clone(),
						})
					},
				}
			}).collect();

			let new_rule_group = RuleGroup {
				priority: *priority,
				action: RuleAction::Allow,
				rules: new_rule_list,
			};
			partial_rule_set.add(new_rule_group);
		}
	}

	RULESET.set(partial_rule_set).expect("Initialisation occurs once");
}

/// Returns true if a log should be filtered (ignored), returns false if it should be processed
pub fn filter_log_entry(entry: &LogEntry) -> bool {
	let ruleset: &Vec<RuleGroup> = RULESET.get().unwrap().filters.borrow();
	
	for rule_group in ruleset.iter() {
		let group_is_match = rule_group.rules.iter().all(|rule| { // when multiple rules are specified in a group, they are always ANDed together
			let log_field = match entry.get_field(&rule.field) {
				Ok(v) => v,
				Err(e) => {
					println!("[filter_log_entry] Error getting field {}: {}", &rule.field, e);
					return false;
				},
			};

			let matched = match rule.logic {
				RuleLogic::Any => rule.re.iter().any(|re| re.is_match(&log_field)),
				RuleLogic::All => rule.re.iter().all(|re| re.is_match(&log_field)),
			};

			return matched
		});

		if group_is_match {
			return match rule_group.action {
				RuleAction::Allow => false,
				RuleAction::Deny => true,
			}
		}
	}
	
	false // if no rules match, allow the log through by default
}