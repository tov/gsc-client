use gsc_client::errors::{ErrorKind, Result};

fn main() {
    vlog::set_verbosity_level(1);

    if let Err(err) = do_it() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

enum Command {
    Auth(String),
    Deauth,
}

fn do_it() -> Result<()> {
    let mut config = gsc_client::config::Config::new();
    config.load_dotfile()?;
    let command    = GscClientApp::new().process(&mut config)?;
    let mut client = gsc_client::GscClient::new(config)?;

    match command {
        Command::Auth(username) => client.auth(&username)?,
        Command::Deauth         => client.deauth(),
    }

    Ok(())
}

struct GscClientApp<'a: 'b, 'b>(clap::App<'a, 'b>);

fn process_common<'a>(matches: &clap::ArgMatches<'a>,
                      config: &mut gsc_client::config::Config)
{
    let dverbosity = matches.occurrences_of("VERBOSE") - matches.occurrences_of("QUIET");
    vlog::set_verbosity_level(dverbosity as usize + vlog::get_verbosity_level());
}

impl<'a, 'b> GscClientApp<'a, 'b> {
    fn new() -> Self {
        use clap::*;

        GscClientApp(App::new("gsc")
            .author("Jesse A. Tov <jesse@eecs.northwestern.edu>")
            .version(crate_version!())
            .add_common()
            .subcommand(SubCommand::with_name("auth")
                .about("Authenticates with the server")
                .add_common()
                .arg(Arg::with_name("USER")
                    .help("The user to login as")
                    .required(true)))
             .subcommand(SubCommand::with_name("deauth")
                 .about("Forgets authentication credentials")))
    }

    fn process(self, config: &mut gsc_client::config::Config) -> Result<Command> {
        let matches = self.0.get_matches_safe()?;
        process_common(&matches, config);

        if let Some(submatches) = matches.subcommand_matches("auth") {
            process_common(submatches, config);
            let username = submatches.value_of("USER").unwrap();
            return Ok(Command::Auth(username.to_owned()));
        }

        else if let Some(_) = matches.subcommand_matches("deauth") {
            return Ok(Command::Deauth);
        }

        Err(ErrorKind::NoCommandGiven)?
    }
}

trait CommonOptions {
    fn add_common(self) -> Self;
}

impl<'a, 'b> CommonOptions for clap::App<'a, 'b> {
    fn add_common(self) -> Self {
        use clap::*;
        
        self
            .arg(Arg::with_name("VERBOSE")
                .short("v")
                .long("verbose")
                .multiple(true)
                .takes_value(false)
                .help("Makes the output more verbose"))
            .arg(Arg::with_name("QUIET")
                .short("q")
                .long("quiet")
                .multiple(true)
                .takes_value(false)
                .help("Makes the output quieter"))
    }
}
