use fabric_sdk::gateway::client::ClientBuilder;
use fabric_sdk::identity::IdentityBuilder;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use once_cell::sync::Lazy;
use fabric_sdk::error::SubmitError;

const MSP_ID: &str = "Org1MSP";
const CERT_PATH: &str = "../../test-network/organizations/peerOrganizations/org1.example.com/users/User1@org1.example.com/msp/signcerts";
const KEY_PATH: &str = "../../test-network/organizations/peerOrganizations/org1.example.com/users/User1@org1.example.com/msp/keystore";
const TLS_CERT_PATH: &str = "../../test-network/organizations/peerOrganizations/org1.example.com/peers/peer0.org1.example.com/tls/ca.crt";
const PEER_ENDPOINT: &str = "localhost:7051";

static ASSET_ID: Lazy<String> = Lazy::new(|| generate_asset_id());

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = new_gateway_connection().await?;
    let chaincode_name = std::env::var("CHAINCODE_NAME").unwrap_or_else(|_| "basic".to_string());
    let channel_name = std::env::var("CHANNEL_NAME").unwrap_or_else(|_| "mychannel".to_string());

    init_ledger(&client, &channel_name, &chaincode_name).await?;
    get_all_assets(&client, &channel_name, &chaincode_name).await?;
    create_asset(&client, &channel_name, &chaincode_name).await?;
    read_asset_by_id(&client, &channel_name, &chaincode_name).await?;
    transfer_asset_async(&client, &channel_name, &chaincode_name).await?;
    example_error_handling(&client, &channel_name, &chaincode_name).await?;
    Ok(())
}

async fn new_gateway_connection() -> Result<fabric_sdk::gateway::client::Client, Box<dyn Error>> {
    let tls_cert = fs::read(TLS_CERT_PATH)?;
    let pem_bytes = read_first_file(CERT_PATH)?;
    let private_key_bytes = read_first_file(KEY_PATH)?;

    let identity = IdentityBuilder::from_pem(&pem_bytes)?
        .with_msp(MSP_ID)?
        .with_private_key(private_key_bytes)?
        .build()?;

    let mut client = ClientBuilder::new()
        .with_identity(identity)?
        .with_tls(tls_cert)?
        .with_scheme("https")?
        .with_authority(PEER_ENDPOINT)?
        .build()?;

    client.connect().await?;

    Ok(client)
}

fn read_first_file(dir_path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut entries = fs::read_dir(dir_path)?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return Err(format!("No files found in directory: {}", dir_path).into());
    }

    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let file_path = entries[0].path();
    Ok(fs::read(file_path)?)
}

async fn init_ledger(
    client: &fabric_sdk::gateway::client::Client,
    channel_name: &str,
    chaincode_name: &str,
) -> Result<(), Box<dyn Error>> {
    println!(
        "\n--> Submit Transaction: InitLedger, function creates the initial set of assets on the ledger"
    );

    let tx_builder = client
        .get_chaincode_call_builder()
        .with_channel_name(channel_name)?
        .with_chaincode_id(chaincode_name)?
        .with_function_name("InitLedger")?
        .build()?;

    match client.submit_chaincode_call(tx_builder).await {
        Ok(result) => {
            println!("{}", String::from_utf8_lossy(result.as_slice()));
            println!("*** Transaction committed successfully");
        }
        Err(err) => println!("{}", err),
    }
    Ok(())
}

async fn get_all_assets(
    client: &fabric_sdk::gateway::client::Client,
    channel_name: &str,
    chaincode_name: &str,
) -> Result<(), Box<dyn Error>> {
    println!(
        "\n--> Evaluate Transaction: GetAllAssets, function returns all the current assets on the ledger"
    );

    let tx_builder = client
        .get_chaincode_call_builder()
        .with_channel_name(channel_name)?
        .with_chaincode_id(chaincode_name)?
        .with_function_name("GetAllAssets")?
        .build()?;

    match client.peek_chaincode_call(tx_builder).await {
        Ok(result) => {
            let result_str = String::from_utf8_lossy(&result);
            if result_str.is_empty() {
                println!("*** No assets found");
                return Ok(());
            }
            let formatted_json = format_json(&result_str);
            match formatted_json {
                Ok(json) => println!("*** Result:{}", json),
                Err(_) => println!("*** Result:{}", result_str),
            }
            Ok(())
        }
        Err(err) => {
            eprintln!("Failed to evaluate transaction: {:?}", err);
            Err(err.into())
        }
    }
}

fn format_json(data: &str) -> Result<String, Box<dyn Error>> {
    let parsed: Value = serde_json::from_str(data)?;
    Ok(serde_json::to_string_pretty(&parsed)?)
}

async fn create_asset(
    client: &fabric_sdk::gateway::client::Client,
    channel_name: &str,
    chaincode_name: &str,
) -> Result<(), Box<dyn Error>> {
    println!(
        "\n--> Submit Transaction: CreateAsset, creates new asset with ID, Color, Size, Owner and AppraisedValue arguments"
    );

    let tx_builder = client
        .get_chaincode_call_builder()
        .with_channel_name(channel_name)?
        .with_chaincode_id(chaincode_name)?
        .with_function_name("CreateAsset")?
        .with_function_args([&ASSET_ID, "yellow", "5", "Tom", "1300"])?
        .build()?;

    match client.submit_chaincode_call(tx_builder).await {
        Ok(_) => {
            println!("*** Transaction committed successfully");
            Ok(())
        }
        Err(err) => {
            eprintln!("Failed to submit transaction: {:?}", err);
            Err(err.into())
        }
    }
}

fn generate_asset_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    format!("asset{}", now.as_millis())
}


async fn read_asset_by_id(
    client: &fabric_sdk::gateway::client::Client,
    channel_name: &str,
    chaincode_name: &str,
) -> Result<(), Box<dyn Error>> {
    println!("\n--> Evaluate Transaction: ReadAsset, function returns asset attributes");

    let tx_builder = client
        .get_chaincode_call_builder()
        .with_channel_name(channel_name)?
        .with_chaincode_id(chaincode_name)?
        .with_function_name("ReadAsset")?
        .with_function_args([&ASSET_ID.to_string()])?
        .build()?;

    match client.peek_chaincode_call(tx_builder).await {
        Ok(result) => {
            let result_str = String::from_utf8_lossy(&result);
            if result_str.is_empty() {
                println!("*** Asset not found");
                return Ok(());
            }
            let formatted_json = format_json(&result_str);
            match formatted_json {
                Ok(json) => println!("*** Result:{}", json),
                Err(_) => println!("*** Result:{}", result_str),
            }
            Ok(())
        }
        Err(err) => {
            eprintln!("Failed to evaluate transaction: {:?}", err);
            Err(err.into())
        }
    }
}


async fn transfer_asset_async(
    client: &fabric_sdk::gateway::client::Client,
    channel_name: &str,
    chaincode_name: &str,
) -> Result<(), Box<dyn Error>> {
    println!("\n--> Async Submit Transaction: TransferAsset, updates existing asset owner");

    // Use a known asset ID that should exist after CreateAsset
    let asset_id = "asset1".to_string();

    let tx_builder = client
        .get_chaincode_call_builder()
        .with_channel_name(channel_name)?
        .with_chaincode_id(chaincode_name)?
        .with_function_name("TransferAsset")?
        .with_function_args([&asset_id, "Mark"])?
        .build()?;

    match client.submit_chaincode_async_call(tx_builder).await {
        Ok(result) => {
            println!(
                "\n*** Successfully submitted transaction to transfer ownership from {} to Mark.",
                String::from_utf8_lossy(&result.result)
            );
            println!("*** Transaction submitted asynchronously. Checking commit status...");

            match  client.commit_status(result.txn_id.clone(), channel_name.to_string()).await {
                Ok(result) => {
                    println!(
                        "Commit result: {} block num: {}",
                        result.result, result.block_number
                    );
                }
                Err(err) => {
                    eprintln!("Failed to get commit status: {:?}", err);
                }
            }
            println!("*** Transaction committed successfully");
            Ok(())
        }
        Err(err) => {
            eprintln!("Failed to submit transaction asynchronously: {:?}", err);
            Err(err.into())
        }
    }
}

async fn example_error_handling(
    client: &fabric_sdk::gateway::client::Client,
    channel_name: &str,
    chaincode_name: &str,
) -> Result<(), Box<dyn Error>> {
    println!(
        "\n--> Submit Transaction: UpdateAsset asset70, asset70 does not exist and should return an error"
    );

    let tx_builder = client
        .get_chaincode_call_builder()
        .with_channel_name(channel_name)?
        .with_chaincode_id(chaincode_name)?
        .with_function_name("UpdateAsset")?
        .with_function_args(["asset70", "blue", "5", "Tomoko", "300"])?
        .build()?;

    match client.submit_chaincode_call(tx_builder).await {
        Ok(_) => {
            panic!("******** FAILED to return an error");
        }
        Err(err) => {
            println!("*** Successfully caught the error:");
            match err {
                SubmitError::NodeError(msg) => {
                    println!("endorse error: {}", msg);
                }
                _ => {
                    println!("Unexpected error: {:?}", err);
                }
            }
            Ok(())
        }
    }
}