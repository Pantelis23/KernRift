use std::os::unix::process::CommandExt;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: kernrift <file.krbo>");
        std::process::exit(2);
    }
    let file = &args[1];
    let err = std::process::Command::new(file).args(&args[2..]).exec();
    eprintln!("kernrift: failed to execute '{}': {}", file, err);
    std::process::exit(1);
}
