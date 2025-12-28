use gromnie::load_tester::ClientNaming;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <client_id>", args[0]);
        println!("Generates account and character names for load testing");
        std::process::exit(1);
    }

    let client_id: u32 = args[1].parse().expect("Invalid client ID");
    let naming = ClientNaming::new(client_id);

    println!("Client ID: {}", client_id);
    println!("Account: {}", naming.account_name());
    println!("Password: {}", naming.password());
    println!("Character: {}", naming.character_name());
}
