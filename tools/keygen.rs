use stellar_strkey::ed25519::{PublicKey, SecretKey};
use std::env;
use std::process;

fn main() {
    // Generate a new Stellar keypair
    let secret_key = SecretKey::random();
    let public_key = PublicKey::from_secret_key(&secret_key);

    // Output the public and secret keys
    println!("Public Key: {}", public_key);
    println!("Secret Key: {}", secret_key);
    println!("\nWARNING: Keep the secret key safe and do not share it.");

    // Store the secret key in an environment variable
    let secret_key_env = "STELLAR_PLATFORM_SECRET";
    if let Err(e) = env::set_var(secret_key_env, secret_key.to_string()) {
        eprintln!("Failed to set environment variable {}: {}", secret_key_env, e);
        process::exit(1);
    }

    println!("Secret key stored in environment variable: {}", secret_key_env);
}