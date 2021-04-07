use clap::*;

pub fn build_cli() -> App<'static, 'static> {
    App::new("gsc")
        .author("Jesse A. Tov <jesse@eecs.northwestern.edu>")
        .about("Command-line interface to the GSC server")
        .version(crate_version!())
        .add_common()
        .add_admin()
        .subcommand(
            SubCommand::with_name("auth")
                .about("Authenticates with the server")
                .add_common()
                .req_arg("USER", "Your username (i.e., your NetID)"),
        )
        .subcommand(
            SubCommand::with_name("cat")
                .about("Prints remote files to stdout")
                .add_common()
                .flag("ALL", "all", "Print all files in the specified homeworks")
                .req_args("SPEC", "The remote files or homeworks to print"),
        )
        .subcommand(
            SubCommand::with_name("cp")
                .about("Copies files to or from the server")
                .add_common()
                .add_overwrite_opts()
                .flag(
                    "ALL",
                    "all",
                    "Copy all the files in the specified source homeworks",
                )
                .req_args("SRC", "The files to copy")
                .req_arg("DST", "The destination of the files"),
        )
        .subcommand(
            SubCommand::with_name("deauth")
                .about("Forgets authentication credentials")
                .add_common(),
        )
        .subcommand(
            SubCommand::with_name("eval")
                .about("Manages self evaluation")
                .add_common()
                .subcommand(
                    SubCommand::with_name("get")
                        .about("Shows evaluation")
                        .req_arg("HW", "The homework to lookup")
                        .req_arg("NUMBER", "The eval item to lookup"),
                )
                .subcommand(
                    SubCommand::with_name("set")
                        .about("Performs self evaluation")
                        .req_arg("HW", "The homework to evaluate")
                        .req_arg("NUMBER", "The eval item to set")
                        .req_arg("SCORE", "The score [0.0, 1.0]")
                        .opt_arg("EXPLANATION", "Your justification for the score"),
                ),
        )
        .subcommand(
            SubCommand::with_name("ls")
                .about("Lists files")
                .add_common()
                .req_args("SPEC", "The homeworks or files to list, e.g. ‘hw3’"),
        )
        .subcommand(
            SubCommand::with_name("mv")
                .about("Renames a remote file")
                .add_common()
                .add_overwrite_opts()
                .req_arg("SRC", "The file to rename")
                .req_arg("DST", "The new name "),
        )
        .subcommand(
            SubCommand::with_name("partner")
                .about("Manages partners")
                .add_common()
                .subcommand(
                    SubCommand::with_name("request")
                        .about("Sends a partner request")
                        .add_partner_args(),
                )
                .subcommand(
                    SubCommand::with_name("accept")
                        .about("Accepts a partner request")
                        .add_partner_args(),
                )
                .subcommand(
                    SubCommand::with_name("cancel")
                        .about("Cancels a partner request")
                        .add_partner_args(),
                ),
        )
        .subcommand(
            SubCommand::with_name("rm")
                .about("Removes remote files")
                .add_common()
                .flag(
                    "ALL",
                    "all",
                    "Remove all the files in the specified homework",
                )
                .req_args("SPEC", "The remote files or homeworks to remove"),
        )
        .subcommand(
            SubCommand::with_name("status")
                .about("Retrieves user or submission status")
                .add_common()
                .opt_arg("HW", "The homework to lookup, e.g. ‘hw3’"),
        )
        .subcommand(
            SubCommand::with_name("whoami")
                .about("Prints your username, if authenticated")
                .add_common(),
        )
}

trait AppExt {
    fn add_admin(self) -> Self;
    fn add_common(self) -> Self;
    fn add_everywhere(self) -> Self;
    fn add_overwrite_opts(self) -> Self;
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

        self.subcommand(
            SubCommand::with_name("admin")
                .about("Administrative commands")
                .add_common()
                .subcommand(
                    SubCommand::with_name("csv")
                        .about("Prints the grade spreadsheet")
                        .add_common(),
                )
                .subcommand(
                    SubCommand::with_name("add_user")
                        .about("Adds a user")
                        .add_everywhere()
                        .req_arg("USER", "Name of user to add")
                        .arg(clap::Arg::with_name("GRADER_ROLE")
                            .long("grader")
                            .takes_value(false)
                            .conflicts_with("ADMIN_ROLE")
                            .help("Creates user with grader role"))
                        .arg(clap::Arg::with_name("ADMIN_ROLE")
                            .long("admin")
                            .takes_value(false)
                            .conflicts_with("GRADER_ROLE")
                            .help("Creates user with admin role")),
                )
                .subcommand(
                    SubCommand::with_name("del_user")
                        .about("Deletes a user")
                        .add_everywhere()
                        .req_arg("USER", "Name of user to delete"),
                )
                .subcommand(
                    SubCommand::with_name("divorce")
                        .about("Ends a partnership")
                        .add_common()
                        .req_arg("HW", "The homework in question")
                        .req_arg("USER", "One of the two partners"),
                )
                .subcommand(
                    SubCommand::with_name("extend")
                        .about("Extends a due date")
                        .add_common()
                        .flag(
                            "EVAL",
                            "eval",
                            "Extends self eval instead of file submission",
                        )
                        .req_arg("HW", "The homework to extend")
                        .req_arg("USER", "The user to extend")
                        .req_arg("DATESPEC", "The new due date"),
                )
                .subcommand(
                    SubCommand::with_name("partners")
                        .about("Looks up a partnership")
                        .add_common()
                        .req_arg("HW", "The homework to lookup")
                        .req_arg("USER", "The user to lookup"),
                )
                .subcommand(
                    SubCommand::with_name("permalink")
                        .about("Prints the permalink hash for a given self evaluation")
                        .add_common()
                        .req_arg("HW", "The homework of the self evaluation")
                        .req_arg("USER", "The user whose self evaluation to find")
                        .req_arg("NUMBER", "The eval item number to find"),
                )
                .subcommand(
                    SubCommand::with_name("set_grade")
                        .about("Records the grade for any eval item")
                        .add_common()
                        .req_arg("HW", "The homework to set the grade on")
                        .req_arg("USER", "The user whose grade to set")
                        .req_arg("NUMBER", "The eval item number to set")
                        .req_arg("SCORE", "The score [0.0, 1.0]")
                        .req_arg("COMMENT", "A comment"),
                )
                .subcommand(
                    SubCommand::with_name("set_auto")
                        .about("Records the result of the autograder")
                        .add_common()
                        .req_arg("HW", "The homework to set the grade on")
                        .req_arg("USER", "The user whose grade to set")
                        .req_arg("SCORE", "The score [0.0, 1.0]")
                        .req_arg("COMMENT", "A comment"),
                )
                .subcommand(
                    SubCommand::with_name("set_exam")
                        .about("Sets the grade for an exam")
                        .add_common()
                        .req_arg("EXAM", "The exam number whose grade to set")
                        .req_arg("USER", "The user whose grade to set")
                        .req_arg("POINTS", "The points scored")
                        .req_arg("POSSIBLE", "The points possible"),
                )
                .subcommand(
                    SubCommand::with_name("submissions")
                        .about("Lists submissions for a given assignment")
                        .add_common()
                        .req_arg("HW", "The assignment to query"),
                ),
        )
    }

    #[cfg(not(feature = "admin"))]
    fn add_admin(self) -> Self {
        self
    }

    fn add_everywhere(self) -> Self {
        self.arg(
            clap::Arg::with_name("VERBOSE")
                .short("v")
                .long("verbose")
                .multiple(true)
                .takes_value(false)
                .help("Makes the output more verbose"),
        )
        .arg(
            clap::Arg::with_name("QUIET")
                .short("q")
                .long("quiet")
                .multiple(true)
                .takes_value(false)
                .help("Makes the output quieter"),
        )
    }

    fn add_common(self) -> Self {
        self.arg(
            clap::Arg::with_name("JSON")
                .short("j")
                .long("json")
                .takes_value(false)
                .help("Show raw JSON response"),
        )
        .arg(
            clap::Arg::with_name("HUMAN")
                .short("H")
                .long("human")
                .takes_value(false)
                .help("Show human-formatted result (overrides --json)"),
        )
        .add_everywhere()
        .add_user_opt()
    }

    fn add_overwrite_opts(self) -> Self {
        self.flag("ALWAYS", "f", "Overwrite existing files without asking")
            .flag(
                "ASK",
                "i",
                "Ask (interactively) before overwriting existing files",
            )
            .flag("NEVER", "n", "Never overwrite existing files")
            .group(
                clap::ArgGroup::with_name("overwrite")
                    .args(&["ALWAYS", "ASK", "NEVER"])
                    .multiple(false)
                    .required(false),
            )
    }

    fn add_partner_args(self) -> Self {
        self.add_common()
            .req_arg("HW", "The homework of the partner request")
            .req_arg("USER", "The other user of the partner request")
    }

    #[cfg(feature = "admin")]
    fn add_user_opt(self) -> Self {
        self.arg(
            clap::Arg::with_name("ME")
                .long("user")
                .short("u")
                .help("The user to act on behalf of")
                .takes_value(true)
                .required(false),
        )
    }

    #[cfg(not(feature = "admin"))]
    fn add_user_opt(self) -> Self {
        self
    }

    fn opt_arg(self, name: &'static str, help: &'static str) -> Self {
        self.arg(
            clap::Arg::with_name(name)
                .takes_value(true)
                .required(false)
                .help(help),
        )
    }

    fn req_arg(self, name: &'static str, help: &'static str) -> Self {
        self.arg(
            clap::Arg::with_name(name)
                .takes_value(true)
                .required(true)
                .help(help),
        )
    }

    fn req_args(self, name: &'static str, help: &'static str) -> Self {
        self.arg(
            clap::Arg::with_name(name)
                .takes_value(true)
                .multiple(true)
                .required(true)
                .help(help),
        )
    }

    fn flag(self, name: &'static str, long: &'static str, help: &'static str) -> Self {
        let mut arg = clap::Arg::with_name(name)
            .short(&long[..1])
            .help(help)
            .takes_value(false)
            .required(false)
            .multiple(true);

        if long.len() > 1 {
            arg = arg.long(long);
        }

        self.arg(arg)
    }
}
