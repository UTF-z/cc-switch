use crate::panic_hook;
use crate::services::ProviderService;
use crate::store::AppState;
use args::{Cli, Commands};
use clap::Parser;
use std::str::FromStr;
use std::sync::Arc;

pub mod args {
    use clap::{Parser, Subcommand};

    #[derive(Parser, Debug)]
    #[command(name = "cc-switch")]
    #[command(version, about = "CC Switch - CLI mode for headless servers")]
    pub struct Cli {
        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Provider management
        Provider {
            #[command(subcommand)]
            command: ProviderCommands,
        },
        /// MCP server management
        Mcp {
            #[command(subcommand)]
            command: McpCommands,
        },
        /// Skill management
        Skill {
            #[command(subcommand)]
            command: SkillCommands,
        },
        /// Prompt management
        Prompt {
            #[command(subcommand)]
            command: PromptCommands,
        },
        /// Proxy control
        Proxy {
            #[command(subcommand)]
            command: ProxyCommands,
        },
        /// Database and config info
        Info,
    }

    #[derive(Subcommand, Debug)]
    pub enum ProviderCommands {
        /// List all providers
        List {
            /// Filter by app type (claude, codex, gemini, opencode, openclaw)
            #[arg(short, long)]
            app: Option<String>,
        },
        /// Get current active provider
        Current {
            /// App type
            #[arg(short, long)]
            app: Option<String>,
        },
        /// Switch to a provider
        Switch {
            /// App type (claude, codex, gemini, opencode, openclaw)
            #[arg(short, long)]
            app: String,
            /// Provider ID to switch to
            #[arg(short, long)]
            provider: String,
        },
        /// Add a provider from JSON file
        Add {
            /// App type (claude, codex, gemini, opencode, openclaw)
            #[arg(short, long)]
            app: String,
            /// Provider config JSON file
            #[arg(short = 'f', long)]
            file: Option<String>,
            /// Provider name
            #[arg(short = 'n', long)]
            name: Option<String>,
            /// API Key
            #[arg(long)]
            api_key: Option<String>,
            /// Base URL (endpoint)
            #[arg(short = 'u', long)]
            base_url: Option<String>,
            /// Model name (e.g., MiniMax-M2.7, claude-3-5-sonnet-20241022)
            #[arg(short = 'm', long)]
            model: Option<String>,
        },
    }

    #[derive(Subcommand, Debug)]
    pub enum McpCommands {
        /// List all MCP servers
        List {
            /// Filter by app type
            #[arg(short, long)]
            app: Option<String>,
        },
    }

    #[derive(Subcommand, Debug)]
    pub enum SkillCommands {
        /// List all installed skills
        List,
        /// Discover available skills from repos
        Discover,
    }

    #[derive(Subcommand, Debug)]
    pub enum PromptCommands {
        /// List prompts
        List {
            /// App type
            #[arg(short, long)]
            app: Option<String>,
        },
    }

    #[derive(Subcommand, Debug)]
    pub enum ProxyCommands {
        /// Start the proxy server
        Start {
            /// Port to listen on
            #[arg(short, long, default_value = "8080")]
            port: u16,
        },
        /// Stop the proxy server
        Stop,
        /// Get proxy status
        Status,
    }
}

use crate::app_config::AppType;
use crate::config::get_app_config_dir;
use crate::database::Database;

/// Initialize headless mode: setup panic hook, logging, database, and app state
fn init_headless() -> Result<AppState, String> {
    // Setup panic hook
    panic_hook::setup_panic_hook();

    // Initialize simple logging to stdout
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("Initializing CC Switch in headless mode...");

    // Initialize database
    let app_config_dir = get_app_config_dir();
    let db_path = app_config_dir.join("cc-switch.db");

    let db = Database::init().map_err(|e| format!("Failed to init database: {e}"))?;
    let db = Arc::new(db);

    log::info!("Database initialized at {:?}", db_path);

    // Create AppState
    let app_state = AppState::new(db);

    Ok(app_state)
}

pub async fn run_cli() -> Result<(), String> {
    // Filter out --headless and -x flags before parsing with clap
    // These flags are handled in main.rs but clap also sees them
    let args: Vec<String> = std::env::args()
        .filter(|arg| arg != "--headless" && arg != "-x")
        .collect();
    let cli = Cli::parse_from(args.into_iter());

    match &cli.command {
        Commands::Info {} => {
            let app_config_dir = get_app_config_dir();
            println!("CC Switch Headless CLI");
            println!("Version: {}", env!("CARGO_PKG_VERSION"));
            println!("Config dir: {:?}", app_config_dir);
            println!("DB: {:?}", app_config_dir.join("cc-switch.db"));
        }
        Commands::Provider { command } => {
            let state = init_headless()?;
            match command {
                args::ProviderCommands::List { app } => {
                    let app_types: Vec<&str> = if let Some(ref a) = app {
                        vec![a.as_str()]
                    } else {
                        vec!["claude", "codex", "gemini", "opencode", "openclaw"]
                    };

                    for at in app_types {
                        let providers = state.db.get_all_providers(at)?;

                        // Get current provider ID for this app type
                        let current_id = state.db.get_current_provider(at)?;

                        if providers.is_empty() {
                            if app.is_some() {
                                println!("No providers found for {}.", at);
                            }
                            continue;
                        }

                        println!("\n[{}] Providers:", at);
                        for (id, provider) in providers {
                            let current_mark = if current_id.as_ref() == Some(&id) {
                                " [CURRENT]"
                            } else {
                                ""
                            };
                            println!(
                                "  {} ({}){}",
                                provider.name,
                                id,
                                current_mark
                            );
                        }
                    }
                }
                args::ProviderCommands::Current { app } => {
                    let app_types: Vec<&str> = if let Some(ref a) = app {
                        vec![a.as_str()]
                    } else {
                        vec!["claude", "codex", "gemini", "opencode", "openclaw"]
                    };

                    for at in app_types {
                        if let Ok(Some(current_id)) = state.db.get_current_provider(at) {
                            if let Ok(Some(provider)) = state.db.get_provider_by_id(&current_id, at) {
                                println!("{}: {} ({})", at, provider.name, current_id);
                            } else {
                                println!("{}: {} (id: {})", at, "[provider not found]", current_id);
                            }
                        } else {
                            if app.is_some() {
                                println!("No current provider for {}", at);
                            }
                        }
                    }
                }
                args::ProviderCommands::Switch { app, provider } => {
                    let app_type = AppType::from_str(app)
                        .map_err(|e| format!("Invalid app type: {}", e))?;

                    println!("Switching {} to provider {}...", app, provider);
                    match ProviderService::switch(&state, app_type, provider) {
                        Ok(result) => {
                            println!("Switch successful!");
                            if !result.warnings.is_empty() {
                                println!("Warnings:");
                                for warning in &result.warnings {
                                    println!("  - {}", warning);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Switch failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                args::ProviderCommands::Add { app, file, name, api_key, base_url, model } => {
                    let app_type = AppType::from_str(app)
                        .map_err(|e| format!("Invalid app type: {}", e))?;

                    let provider = if let Some(file_path) = file {
                        // Read provider config from JSON file
                        let content = std::fs::read_to_string(file_path)
                            .map_err(|e| format!("Failed to read file: {}", e))?;
                        serde_json::from_str(&content)
                            .map_err(|e| format!("Failed to parse JSON: {}", e))?
                    } else if name.is_some() || api_key.is_some() || base_url.is_some() {
                        // Build simple config from arguments
                        let name = name.clone().ok_or("Name is required when not using a config file")?;
                        let api_key = api_key.clone().ok_or("API key is required when not using a config file")?;
                        let base_url = base_url.clone().unwrap_or_else(|| {
                            match app_type.as_str() {
                                "claude" => "https://api.minimaxi.com/anthropic".to_string(),
                                "codex" => "https://api.openai.com".to_string(),
                                "gemini" => "https://generativelanguage.googleapis.com".to_string(),
                                _ => "".to_string(),
                            }
                        });
                        let model = model.clone().unwrap_or_else(|| {
                            match app_type.as_str() {
                                "claude" => "MiniMax-M2.7".to_string(),
                                "codex" => "gpt-4o".to_string(),
                                "gemini" => "gemini-2.5-pro".to_string(),
                                _ => "".to_string(),
                            }
                        });

                        // Build a Claude-compatible config with env variables
                        let settings = serde_json::json!({
                            "env": {
                                "ANTHROPIC_BASE_URL": base_url,
                                "ANTHROPIC_AUTH_TOKEN": api_key,
                                "ANTHROPIC_MODEL": model,
                                "ANTHROPIC_DEFAULT_SONNET_MODEL": model,
                                "ANTHROPIC_DEFAULT_OPUS_MODEL": model,
                                "ANTHROPIC_DEFAULT_HAIKU_MODEL": model,
                            },
                            "includeCoAuthoredBy": false,
                            "permissions": { "defaultMode": "acceptEdits" },
                            "language": "en"
                        });

                        crate::provider::Provider {
                            id: uuid::Uuid::new_v4().to_string(),
                            name,
                            settings_config: settings,
                            website_url: None,
                            category: Some("custom".to_string()),
                            created_at: None,
                            sort_index: None,
                            notes: None,
                            icon: None,
                            icon_color: None,
                            meta: Default::default(),
                            in_failover_queue: false,
                        }
                    } else {
                        return Err("Either --file or --name/--api-key must be provided".to_string());
                    };

                    match ProviderService::add(&state, app_type, provider, true) {
                        Ok(_) => {
                            println!("Provider added successfully!");
                        }
                        Err(e) => {
                            eprintln!("Failed to add provider: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        Commands::Mcp { command } => {
            let state = init_headless()?;
            match command {
                args::McpCommands::List { app: _ } => {
                    let servers = state.db.get_all_mcp_servers()
                        .map_err(|e| format!("Failed to get MCP servers: {}", e))?;

                    if servers.is_empty() {
                        println!("No MCP servers found.");
                    } else {
                        println!("MCP Servers:");
                        for (id, server) in servers {
                            // Try to extract command from server JSON
                            let cmd = server.server.get("command")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            println!(
                                "  {} - {} ({})",
                                server.name,
                                cmd,
                                id
                            );
                        }
                    }
                }
            }
        }
        Commands::Skill { command } => {
            let state = init_headless()?;
            match command {
                args::SkillCommands::List => {
                    let skills = state.db.get_all_installed_skills()
                        .map_err(|e| format!("Failed to get skills: {}", e))?;

                    if skills.is_empty() {
                        println!("No skills installed.");
                    } else {
                        println!("Installed Skills:");
                        for (id, skill) in skills {
                            println!("  {} ({})", skill.name, id);
                        }
                    }
                }
                args::SkillCommands::Discover => {
                    println!("Discovering skills...");
                    println!("Not yet implemented");
                }
            }
        }
        Commands::Prompt { command } => {
            let state = init_headless()?;
            match command {
                args::PromptCommands::List { app } => {
                    let app_types: Vec<&str> = if let Some(ref a) = app {
                        vec![a.as_str()]
                    } else {
                        vec!["claude", "codex", "gemini", "opencode", "openclaw"]
                    };

                    for at in app_types {
                        let prompts = state.db.get_prompts(at)
                            .map_err(|e| format!("Failed to get prompts: {}", e))?;

                        if prompts.is_empty() {
                            if app.is_some() {
                                println!("No prompts found for {}.", at);
                            }
                            continue;
                        }

                        println!("\n[{}] Prompts:", at);
                        for (id, prompt) in prompts {
                            println!("  {} ({})", prompt.name, id);
                        }
                    }
                }
            }
        }
        Commands::Proxy { command } => {
            let _state = init_headless()?;
            match command {
                args::ProxyCommands::Start { port } => {
                    println!("Starting proxy on port {}...", port);
                    println!("Not yet implemented");
                }
                args::ProxyCommands::Stop => {
                    println!("Stopping proxy...");
                    println!("Not yet implemented");
                }
                args::ProxyCommands::Status => {
                    println!("Proxy status:");
                    println!("Not yet implemented");
                }
            }
        }
    }

    Ok(())
}
