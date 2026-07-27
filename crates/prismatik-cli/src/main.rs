//! Prismatik CLI executable entrypoint.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = prismatik_cli::execute(&args);
    std::process::exit(code);
}
