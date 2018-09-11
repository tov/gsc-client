use gsc_client;
use gsc_client::errors::Result;

fn main() -> Result<()> {
    vlog::set_verbosity_level(3);

    let mut client = gsc_client::GscClient::new()?;
//    client.logout();
//    let result = client.login("jtov")?;
    let result = client.get_users()?;
    println!("{}", result);
    Ok(())
}

