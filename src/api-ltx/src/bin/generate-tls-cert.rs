use rcgen::generate_simple_self_signed;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <output_directory> [subject_alt_name...]", args[0]);
        eprintln!();
        eprintln!("Generate a self-signed TLS certificate for development/testing.");
        eprintln!("Creates cert.pem and key.pem in the specified output directory.");
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  output_directory      Directory where cert.pem and key.pem will be created");
        eprintln!(
            "  subject_alt_name      Optional Subject Alternative Names (default: localhost, 127.0.0.1, 0.0.0.0)"
        );
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  cargo run --bin generate-tls-cert -- ./certs");
        eprintln!("  cargo run --bin generate-tls-cert -- ./certs localhost 127.0.0.1 myapp.local");
        process::exit(1);
    }

    let output_dir = PathBuf::from(&args[1]);

    // Create output directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&output_dir) {
        eprintln!("Error creating output directory: {}", e);
        process::exit(1);
    }

    // Parse Subject Alternative Names from arguments or use defaults
    let subject_alt_names: Vec<String> = if args.len() > 2 {
        args[2..].to_vec()
    } else {
        vec!["localhost".to_string(), "127.0.0.1".to_string(), "0.0.0.0".to_string()]
    };

    eprintln!("Generating self-signed certificate...");
    eprintln!("Subject Alternative Names: {:?}", subject_alt_names);

    // Generate the certificate
    let cert = match generate_simple_self_signed(subject_alt_names) {
        Ok(cert) => cert,
        Err(e) => {
            eprintln!("Error generating certificate: {}", e);
            process::exit(1);
        }
    };

    // Write certificate to file
    let cert_path = output_dir.join("cert.pem");
    if let Err(e) = fs::write(&cert_path, cert.cert.pem()) {
        eprintln!("Error writing certificate file: {}", e);
        process::exit(1);
    }

    // Write private key to file
    let key_path = output_dir.join("key.pem");
    if let Err(e) = fs::write(&key_path, cert.key_pair.serialize_pem()) {
        eprintln!("Error writing private key file: {}", e);
        process::exit(1);
    }

    eprintln!();
    eprintln!("✓ TLS certificate generated successfully!");
    eprintln!();
    println!("{}", cert_path.display());
    println!("{}", key_path.display());
    eprintln!();
    eprintln!("Add these paths to your .env file:");
    eprintln!("  TLS_CERT_PATH={}", cert_path.display());
    eprintln!("  TLS_KEY_PATH={}", key_path.display());
}
