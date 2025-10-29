//! CLI Manager Module (Simplified)
//!
//! This module provides basic CLI tool management without authentication features.

use color_eyre::eyre::Result;
use console::{Term, style};

/// CLI Manager for handling CLI tool operations
pub struct CliManager {
    term: Term,
}

impl CliManager {
    /// Detect Node.js installation
    pub fn detect_nodejs(&self) -> Option<String> {
        use std::process::Command;

        match Command::new("node").arg("--version").output() {
            Ok(output) if output.status.success() => {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => None,
        }
    }
    /// Create a new CLI manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            term: Term::stdout(),
        })
    }

    /// Run the CLI management flow
    pub async fn run_management_flow(&self) -> Result<()> {
        self.show_welcome();
        self.show_main_menu().await
    }

    /// Display welcome message
    fn show_welcome(&self) {
        self.term.clear_screen().unwrap();
        println!(
            "{}",
            style("🚀 Welcome to Codex Warden CLI Manager")
                .bold()
                .cyan()
        );
        println!("{}", "=".repeat(50));
        println!("A universal AI CLI tool manager and supervisor");
        println!();
    }

    /// Show main menu and handle user selection
    async fn show_main_menu(&self) -> Result<()> {
        loop {
            use dialoguer::Select;

            let items = vec![
                "🔍 Detect and Setup CLI Tools",
                "⚙️  Configure CLI Tools",
                "🚪 Exit",
            ];

            let selection = Select::new()
                .with_prompt("What would you like to do?")
                .items(&items)
                .interact()?;

            match selection {
                0 => self.detect_and_setup_tools().await?,
                1 => self.configure_tools().await?,
                2 => {
                    println!("{}", style("👋 Goodbye!").green());
                    break;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    /// Detect and setup CLI tools
    async fn detect_and_setup_tools(&self) -> Result<()> {
        println!("\n{}", style("🔍 Detecting CLI Tools").bold().blue());
        println!("{}", "-".repeat(30));

        // Detect Node.js first
        match self.detect_nodejs() {
            Some(version) => {
                println!("✅ Node.js found: {}", version);
            }
            None => {
                println!("❌ Node.js not found");
                println!("💡 Node.js is required for most AI CLI tools");
                println!("📥 Please install Node.js from https://nodejs.org/");
            }
        }

        println!();
        dialoguer::Confirm::new()
            .with_prompt("Press Enter to continue...")
            .default(true)
            .interact()?;

        Ok(())
    }

    /// Configure CLI tools
    async fn configure_tools(&self) -> Result<()> {
        println!("\n{}", style("⚙️  CLI Configuration").bold().blue());
        println!("{}", "-".repeat(25));

        println!("ℹ️  Configuration features will be available in future versions");
        println!("📝 This will include API key management and tool preferences");
        println!();

        dialoguer::Confirm::new()
            .with_prompt("Press Enter to continue...")
            .default(true)
            .interact()?;

        Ok(())
    }
}
