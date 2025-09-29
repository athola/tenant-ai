use tenant_ai_api::run;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("application error: {err}");
        std::process::exit(1);
    }
}
