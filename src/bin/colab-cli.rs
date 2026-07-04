#[tokio::main]
async fn main() {
    colab_cli::cocli::cli::dispatch::main_entry().await;
}
