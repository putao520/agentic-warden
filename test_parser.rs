use aiw::commands::parser::separate_provider_and_cli_args;

fn main() {
    let tokens = vec![
        "claude".to_string(),
        "-p".to_string(), 
        "glm".to_string(),
        "--dangerously-skip-permissions".to_string(),
        "-c".to_string()
    ];
    
    match separate_provider_and_cli_args(&tokens) {
        Ok(args) => {
            println!("Provider: {:?}", args.provider);
            println!("CLI Args: {:?}", args.cli_args);
            println!("Prompt: {:?}", args.prompt);
        }
        Err(e) => println!("Error: {}", e),
    }
}
