use bcrypt::{DEFAULT_COST, hash};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <password>", args[0]);
        eprintln!();
        eprintln!("Generate a bcrypt hash for the given password.");
        eprintln!("The hash can be used as the AUTH_PASSWORD_HASH environment variable.");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  cargo run --bin generate-password-hash -- mypassword");
        process::exit(1);
    }

    let password = &args[1];

    match hash(password, DEFAULT_COST) {
        Ok(hashed) => {
            println!("Bcrypt hash for password '{}':", password);
            println!();
            println!("{}", hashed);
            println!();
            println!("Add this to your .env file:");
            println!("AUTH_PASSWORD_HASH={}", hashed);
        }
        Err(e) => {
            eprintln!("Error generating hash: {}", e);
            process::exit(1);
        }
    }
}
