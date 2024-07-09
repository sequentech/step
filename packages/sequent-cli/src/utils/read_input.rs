use std::io::{self, Write};

pub fn prompt(message: &str, required: bool) -> String {
    loop {
        print!("{}", message);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");
        let input = input.trim().to_string();

        if !required || !input.is_empty() {
            return input;
        }

        println!("This field is required and cannot be blank.");
    }
}
