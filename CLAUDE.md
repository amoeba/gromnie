# gromnie

We are building gromnie, a command line client for a client-server video game protocol in Rust that runs on tokio for async.
The client uses a well defined protocol from an in-development crate called acprotocol. We currently have some structures redefined in gromnie for ease of development and while we troubleshoot issues in acprotocol but we should generally try to use structures from acprotocol when we can.

We are currently working on getting logged into the game world. So far, we have been able to send our login request (LoginRequest), receive the servers ConnectRequest, send the server back the ConnectResponse, and we seem to be getting the list of characters from the server (LoginCharacterSet) though we're still verifying this.

As you test, always use tmux sessions for running subcommands and reading their output, never temporary files or subshells.

- List sessions to find our sessions `tmux list-sessions`
- Send a command to a session: `tmux send-keys`
- Read text from a session: `tmux capture-pane`

Run the client with `cargo run` in the tmux session called "gromnie".
The server we're connecting to is also being run in tmux in a session called "ace".

There are two critical resources for our use here:

1. The server source code, which we can modify with debugging statements to help us. It's in ~/src/acemulator/ace/source/ace.server.
2. A WIP test client in C#, in ~/src/actestclient. The person who wrotes this knows their stuff really well but I'm not sure it works.
