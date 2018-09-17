use gsc_client::*;
use gsc_client::errors::{Result, ResultExt, ErrorKind, syntax_error};

use vlog::*;

use std::error::Error;
use std::process::exit;
use std::str::FromStr;

fn main() {
    vlog::set_verbosity_level(3);

    match do_it() {
        Err(err)  => {
            ve1!("{}", err);

            let mut cause = err.cause();

            while let Some(error) = cause {
                ve1!("Cause: {}", error);
                cause = error.cause();
            }

            exit(1);
        }
        Ok(true)  => exit(2),
        Ok(false) => (),
    }
}

enum Command {
    AdminCsv,
    AdminDivorce{user: String, hw: usize},
    AdminExtend{user: String, hw: usize, date: String, eval: bool},
    AdminPartners{user: String, hw: usize},
    AdminSetAuto{user: String, hw: usize, score: f64, comment: String},
    AdminSetExam{user: String, exam: usize, num: usize, den: usize},
    AdminSubmissions{hw: usize},
    Auth{user: String},
    Cat{rpats: Vec<RemotePattern>},
    Create{user: String},
    Cp{srcs: Vec<CpArg>, dst: CpArg},
    Deauth,
    Ls{rpats: Vec<RemotePattern>},
    Partner,
    PartnerRequest{hw: usize, them: String},
    PartnerAccept{hw: usize, them: String},
    PartnerCancel{hw: usize, them: String},
    Passwd,
    Rm{rpats: Vec<RemotePattern>},
    Status{hw: Option<usize>},
    Whoami,
}

fn do_it() -> Result<bool> {
    let mut client = GscClient::new()?;
    let command    = GscClientApp::new().process(client.config_mut())?;
    client.config().activate_verbosity();

    use self::Command::*;

    match command {
        AdminCsv                     => client.admin_csv(),
        AdminDivorce{user, hw}       => client.admin_divorce(&user, hw),
        AdminExtend{user, hw, date, eval}
                                     => client.admin_extend(&user, hw, &date, eval),
        AdminPartners{user, hw}      => client.admin_partners(&user, hw),
        AdminSetAuto{user, hw, score, comment}
                                     => client.admin_set_auto(&user, hw, score, &comment),
        AdminSetExam{user, exam, num, den}
                                     => client.admin_set_exam(&user, exam, num, den),
        AdminSubmissions{hw}         => client.admin_submissions(hw),
        Auth{user}                   => client.auth(&user),
        Cat{rpats}                   => client.cat(&rpats),
        Create{user}                 => client.create(&user),
        Cp{srcs, dst}                => client.cp(&srcs, &dst),
        Deauth                       => client.deauth(),
        Ls{rpats}                    => client.ls(&rpats),
        Partner                      => client.partner(),
        PartnerRequest{hw, them}     => client.partner_request(hw, &them),
        PartnerAccept{hw, them}      => client.partner_accept(hw, &them),
        PartnerCancel{hw, them}      => client.partner_cancel(hw, &them),
        Passwd                       => client.passwd(),
        Rm{rpats}                    => client.rm(&rpats),
        Status{hw: Some(i)}          => client.status_hw(i),
        Status{hw: None}             => client.status_user(),
        Whoami                       => client.whoami(),
    }?;

    Ok(client.had_warning())
}

struct GscClientApp<'a: 'b, 'b>(clap::App<'a, 'b>);

fn process_common<'a>(matches: &clap::ArgMatches<'a>, config: &mut config::Config) {
    let vs = matches.occurrences_of("VERBOSE") as isize;
    let qs = matches.occurrences_of("QUIET") as isize;
    let verbosity = config.get_verbosity() + vs - qs;
    config.set_verbosity(verbosity);

    if let Some(user) = matches.value_of("ME") {
        config.set_on_behalf(user.to_owned());
    }
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
                .flag("ALL", "all", "Print all files in the specified homeworks")
                .req_args("SPEC", "The remote files or homeworks to print"))
            .subcommand(SubCommand::with_name("cp")
                .about("Copies files to or from the server")
                .add_common()
                .flag("ALL", "all", "Copy all the files in the specified source homeworks")
                .req_args("SRC", "The files to copy")
                .req_arg("DST", "The destination of the files"))
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
                .req_args("SPEC", "The homeworks or files to list, e.g. ‘hw3’"))
            .subcommand(SubCommand::with_name("partner")
                .about("Manages partners")
                .add_common()
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
                .add_common())
            .subcommand(SubCommand::with_name("rm")
                .about("Removes remote files")
                .add_common()
                .flag("ALL", "all", "Remove all the files in the specified homework")
                .req_args("SPEC", "The remote files or homeworks to remove"))
            .subcommand(SubCommand::with_name("status")
                .about("Retrieves user or submission status")
                .add_common()
                .opt_arg("HW", "The homework to lookup, e.g. ‘hw3’"))
            .subcommand(SubCommand::with_name("whoami")
                .about("Prints your username, if authenticated")
                .add_common()))
    }

    fn process(self, config: &mut config::Config) -> Result<Command> {
        let matches = self.0.get_matches();
        process_common(&matches, config);

        if let Some(submatches) = matches.subcommand_matches("admin") {
            process_common(submatches, config);

            if let Some(subsubmatches) = submatches.subcommand_matches("csv") {
                process_common(subsubmatches, config);
                Ok(Command::AdminCsv)
            } else if let Some(subsubmatches) = submatches.subcommand_matches("divorce") {
                process_common(subsubmatches, config);
                let hw   = parse_hw(subsubmatches.value_of("HW").unwrap())?;
                let user = subsubmatches.value_of("USER").unwrap().to_owned();
                Ok(Command::AdminDivorce { user, hw })
            } else if let Some(subsubmatches) = submatches.subcommand_matches("extend") {
                process_common(subsubmatches, config);
                let eval = subsubmatches.is_present("EVAL");
                let hw   = parse_hw(subsubmatches.value_of("HW").unwrap())?;
                let user = subsubmatches.value_of("USER").unwrap().to_owned();
                let date = subsubmatches.value_of("DATESPEC").unwrap().to_owned();
                Ok(Command::AdminExtend { hw, user, date, eval })
            } else if let Some(subsubmatches) = submatches.subcommand_matches("partners") {
                process_common(subsubmatches, config);
                let hw   = parse_hw(subsubmatches.value_of("HW").unwrap())?;
                let user = subsubmatches.value_of("USER").unwrap().to_owned();
                Ok(Command::AdminPartners { user, hw })
            } else if let Some(subsubmatches) = submatches.subcommand_matches("set_auto") {
                process_common(subsubmatches, config);
                let hw      = parse_hw(subsubmatches.value_of("HW").unwrap())?;
                let user    = subsubmatches.value_of("USER").unwrap().to_owned();
                let score   = subsubmatches.value_of("SCORE").unwrap().parse()?;
                let comment = subsubmatches.value_of("COMMENT").unwrap().to_owned();
                Ok(Command::AdminSetAuto { hw, user, score, comment })
            } else if let Some(subsubmatches) = submatches.subcommand_matches("set_exam") {
                process_common(subsubmatches, config);
                let exam = subsubmatches.value_of("EXAM").unwrap().parse_descr("exam number")?;
                let user = subsubmatches.value_of("USER").unwrap().to_owned();
                let num  = subsubmatches.value_of("POINTS").unwrap().parse_descr("points scored")?;
                let den  = subsubmatches.value_of("POSSIBLE").unwrap().parse_descr("points possible")?;
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
            let all = submatches.is_present("ALL");

            let mut rpats = Vec::new();

            for arg in submatches.values_of("SPEC").unwrap() {
                let rpat = parse_hw_opt_file(arg)?;

                if rpat.is_whole_hw() && !all {
                    Err(ErrorKind::CommandRequiresFlag("cat".to_owned()))?;
                }

                rpats.push(rpat);
            }

            Ok(Command::Cat{rpats})
        }

        else if let Some(submatches) = matches.subcommand_matches("create") {
            process_common(submatches, config);
            let user = submatches.value_of("USER").unwrap().to_owned();
            Ok(Command::Create{user})
        }

        else if let Some(submatches) = matches.subcommand_matches("cp") {
            process_common(submatches, config);
            let all      = submatches.is_present("ALL");
            let mut srcs = Vec::new();
            let dst      = parse_cp_arg(submatches.value_of("DST").unwrap())?;

            for src in submatches.values_of("SRC").unwrap() {
                let arg = parse_cp_arg(src)?;

                if arg.is_whole_hw() && !all {
                    Err(ErrorKind::CommandRequiresFlag("cp".to_owned()))?;
                }

                srcs.push(arg);
            }

            Ok(Command::Cp{srcs, dst})
        }

        else if let Some(submatches) = matches.subcommand_matches("deauth") {
            process_common(submatches, config);
            Ok(Command::Deauth)
        }

        else if let Some(submatches) = matches.subcommand_matches("ls") {
            process_common(submatches, config);
            
            let ls_specs   = submatches.values_of("SPEC").unwrap();
            let mut rpats = Vec::new();

            for ls_spec in ls_specs {
                rpats.push(parse_hw_opt_file(ls_spec)?);
            }

            Ok(Command::Ls{rpats})
        }

        else if let Some(submatches) = matches.subcommand_matches("partner") {
            process_common(submatches, config);

            let mut process_partner = |matches: &clap::ArgMatches| -> Result<_> {
                process_common(matches, config);
                let hw   = matches.value_of("HW").unwrap();
                let them = matches.value_of("USER").unwrap();
                Ok((parse_hw(hw)?, them.to_owned()))
            };

            if let Some(subsubmatches) = submatches.subcommand_matches("request") {
                let (hw, them) = process_partner(subsubmatches)?;
                Ok(Command::PartnerRequest{hw, them})
            } else if let Some(subsubmatches) = submatches.subcommand_matches("accept") {
                let (hw, them) = process_partner(subsubmatches)?;
                Ok(Command::PartnerAccept{hw, them})
            } else if let Some(subsubmatches) = submatches.subcommand_matches("cancel") {
                let (hw, them) = process_partner(subsubmatches)?;
                Ok(Command::PartnerCancel{hw, them})
            } else {
                Ok(Command::Partner)
            }
        }

        else if let Some(submatches) = matches.subcommand_matches("passwd") {
            process_common(submatches, config);
            Ok(Command::Passwd)
        }

        else if let Some(submatches) = matches.subcommand_matches("rm") {
            process_common(submatches, config);
            let all       = submatches.is_present("ALL");
            let mut rpats = Vec::new();

            for arg in submatches.values_of("SPEC").unwrap() {
                let rpat = parse_hw_opt_file(arg)?;

                if rpat.is_whole_hw() && !all {
                    Err(ErrorKind::CommandRequiresFlag("rm".to_owned()))?;
                }

                rpats.push(rpat);
            }

            Ok(Command::Rm{rpats})
        }

        else if let Some(submatches) = matches.subcommand_matches("status") {
            process_common(submatches, config);
            let hw   = match submatches.value_of("HW") {
                Some(hw_spec) => Some(parse_hw(hw_spec)?),
                None          => None,
            };
            Ok(Command::Status{hw})
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
    fn add_user_opt(self) -> Self;

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
            .subcommand(SubCommand::with_name("csv")
                .about("Prints the grade spreadsheet")
                .add_common())
            .subcommand(SubCommand::with_name("divorce")
                .about("Ends a partnership")
                .add_common()
                .req_arg("HW", "The homework in question")
                .req_arg("USER", "One of the two partners"))
            .subcommand(SubCommand::with_name("extend")
                .about("Extends a due date")
                .add_common()
                .flag("EVAL", "eval", "Extends self eval instead of file submission")
                .req_arg("HW", "The homework to extend")
                .req_arg("USER", "The user to extend")
                .req_arg("DATESPEC", "The new due date"))
            .subcommand(SubCommand::with_name("partners")
                .about("Looks up a partnership")
                .add_common()
                .req_arg("HW", "The homework to lookup")
                .req_arg("USER", "The user to lookup"))
            .subcommand(SubCommand::with_name("set_auto")
                .about("Records the result of the autograder")
                .add_common()
                .req_arg("HW", "The homework to set the grade on")
                .req_arg("USER", "The user whose grade to set")
                .req_arg("SCORE", "The score [0.0, 1.0]")
                .req_arg("COMMENT", "A comment"))
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
            .add_user_opt()
    }

    fn add_partner_args(self) -> Self {
        self.add_common()
            .req_arg("HW", "The homework of the partner request")
            .req_arg("USER", "The other user of the partner request")
    }

    #[cfg(feature = "admin")]
    fn add_user_opt(self) -> Self {
        self.arg(clap::Arg::with_name("ME")
            .long("user")
            .short("u")
            .help("The user to act on behalf of")
            .takes_value(true)
            .required(false))
    }

    #[cfg(not(feature = "admin"))]
    fn add_user_opt(self) -> Self {
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
        pub static ref LOCAL_FILE:      Regex = Regex::new(r"^:(.+)$").unwrap();
    }
}

trait ParseWithDescription {
    fn parse_descr<F: FromStr>(&self, descr: &str) -> Result<F>
        where <F as FromStr>::Err: std::error::Error + Send + 'static;
}

impl ParseWithDescription for str {
    fn parse_descr<F: FromStr>(&self, descr: &str) -> Result<F>
        where <F as FromStr>::Err: std::error::Error + Send + 'static
    {
        self.parse().chain_err(|| syntax_error(descr, self))
    }
}

fn parse_hw(spec: &str) -> Result<usize> {
    if let Some(i) = re::HW_ONLY.captures(spec)
        .and_then(|captures| captures.get(1))
        .and_then(|s| s.as_str().parse().ok()) {
        Ok(i)
    } else {
        Err(syntax_error("homework spec", spec))?
    }
}

fn parse_hw_opt_file(spec: &str) -> Result<RemotePattern> {
    let captures  = re::HW_OPT_FILE.captures(spec)
        .ok_or_else(|| syntax_error("homework or file spec", spec))?;
    let capture1  = captures.get(1).unwrap().as_str();
    let capture2  = captures.get(2).map(|c| c.as_str());
    let hw        = capture1.parse().unwrap();
    let pat       = capture2.unwrap_or("").to_owned();
    Ok(RemotePattern{hw, pat})
}

fn parse_hw_file(file_spec: &str) -> Result<RemotePattern> {
    let captures  = re::HW_FILE.captures(file_spec)
        .ok_or_else(|| syntax_error("remote file or homework spec", file_spec))?;
    let capture1  = captures.get(1).unwrap().as_str();
    let capture2  = captures.get(2).unwrap().as_str();
    let hw        = capture1.parse().unwrap();
    let pat       = capture2.to_owned();
    Ok(RemotePattern{hw, pat})
}

fn parse_cp_arg(spec: &str) -> Result<CpArg> {
    if spec.is_empty() {
        Err(syntax_error("file name", spec))?
    } else if let Some(captures) = re::LOCAL_FILE.captures(spec) {
        let filename = captures.get(1).unwrap().as_str().to_owned();
        Ok(CpArg::Local(filename.into()))
    } else if let Some(_) = spec.find(':') {
        let rp = parse_hw_file(spec)?;
        Ok(CpArg::Remote(rp))
    } else {
        Ok(CpArg::Local(spec.into()))
    }
}

