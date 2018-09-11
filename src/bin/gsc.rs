use gsc_client;
use gsc_client::errors::Result;

fn main() {
    vlog::set_verbosity_level(3);

    if let Err(err) = do_it() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn do_it() -> Result<()> {
    let mut client = gsc_client::GscClient::new()?;
//    let result = client.get_users()?;
    let result = client.login("jtov")?;
    println!("{}", result);
    Ok(())
}
