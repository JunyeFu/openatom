#[tokio::main]
async fn main() {
    if sf_cloud_service::run().await.is_err() {
        eprintln!(
            "cloud service failed; check private configuration, database availability and schema version"
        );
        std::process::exit(1);
    }
}
