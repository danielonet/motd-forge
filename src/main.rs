fn main() {
    std::process::exit(motd_forge::cli::run(std::env::args().skip(1).collect()));
}
