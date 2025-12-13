use aiw::commands::parser::parse_external_as_ai_cli;

fn main() {
    let tokens = vec![
        "claude".to_string(),
        "-p".to_string(), 
        "glm".to_string(),
        "--dangerously-skip-permissions".to_string(),
        "-c".to_string()
    ];
    
    match parse_external_as_ai_cli(&tokens) {
        Ok(args) => {
            println!("Selector: {}", args.selector);
            println!("Provider: {:?}", args.provider);
            println!("CLI Args: {:?}", args.cli_args);
            println!("Prompt: {:?}", args.prompt);
            println!("Is empty prompt: {}", args.prompt.is_empty());
        }
        Err(e) => println!("Error: {}", e),
    }
}
