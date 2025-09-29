mod cli;
mod demo;
mod infra;
mod routes;
mod server;

use tenant_ai::error::AppError;

pub async fn run() -> Result<(), AppError> {
    cli::run().await
}
