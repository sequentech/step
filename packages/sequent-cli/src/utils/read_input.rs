use inquire::{Select, Text};
use std::{
    default,
    fmt::{Debug, Display},
    io::{self, Write},
};

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

pub fn auto_complete_text_prompt(message: &str, default: Option<&str>) -> String {
    let text_prompt = if let Some(default_value) = default {
        Text::new(message).with_default(default_value).prompt()
    } else {
        Text::new(message).prompt()
    };

    match text_prompt {
        Ok(input) => input,
        Err(_) => String::new(),
    }
}

pub fn select_option_prompt<T: Clone + Debug + Display>(message: &str, options: &[T]) -> T {
    let selection = Select::new(message, options.to_vec()).prompt();

    match selection {
        Ok(selected) => selected,
        Err(_) => options[0].clone(),
    }
}
