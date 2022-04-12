// Simple worker that sends app-started telemetry request to the backend then exits
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut header = Default::default();
    let telemetry = telemetry::build_full(&mut header).await;

    println!(
        "Payload to be sent: {}",
        serde_json::to_string_pretty(&telemetry).unwrap()
    );

    telemetry::push_telemetry(&telemetry).await?;

    println!("Telemetry submitted correctly");
    Ok(())
}
