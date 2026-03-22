use std::fs;
use std::error::Error;
use fabric_sdk::gateway::client::ClientBuilder;
use fabric_sdk::identity::IdentityBuilder;

const MSP_ID: &str = "Org1MSP";
const CERT_PATH: &str = "../../test-network/organizations/peerOrganizations/org1.example.com/users/User1@org1.example.com/msp/signcerts";
const KEY_PATH: &str = "../../test-network/organizations/peerOrganizations/org1.example.com/users/User1@org1.example.com/msp/keystore";
const TLS_CERT_PATH: &str = "../../test-network/organizations/peerOrganizations/org1.example.com/peers/peer0.org1.example.com/tls/ca.crt";
const PEER_ENDPOINT: &str = "localhost:7051";

pub async fn new_gateway_connection() -> Result<fabric_sdk::gateway::client::Client, Box<dyn Error>> {
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