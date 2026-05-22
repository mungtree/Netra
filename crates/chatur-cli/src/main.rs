//! `chatur` — the headless command-line interface for Mini ChatUR.
//!
//! A thin shell over the [`chatur_api`] library: it proves the library is
//! fully usable without the Tauri front-end, and serves as an end-to-end
//! harness.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use clap::{Parser, Subcommand};

use chatur_api::chatur_core::ids::ProjectId;
use chatur_api::{Chatur, ChaturConfig};

/// Mini ChatUR headless CLI.
#[derive(Parser)]
#[command(name = "chatur", version, about = "Mini ChatUR — headless CLI")]
struct Cli {
    /// Path to the configuration file (TOML); defaults are used if absent.
    #[arg(long, default_value = "chatur.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage registered projects.
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// Queue a job and return immediately.
    Queue {
        /// Target project id.
        project: ProjectId,
        /// Prompt text (all trailing words are joined).
        #[arg(required = true)]
        prompt: Vec<String>,
    },
    /// Queue a job and wait for it to finish, printing the result.
    Run {
        /// Target project id.
        project: ProjectId,
        /// Prompt text (all trailing words are joined).
        #[arg(required = true)]
        prompt: Vec<String>,
    },
    /// List the jobs of a project.
    Jobs {
        /// Project id.
        project: ProjectId,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// Register a project.
    Add {
        /// Display name.
        name: String,
        /// Path to the project root.
        path: PathBuf,
    },
    /// List registered projects.
    List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config = ChaturConfig::load_or_default(&cli.config)
        .with_context(|| format!("loading config from {}", cli.config.display()))?;
    let chatur = Chatur::start(config).await.context("starting Chatur")?;

    let result = dispatch(&chatur, cli.command).await;
    chatur.shutdown().await.context("shutting down Chatur")?;
    result
}

/// Executes one CLI command.
async fn dispatch(chatur: &Chatur, command: Command) -> anyhow::Result<()> {
    match command {
        Command::Project { action } => project(chatur, action).await?,
        Command::Queue { project, prompt } => {
            let id = chatur.queue_job(project, prompt.join(" ")).await?;
            println!("job queued: {id}");
        }
        Command::Run { project, prompt } => {
            let id = chatur.queue_job(project, prompt.join(" ")).await?;
            println!("job {id} running...");
            let job = chatur.wait_for_job(id, Duration::from_secs(600)).await?;
            println!("status: {:?}", job.status);
            if let Some(output) = job.output {
                println!("--- output ---\n{}", output.text);
            }
        }
        Command::Jobs { project } => {
            let jobs = chatur.list_jobs(project).await?;
            if jobs.is_empty() {
                println!("no jobs");
            }
            for job in jobs {
                println!("{}  {:?}  {}", job.id, job.status, job.prompt);
            }
        }
    }
    Ok(())
}

/// Executes a `project` sub-command.
async fn project(chatur: &Chatur, action: ProjectAction) -> anyhow::Result<()> {
    match action {
        ProjectAction::Add { name, path } => {
            let id = chatur.add_project(name, path).await?;
            println!("project added: {id}");
        }
        ProjectAction::List => {
            let projects = chatur.list_projects().await?;
            if projects.is_empty() {
                println!("no projects");
            }
            for project in projects {
                println!(
                    "{}  {}  {}",
                    project.id,
                    project.name,
                    project.root_path.display()
                );
            }
        }
    }
    Ok(())
}
