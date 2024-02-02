use crate::journal::LogEntry;

pub fn generate_messages(buffer: &Vec<LogEntry>) -> Vec<String> {
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

/// Flatten multiple message lists of sparse length to a single list of messages at maximum length
pub fn flatten_messages(message_lists: [&Vec<String>; 2]) -> Vec<String> {
	// if two or more messages can be concatenated and still be within the size limit, do it
	let mut flattened_messages: Vec<String> = Vec::new();
	
	for list in message_lists {
		for message in list {
			if flattened_messages.len() == 0 {
				flattened_messages.push(message.clone());
				continue
			}

			if flattened_messages.last().unwrap().len() + message.len() < 4108 {
				let mut previous_message: String = flattened_messages.pop().unwrap();
				
				previous_message = previous_message
					.strip_suffix("</code>")
					.expect("last message did not end with </code>")
					.to_string();
				
				previous_message.push_str(message.strip_prefix("<code>").unwrap_or(""));
				flattened_messages.push(previous_message);
			}
		}
	}

	return flattened_messages
}

pub fn colour_translate(priority: u8) -> String {
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