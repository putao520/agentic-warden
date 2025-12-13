use aiw::commands::parser::parse_external_as_ai_cli;

fn main() {
    // 测试用例1: 无值标志
    let tokens1 = vec![
        "claude".to_string(),
        "-p".to_string(), 
        "glm".to_string(),
        "--dangerously-skip-permissions".to_string(),
        "-c".to_string()
    ];
    
    println!("=== 测试用例1: 无值标志 ===");
    match parse_external_as_ai_cli(&tokens1) {
        Ok(args) => {
            println!("Selector: {}", args.selector);
            println!("Provider: {:?}", args.provider);
            println!("CLI Args: {:?}", args.cli_args);
            println!("Prompt: {:?}", args.prompt);
            println!("Is empty prompt: {}", args.prompt.is_empty());
        }
        Err(e) => println!("Error: {}", e),
    }

    println!();

    // 测试用例2: 有值参数
    let tokens2 = vec![
        "claude".to_string(),
        "-p".to_string(), 
        "glm".to_string(),
        "--model".to_string(),
        "sonnet".to_string(),
        "--max-tokens".to_string(),
        "1000".to_string(),
        "--print".to_string(),
        "Explain this".to_string()
    ];
    
    println!("=== 测试用例2: 有值参数 ===");
    match parse_external_as_ai_cli(&tokens2) {
        Ok(args) => {
            println!("Selector: {}", args.selector);
            println!("Provider: {:?}", args.provider);
            println!("CLI Args: {:?}", args.cli_args);
            println!("Prompt: {:?}", args.prompt);
        }
        Err(e) => println!("Error: {}", e),
    }
}
