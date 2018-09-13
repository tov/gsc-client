use gsc_client::*;
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
    Auth{user: String},
    Cat{user: Option<String>, pats: Vec<RemotePattern>},
    Create{user: String},
    Deauth,
    Ls{user: Option<String>, hw: usize, pat: String},
    Passwd{user: Option<String>},
    Status{user: Option<String>, hw: usize},
}

fn do_it() -> Result<()> {
    let mut config = config::Config::new();
    config.load_dotfile()?;
    let command    = GscClientApp::new().process(&mut config)?;
    let mut client = GscClient::new(config)?;

    match command {
        Command::Auth{user}        => client.auth(&user)?,
        Command::Cat{user, pats}   => client.cat(bs(&user), &pats)?,
        Command::Create{user}      => client.create(&user)?,
        Command::Deauth            => client.deauth(),
        Command::Ls{user, hw, pat} => client.ls(bs(&user), hw, &pat)?,
        Command::Passwd{user}      => client.passwd(bs(&user))?,
        Command::Status{user, hw}  => client.status(bs(&user), hw)?,
    }

    Ok(())
}

fn bs(so: &Option<String>) -> Option<&str> {
    so.as_ref().map(String::as_str)
}

struct GscClientApp<'a: 'b, 'b>(clap::App<'a, 'b>);

fn process_common<'a>(matches: &clap::ArgMatches<'a>,
                      _config: &mut config::Config)
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
                .about("Authenticates with the server")
                .add_common()
                .arg(Arg::with_name("USER")
                    .help("The user to login as")
                    .required(true)))
            .subcommand(SubCommand::with_name("cat")
                .about("Prints remote files to stdout")
                .add_common()
                .add_user_opt("The user whose files to print")
                .arg(Arg::with_name("FILE")
                    .help("The remote files to print")
                    .required(true)
                    .multiple(true)))
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
                .add_user_opt("The user whose homework to list")
                .arg(Arg::with_name("HW")
                    .help("The homework to list, e.g. ‘hw3’")
                    .required(true)))
            .subcommand(SubCommand::with_name("passwd")
                .about("Changes the password")
                .add_common()
                .add_user_opt("The user whose password to change"))
            .subcommand(SubCommand::with_name("status")
                .about("Retrieves submission status")
                .add_common()
                .add_user_opt("The user whose homework to lookup")
                .arg(Arg::with_name("HW")
                    .help("The homework, e.g. ‘hw3’")
                    .required(true))))
    }

    fn process(self, config: &mut config::Config) -> Result<Command> {
        let matches = self.0.get_matches_safe()?;
        process_common(&matches, config);

        if let Some(submatches) = matches.subcommand_matches("auth") {
            process_common(submatches, config);
            let user = submatches.value_of("USER").unwrap().to_owned();
            Ok(Command::Auth{user})
        }

        else if let Some(submatches) = matches.subcommand_matches("cat") {
            process_common(submatches, config);
            let user     = submatches.value_of("USER").map(str::to_owned);
            let mut pats = Vec::new();

            for arg in submatches.values_of("FILE").unwrap() {
                pats.push(parse_remote_pattern(arg)?);
            }

            Ok(Command::Cat{user, pats})
        }

        else if let Some(submatches) = matches.subcommand_matches("create") {
            process_common(submatches, config);
            let user = submatches.value_of("USER").unwrap().to_owned();
            Ok(Command::Create{user})
        }

        else if let Some(_) = matches.subcommand_matches("deauth") {
            Ok(Command::Deauth)
        }

        else if let Some(submatches) = matches.subcommand_matches("ls") {
            process_common(submatches, config);
            let ls_spec = submatches.value_of("HW").unwrap();
            let user = submatches.value_of("USER").map(str::to_owned);
            let (hw, pat) = parse_hw_spec(ls_spec)?;
            Ok(Command::Ls{user, hw, pat})
        }

        else if let Some(submatches) = matches.subcommand_matches("passwd") {
            process_common(submatches, config);
            let user = submatches.value_of("USER").map(str::to_owned);
            Ok(Command::Passwd{user})
        }

        else if let Some(submatches) = matches.subcommand_matches("status") {
            process_common(submatches, config);
            let ls_spec = submatches.value_of("HW").unwrap();
            let user = submatches.value_of("USER").map(str::to_owned);
            let hw = parse_status_spec(ls_spec)?;
            Ok(Command::Status{user, hw})
        }

        else {
            Err(ErrorKind::NoCommandGiven)?
        }
    }
}

trait AppExt {
    fn add_common(self) -> Self;
    fn add_user_opt(self, about: &'static str) -> Self;
}

impl<'a, 'b> AppExt for clap::App<'a, 'b> {
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

    #[cfg(feature = "admin")]
    fn add_user_opt(self, about: &'static str) -> Self {
        use clap::*;
        self.arg(Arg::with_name("USER")
            .short("u")
            .long("user")
            .help(about)
            .takes_value(true)
            .required(false))
    }

    #[cfg(not(feature = "admin"))]
    fn add_user_opt(self, _about: &'static str) -> Self {
        self
    }
}

fn parse_hw_spec(hw_spec: &str) -> Result<(usize, String)> {
    lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"hw(\d+)(?::(.*))?").unwrap();
    }

    let captures  = RE.captures(hw_spec)
        .ok_or_else(|| ErrorKind::SyntaxError("homework spec".to_owned()))?;
    let capture1  = captures.get(1).unwrap().as_str();
    let capture2  = captures.get(2).map(|c| c.as_str());
    let hw_number = capture1.parse().unwrap();
    let pattern   = capture2.unwrap_or("").to_owned();
    Ok((hw_number, pattern))
}

fn parse_status_spec(status_spec: &str) -> Result<usize> {
    lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"hw(\d+)").unwrap();
    }

    if let Some(i) = RE.captures(status_spec)
        .and_then(|captures| captures.get(1))
        .and_then(|s| s.as_str().parse().ok()) {
        Ok(i)
    } else {
        Err(ErrorKind::SyntaxError("homework spec".to_owned()))?
    }
}

fn parse_remote_pattern(file_spec: &str) -> Result<RemotePattern> {
    lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"hw(\d+):(.+)").unwrap();
    }

    let captures  = RE.captures(file_spec)
        .ok_or_else(|| ErrorKind::SyntaxError("remote file spec".to_owned()))?;
    let capture1  = captures.get(1).unwrap().as_str();
    let capture2  = captures.get(2).unwrap().as_str();
    let hw        = capture1.parse().unwrap();
    let pat       = capture2.to_owned();
    Ok(RemotePattern{hw, pat})
}
