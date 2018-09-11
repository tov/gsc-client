use gsc_client;
use gsc_client::errors::Result;

fn main() -> Result<()> {
    let mut client = gsc_client::GscClient::new()?;
//    let result = client.login()?;
    let result = client.get_users()?;
    println!("{}", result);
    Ok(())
}

