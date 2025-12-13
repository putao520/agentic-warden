use aiw::commands::parser::parse_external_as_ai_cli;

fn main() {
    // 测试用例: 混合参数和prompt
    let tokens = vec![
        "claude".to_string(),
        "-p".to_string(), 
        "glm".to_string(),
        "--model".to_string(),
        "sonnet".to_string(),
        "--max-tokens".to_string(),
        "1000".to_string(),
        "--print".to_string(),
        "Explain quantum computing".to_string()
    ];
    
    match parse_external_as_ai_cli(&tokens) {
        Ok(args) => {
            println!("Selector: {}", args.selector);
            println!("Provider: {:?}", args.provider);
            println!("CLI Args: {:?}", args.cli_args);
            println!("Prompt: {:?}", args.prompt);
            println!("Prompt text: {}", args.prompt_text());
        }
        Err(e) => println!("Error: {}", e),
    }
}
