pub mod controls;
pub mod models;
pub mod ui;

use anyhow::Result;
use clap::Parser;
use controls::run_app;
use indicatif::{ProgressBar, ProgressStyle};
use models::{
    app_state::AppState, args::Args, config::Config, issue::Issue, menu_items::MenuItems,
};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use std::{io, time::Duration};

use crossterm::{
    style::Stylize,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

async fn fetch_issues(client: &reqwest::Client, config: &Config) -> Result<Vec<Issue>> {
    Ok(client
        .get("https://api.github.com/issues")
        .header(
            AUTHORIZATION,
            format!("Bearer {}", &config.github_access_token),
        )
        .header(ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header(USER_AGENT, &config.user_name)
        .send()
        .await?
        .json::<Vec<Issue>>()
        .await?)
}

fn create_spinner(message: String) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(Duration::from_millis(120));
    spinner.set_style(
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "-"]),
    );
    spinner.set_message(message);

    spinner
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();
    let args = Args::parse();

    let config = Config::initialise_config(Config {
        github_access_token: args.token.unwrap_or(String::new()),
        user_name: args.user_name.unwrap_or(String::new()),
    });

    if args.file_path {
        eprintln!(
            "{:?}",
            confy::get_configuration_file_path("issue-tracker", None).unwrap()
        );
        reset_terminal().unwrap_or_else(|_| panic!("Failed to reset terminal"));
        std::process::exit(1);
    }

    let spinner = create_spinner(String::from("Fetching issues.."));
    let issues = fetch_issues(&client, &config).await?;
    spinner.finish();

    let mut terminal = init_terminal()?;

    let app_state = AppState::new(issues);
    let res = run_app(&mut terminal, app_state);

    reset_terminal()?;

    if let Err(err) = res {
        eprintln!("{}: {}", "Error".red().bold(), err);
        reset_terminal().unwrap_or_else(|_| panic!("Failed to reset terminal"));
        std::process::exit(1);
    }

    Ok(())
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(io::stdout());

    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let original_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic| {
        reset_terminal().unwrap_or_else(|_| panic!("Failed to reset terminal"));
        original_hook(panic);
    }));

    Ok(terminal)
}

fn reset_terminal() -> Result<()> {
    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}
