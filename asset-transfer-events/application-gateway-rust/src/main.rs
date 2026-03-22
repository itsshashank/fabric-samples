mod connect;
use fabric_sdk::gateway::client::Client;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{error::Error, sync::Arc};

static ASSET_ID: Lazy<String> = Lazy::new(|| generate_asset_id());
static CHAINCODE_NAME: Lazy<String> =
    Lazy::new(|| std::env::var("CHAINCODE_NAME").unwrap_or_else(|_| "events".to_string()));

static CHANNEL_NAME: Lazy<String> =
    Lazy::new(|| std::env::var("CHANNEL_NAME").unwrap_or_else(|_| "mychannel".to_string()));

fn generate_asset_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    format!("asset{}", now.as_millis())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Arc::new(connect::new_gateway_connection().await?);

    let event_client = Arc::clone(&client);
    let event_handle = tokio::spawn(async move {
        if let Err(e) = listen_for_events(&event_client).await {
            eprintln!("Event listener failed: {}", e);
        }
    });

    let first_block_number = create_asset(&client, &ASSET_ID).await?;
    println!("First block number: {}", first_block_number);
    update_asset(&client, &ASSET_ID).await?;
    transfer_asset(&client, &ASSET_ID).await?;
    delete_asset(&client, &ASSET_ID).await?;

    replay_chaincode_events(&client, first_block_number).await?;

    event_handle.abort();
    Ok(())
}

async fn listen_for_events(client: &Client) -> Result<(), Box<dyn Error>> {
    let request = client
        .get_chaincode_events_request_builder()
        .with_channel_id(CHANNEL_NAME.to_string())?
        .with_chaincode_id(CHAINCODE_NAME.to_string())?
        .build()?;

    let mut stream = client.chaincode_events(request).await?;

    while let Some(response) = stream.next().await {
        let response = match response {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Error receiving chaincode event: {}", e);
                break;
            }
        };

        for event in response.events {
            let payload = format_json(&event.payload);
            println!(
                "\n<-- Chaincode event received: {} - {}",
                event.event_name, payload
            );
        }
    }

    Ok(())
}

fn format_json(data: &[u8]) -> String {
    match serde_json::from_slice::<Value>(data) {
        Ok(value) => serde_json::to_string_pretty(&value)
            .unwrap_or_else(|_| String::from_utf8_lossy(data).to_string()),
        Err(_) => String::from_utf8_lossy(data).to_string(),
    }
}

async fn create_asset(client: &Client, asset_id: &str) -> Result<u64, Box<dyn Error>> {
    println!(
        "\n--> Submit transaction: CreateAsset, {} owned by Sam with appraised value 100",
        asset_id
    );

    let tx_builder = client
        .get_chaincode_call_builder()
        .with_channel_name(CHANNEL_NAME.to_string())?
        .with_chaincode_id(CHAINCODE_NAME.to_string())?
        .with_function_name("CreateAsset")?
        .with_function_args([asset_id, "blue", "10", "Sam", "100"])?
        .build()?;

    let response = client.submit_chaincode_async_call(tx_builder).await?;
    println!("{}", String::from_utf8_lossy(&response.result));
    let commit_status = client
        .commit_status(response.txn_id.clone(), CHANNEL_NAME.to_string())
        .await?;

    println!("*** CreateAsset committed successfully");
    Ok(commit_status.block_number)
}

async fn update_asset(client: &Client, asset_id: &str) -> Result<(), Box<dyn Error>> {
    println!(
        "\n--> Submit transaction: UpdateAsset, {} update appraised value to 200",
        asset_id
    );

    let tx_builder = client
        .get_chaincode_call_builder()
        .with_channel_name(CHANNEL_NAME.to_string())?
        .with_chaincode_id(CHAINCODE_NAME.to_string())?
        .with_function_name("UpdateAsset")?
        .with_function_args([asset_id, "blue", "10", "Sam", "200"])?
        .build()?;

    let result = client.submit_chaincode_call(tx_builder).await?;
    println!("{}", String::from_utf8_lossy(&result));
    println!("\n*** UpdateAsset committed successfully");

    Ok(())
}

async fn transfer_asset(client: &Arc<Client>, asset_id: &str) -> Result<(), Box<dyn Error>> {
    println!(
        "\n--> Submit transaction: TransferAsset, {} to Mary",
        asset_id
    );

    let tx_builder = client
        .get_chaincode_call_builder()
        .with_channel_name(CHANNEL_NAME.to_string())?
        .with_chaincode_id(CHAINCODE_NAME.to_string())?
        .with_function_name("TransferAsset")?
        .with_function_args([asset_id, "Mary"])?
        .build()?;

    let result = client.submit_chaincode_call(tx_builder).await?;
    println!("{}", String::from_utf8_lossy(&result));
    println!("\n*** TransferAsset committed successfully");

    Ok(())
}

async fn delete_asset(client: &Arc<Client>, asset_id: &str) -> Result<(), Box<dyn Error>> {
    println!("\n--> Submit transaction: DeleteAsset, {}", asset_id);

    let tx_builder = client
        .get_chaincode_call_builder()
        .with_channel_name(CHANNEL_NAME.to_string())?
        .with_chaincode_id(CHAINCODE_NAME.to_string())?
        .with_function_name("DeleteAsset")?
        .with_function_args([asset_id])?
        .build()?;

    let result = client.submit_chaincode_call(tx_builder).await?;
    println!("{}", String::from_utf8_lossy(&result));
    println!("\n*** DeleteAsset committed successfully");

    Ok(())
}

async fn replay_chaincode_events(
    client: &Arc<Client>,
    start_block: u64,
) -> Result<(), Box<dyn Error>> {
    println!("\n*** Start chaincode event replay");

    let request = client
        .get_chaincode_events_request_builder()
        .with_channel_id(CHANNEL_NAME.to_string())?
        .with_chaincode_id(CHAINCODE_NAME.to_string())?
        .with_start_block(start_block)
        .build()?;

    let mut stream = client.chaincode_events(request).await?;

    while let Some(response) = stream.next().await {
        let response = match response {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Error receiving chaincode event: {}", e);
                break;
            }
        };

        for event in response.events {
            let payload = format_json(&event.payload);
            println!(
                "\n<-- Chaincode event replayed: {} - {}",
                event.event_name, payload
            );
            // Break when we reach the DeleteAsset event
            if event.event_name == "DeleteAsset" {
                return Ok(());
            }
        }
    }

    Ok(())
}
