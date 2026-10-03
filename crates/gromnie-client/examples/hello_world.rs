//! Live smoke test for the headless [`GromnieClient`] facade.
//!
//! Connects to a real login server, lists characters, enters the world, and
//! reports what happened at each step. This is the only way to exercise the
//! parts the unit tests cannot reach: the actual handshake, the character
//! list, and the world transition.
//!
//! ```text
//! GROMNIE_ACCOUNT=me GROMNIE_PASSWORD=hunter2 \
//!   cargo run -p gromnie-client --example hello_world -- \
//!     --server play.example.com:9000 [--character "Name"] [--say "hi"] [--hold 15]
//! ```
//!
//! Exits non-zero if any step fails, so it is usable in CI against a test
//! account. Note that entering the world makes the character appear online on
//! the server, and `--say` posts a real message to the game world.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use gromnie_client::api::GromnieClient;
use gromnie_client::client::ClientEvent;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let server = flag(&args, "--server").unwrap_or_else(|| "localhost:9000".to_string());
    let account = match env("GROMNIE_ACCOUNT") {
        Some(value) => value,
        None => {
            eprintln!("GROMNIE_ACCOUNT is not set");
            return ExitCode::from(2);
        }
    };
    let password = match env("GROMNIE_PASSWORD") {
        Some(value) => value,
        None => {
            eprintln!("GROMNIE_PASSWORD is not set");
            return ExitCode::from(2);
        }
    };
    let wanted_character = flag(&args, "--character");
    let say = flag(&args, "--say");
    let hold = flag(&args, "--hold")
        .and_then(|v| v.parse().ok())
        .unwrap_or(10u64);

    println!("connecting to {server} as {account}...");
    let started = Instant::now();

    let client = GromnieClient::builder()
        .with_server(&server)
        .with_account(&account, &password)
        .connect()
        .await;

    let client = match client {
        Ok(client) => {
            println!(
                "  connect() resolved in {:?} at scene {:?}",
                started.elapsed(),
                client.scene()
            );
            client
        }
        Err(e) => {
            eprintln!("  connect() failed after {:?}: {e}", started.elapsed());
            return ExitCode::FAILURE;
        }
    };

    // Watch events for the rest of the run so we can see whether anything was
    // dropped, and so the channel has a live reader throughout.
    let mut events = client.subscribe();

    let characters = client.list_characters();
    println!("  {} character(s):", characters.len());
    for character in &characters {
        println!(
            "    {} (id {}){}",
            character.name,
            character.character_id.0,
            if character.seconds_greyed_out > 0 {
                " [pending deletion]"
            } else {
                ""
            }
        );
    }

    if characters.is_empty() {
        eprintln!("  no characters on this account; stopping before enter_world");
        return ExitCode::FAILURE;
    }

    let target = match &wanted_character {
        Some(name) => match client.character(name) {
            Some(character) => character.name.clone(),
            None => {
                eprintln!("  no enterable character named {name:?}");
                return ExitCode::FAILURE;
            }
        },
        None => {
            let name = characters[0].name.clone();
            println!("  (no --character given, using {name:?})");
            name
        }
    };
    let target_id = client
        .character(&target)
        .expect("target just resolved")
        .character_id
        .0;

    // Exercise the case-insensitive lookup independently of the exact name.
    let shouty = target.to_uppercase();
    assert_eq!(
        client.character(&shouty).map(|c| c.character_id.0),
        Some(target_id),
        "case-insensitive lookup disagreed for {shouty:?}"
    );
    println!("  case-insensitive lookup of {shouty:?} resolved");

    let entering = Instant::now();
    let world = match client.enter_world(&target).await {
        Ok(world) => {
            println!(
                "  enter_world({target:?}) resolved in {:?}, character_id {}",
                entering.elapsed(),
                world.character_id
            );
            world
        }
        Err(e) => {
            eprintln!(
                "  enter_world({target:?}) failed after {:?}: {e}",
                entering.elapsed()
            );
            let _ = client.disconnect().await;
            return ExitCode::FAILURE;
        }
    };
    // The scene we resolved on must be the character we asked for, not merely
    // "some" InWorld scene.
    assert_eq!(world.character_id, target_id, "entered the wrong character");

    if let Some(message) = say {
        match client.say(&message).await {
            Ok(()) => println!("  queued say({message:?})"),
            Err(e) => {
                eprintln!("  say failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    // Hold the connection open briefly, tallying events as they arrive. The
    // split matters: a subscriber that sees protocol traffic but no state
    // events is not actually being kept up to date.
    let held = Instant::now();
    let mut protocol = 0usize;
    let mut state = 0usize;
    let mut system = 0usize;
    let mut last = Instant::now();
    while held.elapsed() < Duration::from_secs(hold) {
        match tokio::time::timeout(Duration::from_millis(500), events.recv()).await {
            Ok(Ok(event)) => {
                match event {
                    ClientEvent::Protocol(_) => protocol += 1,
                    ClientEvent::State(_) => state += 1,
                    ClientEvent::System(_) => system += 1,
                }
                if last.elapsed() > Duration::from_secs(2) {
                    println!("  ... still in world after {:?}", held.elapsed());
                    last = Instant::now();
                }
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(missed))) => {
                // Subscribers may fall behind; internal waits do not use this
                // stream, so this is a subscriber problem only.
                println!("  subscriber lagged, missed {missed} event(s)");
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                eprintln!("  event stream closed early");
                return ExitCode::FAILURE;
            }
            Err(_) => continue,
        }
    }
    println!(
        "  held for {hold}s, observed {} event(s): {protocol} protocol, {state} state, {system} system",
        protocol + state + system
    );

    // The scene must still be InWorld, i.e. we did not silently fall back.
    match client.scene() {
        gromnie_client::client::Scene::InWorld(_) => println!("  scene is still InWorld"),
        other => {
            eprintln!("  scene drifted to {other:?}");
            return ExitCode::FAILURE;
        }
    }

    match client.disconnect().await {
        Ok(()) => println!("  disconnect() clean"),
        Err(e) => {
            eprintln!("  disconnect() reported: {e}");
            return ExitCode::FAILURE;
        }
    }

    println!("OK in {:?}", started.elapsed());
    ExitCode::SUCCESS
}

/// Read a `--flag value` pair.
fn flag(args: &[String], name: &str) -> Option<String> {
    let index = args.iter().position(|a| a == name)?;
    args.get(index + 1).cloned()
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}
