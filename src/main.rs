fn main() {
    let options = smolsandbox::cli::Cli::parse_application().into_options();
    if let Err(error) = options.validate() {
        eprintln!("ERROR: {error}");
        std::process::exit(2);
    }
    let status = match smolsandbox::lifecycle::run(&options) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ERROR: {error:#}");
            1
        }
    };
    std::process::exit(status);
}
