mod journal;
use journal::open_journal_tail;

mod parser;
use parser::print_a_message;


fn main() {

	let mut j = open_journal_tail();

	loop { 
		match j.wait(None) {
			Ok(_) => {
				
				print_a_message(j.next_entry());
				loop {
					let next = j.next_entry();
					match next {
						Ok(Some(_)) => {
							// print!("another: ");
							print_a_message(next);
						},
						Ok(None) => {
							// println!("Last of batch reached");
							break;
						},
						Err(e) => {
							println!("Error in multi batch: {}", e);
							break;
						},
					}
				}
			},
			Err(_) => println!("Timeout"),
		}
	}
}
