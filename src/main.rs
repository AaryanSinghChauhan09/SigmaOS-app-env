//! CLI for sigmaos-app-env.

use clap::{Parser, Subcommand};
use sigmaos_app_env::{builtin_profiles, default_store_path, ProfileStore};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sigmaos-app-env", about = "SigmaOS app environment manager")]
struct Cli {
    #[arg(long, global = true)]
    store: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List profiles.
    List,
    /// Show a profile's launch spec.
    Show { id: String },
    /// Add a profile from JSON file.
    Add { file: PathBuf },
    /// Remove a profile.
    Remove { id: String },
    /// Print store path.
    Path,
}

fn store_path(s: &Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    s.clone()
        .or_else(default_store_path)
        .ok_or_else(|| "cannot determine store path".into())
}

fn load(s: &Option<PathBuf>) -> Result<ProfileStore, Box<dyn std::error::Error>> {
    let p = store_path(s)?;
    if p.exists() {
        Ok(ProfileStore::load(&p)?)
    } else {
        Ok(builtin_profiles())
    }
}

fn save(st: &ProfileStore, path: &Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    st.save(&store_path(path)?)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();
    let mut store = load(&cli.store)?;
    match cli.command {
        Commands::List => {
            for p in store.list() {
                println!("{} — {} ({})", p.id, p.name, p.command.join(" "));
            }
        }
        Commands::Show { id } => {
            let spec = store.render_launch(&id)?;
            println!("{}", spec.to_shell());
        }
        Commands::Add { file } => {
            let text = std::fs::read_to_string(file)?;
            let p: sigmaos_app_env::AppProfile = serde_json::from_str(&text)?;
            store.add(p)?;
            save(&store, &cli.store)?;
        }
        Commands::Remove { id } => {
            store.remove(&id)?;
            save(&store, &cli.store)?;
            println!("removed {id}");
        }
        Commands::Path => println!("{}", store_path(&cli.store)?.display()),
    }
    Ok(())
}
