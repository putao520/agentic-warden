use aiw::commands::parser::separate_provider_and_cli_args;

fn main() {
    let tokens = vec![
        "--model".to_string(),
        "sonnet".to_string(),
        "--max-tokens".to_string(), 
        "1000".to_string(),
        "--print".to_string(),
        "Explain quantum computing".to_string()
    ];
    
    println!("Testing separate_provider_and_cli_args:");
    match separate_provider_and_cli_args(&tokens) {
        Ok(args) => {
            println!("Provider: {:?}", args.provider);
            println!("CLI Args: {:?}", args.cli_args);
            println!("Prompt: {:?}", args.prompt);
        }
        Err(e) => println!("Error: {}", e),
    }
}
