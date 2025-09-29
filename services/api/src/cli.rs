use crate::demo::{run_demo, run_vacancy_report, DemoArgs, VacancyReportArgs};
use crate::server;
use clap::{Args, Parser, Subcommand};
use tenant_ai::error::AppError;

#[derive(Parser, Debug)]
#[command(
    name = "Agentic Property Orchestrator",
    about = "Demonstrate and run the Agentic Property Orchestrator from the command line",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start the HTTP service (default command)
    Serve(ServeArgs),
    /// Generate a vacancy readiness report for stakeholder demos
    Vacancy {
        #[command(subcommand)]
        command: VacancyCommand,
    },
    /// Run an end-to-end CLI demo covering vacancy and application workflows
    Demo(DemoArgs),
}

#[derive(Subcommand, Debug)]
enum VacancyCommand {
    /// Generate a vacancy workflow report and optional task listing
    Report(VacancyReportArgs),
}

#[derive(Args, Debug, Default)]
pub(crate) struct ServeArgs {
    /// Override the configured host for the HTTP server
    #[arg(long)]
    pub(crate) host: Option<String>,
    /// Override the configured port for the HTTP server
    #[arg(long)]
    pub(crate) port: Option<u16>,
}

pub(crate) async fn run() -> Result<(), AppError> {
    let cli = Cli::parse();
    let command = cli
        .command
        .unwrap_or_else(|| Command::Serve(ServeArgs::default()));

    match command {
        Command::Serve(args) => server::run(args).await,
        Command::Vacancy {
            command: VacancyCommand::Report(args),
        } => run_vacancy_report(args),
        Command::Demo(args) => run_demo(args),
    }
}
