use std::env;

mod src {
    pub mod commands {
        pub mod parser {
            pub use aiw::commands::parser::{separate_provider_and_cli_args, SeparatedArgs};
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <cmd...>", args[0]);
        return;
    }

    let tokens = &args[1..];
    println!("Input tokens: {:?}", tokens);

    match src::commands::parser::separate_provider_and_cli_args(tokens) {
        Ok(separated) => {
            println!("Separated args: {:?}", separated);
            println!("Provider: {:?}", separated.provider);
            println!("CLI args: {:?}", separated.cli_args);
            println!("Prompt: {:?}", separated.prompt);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}