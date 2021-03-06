use clap::Shell;
use std::env;

include!("src/bin/clap_app/mod.rs");

fn main() {
    let outdir = match env::var_os("OUT_DIR") {
        None => return,
        Some(outdir) => outdir,
    };
    let mut app = build_cli();
    app.gen_completions("gsc", Shell::Fish, outdir);
}
