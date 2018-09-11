use gsc_client;
use gsc_client::errors::Result;

fn main() -> Result<()> {
    let client = gsc_client::GscClient::new()?;
    client.login()?;
    Ok(())
}

