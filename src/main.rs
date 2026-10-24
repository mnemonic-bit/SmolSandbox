use clap::Parser;

fn main() {
    let options = codex_sandbox::cli::Cli::parse().into_options();
    if let Err(error) = options.validate() {
        eprintln!("ERROR: {error}");
        std::process::exit(2);
    }
    let status = match codex_sandbox::lifecycle::run(&options) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ERROR: {error:#}");
            1
        }
    };
    std::process::exit(status);
}
