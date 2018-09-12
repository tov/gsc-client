use gsc_client::errors::{ErrorKind, Result};
use lazy_static::lazy_static;

fn main() {
    vlog::set_verbosity_level(1);

    if let Err(err) = do_it() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

enum Command {
    Auth(String),
    Create(String),
    Deauth,
    Ls(Option<String>, usize),
    Status(Option<String>, usize),
}

fn do_it() -> Result<()> {
    let mut config = gsc_client::config::Config::new();
    config.load_dotfile()?;
    let command    = GscClientApp::new().process(&mut config)?;
    let mut client = gsc_client::GscClient::new(config)?;

    match command {
        Command::Auth(username)   => client.auth(&username)?,
        Command::Create(username) => client.create(&username)?,
        Command::Deauth           => client.deauth(),
        Command::Ls(user, hw)     => client.ls_submission(user, hw)?,
        Command::Status(user, hw) => client.status(user, hw)?,
    }

    Ok(())
}

struct GscClientApp<'a: 'b, 'b>(clap::App<'a, 'b>);

fn process_common<'a>(matches: &clap::ArgMatches<'a>,
                      _config: &mut gsc_client::config::Config)
{
    let vs = matches.occurrences_of("VERBOSE") as usize;
    let qs = matches.occurrences_of("QUIET") as usize;
    let verbosity = if qs > vs { 0 } else { vlog::get_verbosity_level() + vs - qs };
    vlog::set_verbosity_level(verbosity);
}

impl<'a, 'b> GscClientApp<'a, 'b> {
    fn new() -> Self {
        use clap::*;

        GscClientApp(App::new("gsc")
            .author("Jesse A. Tov <jesse@eecs.northwestern.edu>")
            .about("Command-line interface to the GSC server")
            .version(crate_version!())
            .add_common()
            .subcommand(SubCommand::with_name("auth")
                .name("auth")
                .about("Authenticates with the server")
                .add_common()
                .arg(Arg::with_name("USER")
                    .help("The user to login as")
                    .required(true)))
            .subcommand(SubCommand::with_name("create")
                .about("Creates a new account")
                .add_common()
                .arg(Arg::with_name("USER")
                    .help("The new account’s username")
                    .required(true)))
            .subcommand(SubCommand::with_name("deauth")
                .about("Forgets authentication credentials"))
            .subcommand(SubCommand::with_name("ls")
                .about("Lists files")
                .add_common()
                .arg(Arg::with_name("USER")
                    .long("user")
                    .short("u")
                    .help("The user whose homework to list")
                    .takes_value(true)
                    .required(false))
                .arg(Arg::with_name("HW")
                    .help("The homework to list, e.g. ‘hw3’")
                    .required(true)))
            .subcommand(SubCommand::with_name("status")
                .about("Retrieves submission status")
                .add_common()
                .arg(Arg::with_name("USER")
                    .long("user")
                    .short("u")
                    .help("The user whose homework to lookup")
                    .takes_value(true)
                    .required(false))
                .arg(Arg::with_name("HW")
                    .help("The homework, e.g. ‘hw3’")
                    .required(true))))
    }

    fn process(self, config: &mut gsc_client::config::Config) -> Result<Command> {
        let matches = self.0.get_matches_safe()?;
        process_common(&matches, config);

        if let Some(submatches) = matches.subcommand_matches("auth") {
            process_common(submatches, config);
            let username = submatches.value_of("USER").unwrap();
            Ok(Command::Auth(username.to_owned()))
        }

        else if let Some(submatches) = matches.subcommand_matches("create") {
            process_common(submatches, config);
            let username = submatches.value_of("USER").unwrap();
            Ok(Command::Create(username.to_owned()))
        }

        else if let Some(_) = matches.subcommand_matches("deauth") {
            Ok(Command::Deauth)
        }

        else if let Some(submatches) = matches.subcommand_matches("ls") {
            process_common(submatches, config);
            let ls_spec = submatches.value_of("HW").unwrap();
            let user = submatches.value_of("USER").map(str::to_owned);
            Ok(Command::Ls(user, parse_hw_spec(ls_spec)?))
        }

        else if let Some(submatches) = matches.subcommand_matches("status") {
            process_common(submatches, config);
            let ls_spec = submatches.value_of("HW").unwrap();
            let user = submatches.value_of("USER").map(str::to_owned);
            Ok(Command::Status(user, parse_status_spec(ls_spec)?))
        }

        else {
            Err(ErrorKind::NoCommandGiven)?
        }
    }
}

fn parse_hw_spec(hw_spec: &str) -> Result<usize> {
    lazy_static! {
        static ref HW_RE: regex::Regex = regex::Regex::new(r"hw(\d):?").unwrap();
    }

    if let Some(i) = HW_RE.captures(hw_spec)
        .and_then(|captures| captures.get(1))
        .and_then(|s| s.as_str().parse().ok()) {
        Ok(i)
    } else {
        Err(ErrorKind::SyntaxError("homework spec".to_owned()))?
    }
}

fn parse_status_spec(status_spec: &str) -> Result<usize> {
    lazy_static! {
        static ref HW_RE: regex::Regex = regex::Regex::new(r"hw(\d)").unwrap();
    }

    if let Some(i) = HW_RE.captures(status_spec)
        .and_then(|captures| captures.get(1))
        .and_then(|s| s.as_str().parse().ok()) {
        Ok(i)
    } else {
        Err(ErrorKind::SyntaxError("homework spec".to_owned()))?
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
