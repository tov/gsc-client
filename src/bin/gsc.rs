use gsc_client::*;
use gsc_client::errors::{Result, ErrorKind, syntax_error};
use std::process::exit;

fn main() {
    vlog::set_verbosity_level(1);

    match do_it() {
        Err(err)  => {
            eprintln!("{}", err);
            exit(1);
        }
        Ok(true)  => exit(2),
        Ok(false) => (),
    }
}

enum Command {
    AdminExtend{user: String, hw: usize, date: String, eval: bool},
    AdminSetExam{user: String, exam: usize, num: usize, den: usize},
    AdminSubmissions{hw: usize},
    Auth{user: String},
    Cat{user: Option<String>, rpats: Vec<RemotePattern>},
    Create{user: String},
    Cp{user: Option<String>, srcs: Vec<CpArg>, dst: CpArg},
    Deauth,
    Ls{user: Option<String>, rpat: RemotePattern},
    Partner{me: Option<String>},
    PartnerRequest{me: Option<String>, hw: usize, them: String},
    PartnerAccept{me: Option<String>, hw: usize, them: String},
    PartnerCancel{me: Option<String>, hw: usize, them: String},
    Passwd{user: Option<String>},
    Rm{user: Option<String>, rpats: Vec<RemotePattern>},
    Status{user: Option<String>, hw: Option<usize>},
    Whoami,
}

fn do_it() -> Result<bool> {
    let mut config = config::Config::new();
    config.load_dotfile()?;
    let command    = GscClientApp::new().process(&mut config)?;
    let mut client = GscClient::new(config)?;

    use self::Command::*;

    match command {
        AdminExtend{user, hw, date, eval}
                                     => client.admin_extend(&user, hw, &date, eval),
        AdminSetExam{user, exam, num, den}
                                     => client.admin_set_exam(&user, exam, num, den),
        AdminSubmissions{hw}         => client.admin_submissions(hw),
        Auth{user}                   => client.auth(&user),
        Cat{user, rpats}             => client.cat(bs(&user), &rpats),
        Create{user}                 => client.create(&user),
        Cp{user, srcs, dst}          => client.cp(bs(&user), &srcs, &dst),
        Deauth                       => client.deauth(),
        Ls{user, rpat}               => client.ls(bs(&user), &rpat),
        Partner{me}                  => client.partner(bs(&me)),
        PartnerRequest{me, hw, them} => client.partner_request(bs(&me), hw, &them),
        PartnerAccept{me, hw, them}  => client.partner_accept(bs(&me), hw, &them),
        PartnerCancel{me, hw, them}  => client.partner_cancel(bs(&me), hw, &them),
        Passwd{user}                 => client.passwd(bs(&user)),
        Rm{user, rpats}              => client.rm(bs(&user), &rpats),
        Status{user, hw: Some(i)}    => client.status_hw(bs(&user), i),
        Status{user, hw: None}       => client.status_user(bs(&user)),
        Whoami                       => client.whoami(),
    }?;

    Ok(client.had_warning())
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
            .add_admin()
            .subcommand(SubCommand::with_name("auth")
                .about("Authenticates with the server")
                .add_common()
                .req_arg("USER" ,"Your username (i.e., your NetID)"))
            .subcommand(SubCommand::with_name("cat")
                .about("Prints remote files to stdout")
                .add_common()
                .add_user_opt("The user whose files to print")
                .req_arg("FILE", "The remote files to print"))
            .subcommand(SubCommand::with_name("cp")
                .about("Copies files to or from the server")
                .add_common()
                .add_user_opt("The user whose files to access")
                .flag("ALL", "all", "Copy all the files in the specified source homework(s)")
                .req_args("SRC", "The file(s) to copy")
                .req_arg("DST", "The destination of the file(s)"))
            .subcommand(SubCommand::with_name("create")
                .about("Creates a new account")
                .add_common()
                .req_arg("USER", "The new account’s username (i.e., your NetID)"))
            .subcommand(SubCommand::with_name("deauth")
                .about("Forgets authentication credentials")
                .add_common())
            .subcommand(SubCommand::with_name("ls")
                .about("Lists files")
                .add_common()
                .add_user_opt("The user whose homework to list")
                .req_arg("SPEC", "The homework or file(s) to list, e.g. ‘hw3’"))
            .subcommand(SubCommand::with_name("partner")
                .about("Manages partners")
                .add_common()
                .add_user_opt("The user whose partners to manage")
                .subcommand(SubCommand::with_name("request")
                    .about("Sends a partner request")
                    .add_partner_args())
                .subcommand(SubCommand::with_name("accept")
                    .about("Accepts a partner request")
                    .add_partner_args())
                .subcommand(SubCommand::with_name("cancel")
                    .about("Cancels a partner request")
                    .add_partner_args()))
            .subcommand(SubCommand::with_name("passwd")
                .about("Changes the password")
                .add_common()
                .add_user_opt("The user whose password to change"))
            .subcommand(SubCommand::with_name("rm")
                .about("Removes remote files")
                .add_common()
                .add_user_opt("The user whose files to remove")
                .flag("ALL", "all", "Remove all the files in the specified homework")
                .req_args("FILE", "The remote files to remove"))
            .subcommand(SubCommand::with_name("status")
                .about("Retrieves user or submission status")
                .add_common()
                .add_user_opt("The user whose status check")
                .opt_arg("HW", "The homework to lookup, e.g. ‘hw3’"))
            .subcommand(SubCommand::with_name("whoami")
                .about("Prints your username, if authenticated")
                .add_common()))
    }

    fn process(self, config: &mut config::Config) -> Result<Command> {
        let matches = self.0.get_matches_safe()?;
        process_common(&matches, config);

        if let Some(submatches) = matches.subcommand_matches("admin") {
            process_common(submatches, config);

            if let Some(subsubmatches) = submatches.subcommand_matches("extend") {
                process_common(subsubmatches, config);
                let eval = subsubmatches.is_present("EVAL");
                let hw   = parse_hw(subsubmatches.value_of("HW").unwrap())?;
                let user = subsubmatches.value_of("USER").unwrap().to_owned();
                let date = subsubmatches.value_of("DATESPEC").unwrap().to_owned();
                Ok(Command::AdminExtend { hw, user, date, eval })
            } else if let Some(subsubmatches) = submatches.subcommand_matches("set_exam") {
                process_common(subsubmatches, config);
                let exam = subsubmatches.value_of("EXAM").unwrap().parse()?;
                let user = subsubmatches.value_of("USER").unwrap().to_owned();
                let num  = subsubmatches.value_of("POINTS").unwrap().parse()?;
                let den  = subsubmatches.value_of("POSSIBLE").unwrap().parse()?;
                Ok(Command::AdminSetExam { user, exam, num, den })
            } else if let Some(subsubmatches) = submatches.subcommand_matches("submissions") {
                process_common(subsubmatches, config);
                let hw   = parse_hw(subsubmatches.value_of("HW").unwrap())?;
                Ok(Command::AdminSubmissions{hw})
            } else {
                Err(ErrorKind::NoCommandGiven.into())
            }
        }

        else if let Some(submatches) = matches.subcommand_matches("auth") {
            process_common(submatches, config);
            let user = submatches.value_of("USER").unwrap().to_owned();
            Ok(Command::Auth{user})
        }

        else if let Some(submatches) = matches.subcommand_matches("cat") {
            process_common(submatches, config);
            let user      = submatches.value_of("ME").map(str::to_owned);
            let mut rpats = Vec::new();

            for arg in submatches.values_of("FILE").unwrap() {
                rpats.push(parse_hw_file(arg, false)?);
            }

            Ok(Command::Cat{user, rpats})
        }

        else if let Some(submatches) = matches.subcommand_matches("create") {
            process_common(submatches, config);
            let user = submatches.value_of("USER").unwrap().to_owned();
            Ok(Command::Create{user})
        }

        else if let Some(submatches) = matches.subcommand_matches("cp") {
            process_common(submatches, config);
            let user     = submatches.value_of("ME").map(str::to_owned);
            let all      = submatches.is_present("ALL");
            let mut srcs = Vec::new();
            let dst      = parse_cp_arg(submatches.value_of("DST").unwrap(), true)?;

            for src in submatches.values_of("SRC").unwrap() {
                srcs.push(parse_cp_arg(src, all)?);
            }

            Ok(Command::Cp{user, srcs, dst})
        }

        else if let Some(submatches) = matches.subcommand_matches("deauth") {
            process_common(submatches, config);
            Ok(Command::Deauth)
        }

        else if let Some(submatches) = matches.subcommand_matches("ls") {
            process_common(submatches, config);
            let user      = submatches.value_of("ME").map(str::to_owned);
            let ls_spec   = submatches.value_of("SPEC").unwrap();
            let (hw, pat) = parse_hw_opt_file(ls_spec)?;
            Ok(Command::Ls{user, rpat: RemotePattern{hw, pat}})
        }

        else if let Some(submatches) = matches.subcommand_matches("partner") {
            process_common(submatches, config);
            let me0 = submatches.value_of("ME");

            let process_partner =
                |matches: &clap::ArgMatches, config: &mut config::Config|
                    -> Result<(Option<String>, usize, String)>
                {
                    process_common(matches, config);
                    let hw   = matches.value_of("HW").unwrap();
                    let them = matches.value_of("USER").unwrap();
                    let me   = match matches.value_of("ME") {
                        Some(username) => Some(username),
                        None           => me0,
                    }.map(str::to_owned);
                    Ok((me, parse_hw(hw)?, them.to_owned()))
                };

            if let Some(subsubmatches) = submatches.subcommand_matches("request") {
                let (me, hw, them) = process_partner(subsubmatches, config)?;
                Ok(Command::PartnerRequest{me, hw, them})
            } else if let Some(subsubmatches) = submatches.subcommand_matches("accept") {
                let (me, hw, them) = process_partner(subsubmatches, config)?;
                Ok(Command::PartnerAccept{me, hw, them})
            } else if let Some(subsubmatches) = submatches.subcommand_matches("cancel") {
                let (me, hw, them) = process_partner(subsubmatches, config)?;
                Ok(Command::PartnerCancel{me, hw, them})
            } else {
                Ok(Command::Partner{me: me0.map(str::to_owned)})
            }
        }

        else if let Some(submatches) = matches.subcommand_matches("passwd") {
            process_common(submatches, config);
            let user = submatches.value_of("ME").map(str::to_owned);
            Ok(Command::Passwd{user})
        }

        else if let Some(submatches) = matches.subcommand_matches("rm") {
            process_common(submatches, config);
            let user      = submatches.value_of("ME").map(str::to_owned);
            let all       = submatches.is_present("ALL");
            let mut rpats = Vec::new();

            for arg in submatches.values_of("FILE").unwrap() {
                rpats.push(parse_hw_file(arg, all)?);
            }

            Ok(Command::Rm{user, rpats})
        }

        else if let Some(submatches) = matches.subcommand_matches("status") {
            process_common(submatches, config);
            let user = submatches.value_of("ME").map(str::to_owned);
            let hw   = match submatches.value_of("HW") {
                Some(hw_spec) => Some(parse_hw(hw_spec)?),
                None          => None,
            };
            Ok(Command::Status{user, hw})
        }

        else if let Some(submatches) = matches.subcommand_matches("whoami") {
            process_common(submatches, config);
            Ok(Command::Whoami)
        }

        else {
            Err(ErrorKind::NoCommandGiven.into())
        }
    }
}

trait AppExt {
    fn add_admin(self) -> Self;
    fn add_common(self) -> Self;
    fn add_partner_args(self) -> Self;
    fn add_user_opt(self, about: &'static str) -> Self;

    // An optional positional argument:
    fn opt_arg(self, name: &'static str, help: &'static str) -> Self;
    // A required positional argument:
    fn req_arg(self, name: &'static str, help: &'static str) -> Self;
    // A required, multiple positional argument:
    fn req_args(self, name: &'static str, help: &'static str) -> Self;
    // An optional flag:
    fn flag(self, name: &'static str, flag: &'static str, help: &'static str) -> Self;
}

impl<'a, 'b> AppExt for clap::App<'a, 'b> {
    #[cfg(feature = "admin")]
    fn add_admin(self) -> Self {
        use clap::*;

        self.subcommand(SubCommand::with_name("admin")
            .about("Administrative commands")
            .add_common()
            .subcommand(SubCommand::with_name("extend")
                .about("Extends a due date")
                .add_common()
                .flag("EVAL", "eval", "Extends self eval instead of file submission")
                .req_arg("HW", "The homework to extend")
                .req_arg("USER", "The user to extend")
                .req_arg("DATESPEC", "The new due date"))
            .subcommand(SubCommand::with_name("set_exam")
                .about("Sets the grade for an exam")
                .add_common()
                .req_arg("EXAM", "The exam number whose grade to set")
                .req_arg("USER", "The user whose grade to set")
                .req_arg("POINTS", "The points scored")
                .req_arg("POSSIBLE", "The points possible"))
            .subcommand(SubCommand::with_name("submissions")
                .about("Lists submissions for a given assignment")
                .add_common()
                .req_arg("HW", "The assignment to query")))
    }

    #[cfg(not(feature = "admin"))]
    fn add_admin(self) -> Self {
        self
    }

    fn add_common(self) -> Self {
        self.arg(clap::Arg::with_name("VERBOSE")
                .short("v")
                .long("verbose")
                .multiple(true)
                .takes_value(false)
                .help("Makes the output more verbose"))
            .arg(clap::Arg::with_name("QUIET")
                .short("q")
                .long("quiet")
                .multiple(true)
                .takes_value(false)
                .help("Makes the output quieter"))
    }

    fn add_partner_args(self) -> Self {
        self.add_user_opt("The user whose partners to manage")
            .add_common()
            .req_arg("HW", "The homework of the partner request")
            .req_arg("USER", "The other user of the partner request")
    }

    #[cfg(feature = "admin")]
    fn add_user_opt(self, about: &'static str) -> Self {
        self.arg(clap::Arg::with_name("ME")
            .long("user")
            .short("u")
            .help(about)
            .takes_value(true)
            .required(false))
    }

    #[cfg(not(feature = "admin"))]
    fn add_user_opt(self, _about: &'static str) -> Self {
        self
    }

    fn opt_arg(self, name: &'static str, help: &'static str) -> Self {
        self.arg(clap::Arg::with_name(name)
            .takes_value(true)
            .required(false)
            .help(help))
    }

    fn req_arg(self, name: &'static str, help: &'static str) -> Self {
        self.arg(clap::Arg::with_name(name)
            .takes_value(true)
            .required(true)
            .help(help))
    }

    fn req_args(self, name: &'static str, help: &'static str) -> Self {
        self.arg(clap::Arg::with_name(name)
            .takes_value(true)
            .multiple(true)
            .required(true)
            .help(help))
    }

    fn flag(self, name: &'static str, long: &'static str, help: &'static str) -> Self {
        self.arg(clap::Arg::with_name(name)
            .long(long)
            .short(&long[..1])
            .help(help)
            .takes_value(false)
            .required(false))
    }
}

mod re {
    use lazy_static::lazy_static;
    use regex::Regex;

    lazy_static! {
        pub static ref HW_ONLY:         Regex = Regex::new(r"^hw(\d+):?$").unwrap();
        pub static ref HW_OPT_FILE:     Regex = Regex::new(r"^hw(\d+)(?::(.*))?$").unwrap();
        pub static ref HW_FILE:         Regex = Regex::new(r"^hw(\d+):(.*)$").unwrap();
        pub static ref HW_FILE_NE:      Regex = Regex::new(r"^hw(\d+):(.+)$").unwrap();
        pub static ref LOCAL_FILE:      Regex = Regex::new(r"^:(.+)$").unwrap();
    }
}

fn parse_hw(spec: &str) -> Result<usize> {
    if let Some(i) = re::HW_ONLY.captures(spec)
        .and_then(|captures| captures.get(1))
        .and_then(|s| s.as_str().parse().ok()) {
        Ok(i)
    } else {
        Err(syntax_error("homework spec", spec))
    }
}

fn parse_hw_opt_file(spec: &str) -> Result<(usize, String)> {
    let captures  = re::HW_OPT_FILE.captures(spec)
        .ok_or_else(|| syntax_error("homework or file spec", spec))?;
    let capture1  = captures.get(1).unwrap().as_str();
    let capture2  = captures.get(2).map(|c| c.as_str());
    let hw_number = capture1.parse().unwrap();
    let pattern   = capture2.unwrap_or("").to_owned();
    Ok((hw_number, pattern))
}

fn parse_hw_file(file_spec: &str, allow_bare: bool) -> Result<RemotePattern> {
    let re = if allow_bare {&*re::HW_FILE} else {&*re::HW_FILE_NE};

    let err = || {
        let message = if allow_bare {
            "remote file or homework spec"
        } else {
            "remote file spec"
        };
        syntax_error(message, file_spec)
    };

    let captures  = re.captures(file_spec).ok_or_else(err)?;
    let capture1  = captures.get(1).unwrap().as_str();
    let capture2  = captures.get(2).unwrap().as_str();
    let hw        = capture1.parse().unwrap();
    let pat       = capture2.to_owned();
    Ok(RemotePattern{hw, pat})
}

fn parse_cp_arg(spec: &str, allow_bare: bool) -> Result<CpArg> {
    if spec.is_empty() {
        Err(syntax_error("file name", spec))?
    } else if let Some(captures) = re::LOCAL_FILE.captures(spec) {
        let filename = captures.get(1).unwrap().as_str().to_owned();
        Ok(CpArg::Local(filename.into()))
    } else if let Some(_) = spec.find(':') {
        let rp = parse_hw_file(spec, allow_bare)?;
        Ok(CpArg::Remote(rp))
    } else {
        Ok(CpArg::Local(spec.into()))
    }
}
