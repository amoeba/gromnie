use anyhow::Result;
use wasmtime::component::Linker;

use super::wasm_script::{WasmScriptState, gromnie};
use crate::ScriptContext;
use gromnie_events::SimpleClientAction;

/// Add all host imports to the linker
pub fn add_host_imports(linker: &mut Linker<WasmScriptState>) -> Result<()> {
    // Link the host interface
    gromnie::scripting::host::add_to_linker(linker, |state| state)?;

    Ok(())
}

/// Get the ScriptContext from the WasmScriptState
///
/// # Safety
/// This is safe because:
/// 1. The host_context pointer is set by WasmScript before each WASM call
/// 2. The ScriptContext lives in ScriptRunner which owns the WasmScript
/// 3. The pointer is cleared after each WASM call
/// 4. WASM scripts cannot store the context or use it across calls
fn get_context(state: &mut WasmScriptState) -> &mut ScriptContext {
    unsafe {
        state
            .host_context
            .expect("host_context not set - this is a bug in WasmScript")
            .as_mut()
            .expect("host_context is null - this is a bug in WasmScript")
    }
}

impl gromnie::scripting::host::Host for WasmScriptState {
    fn send_chat(&mut self, message: String) {
        let ctx = get_context(self);
        ctx.send_chat(message);
    }

    fn login_character(&mut self, account_name: String, character_id: u32, character_name: String) {
        let ctx = get_context(self);
        ctx.send_action(SimpleClientAction::LoginCharacter {
            character_id,
            character_name,
            account: account_name,
        });
    }

    fn log(&mut self, message: String) {
        let script_id = self.script_id.clone();
        let ctx = get_context(self);
        ctx.send_action(SimpleClientAction::LogScriptMessage { script_id, message });
    }

    fn schedule_timer(&mut self, delay_secs: u64, name: String) -> u64 {
        let ctx = get_context(self);
        let timer_id = ctx.schedule_timer(delay_secs, name);
        timer_id_to_u64(timer_id)
    }

    fn schedule_recurring(&mut self, interval_secs: u64, name: String) -> u64 {
        let ctx = get_context(self);
        let timer_id = ctx.schedule_recurring(interval_secs, name);
        timer_id_to_u64(timer_id)
    }

    fn cancel_timer(&mut self, timer_id: u64) -> bool {
        let ctx = get_context(self);
        let timer_id = timer_id_from_u64(timer_id);
        ctx.cancel_timer(timer_id)
    }

    fn check_timer(&mut self, timer_id: u64) -> bool {
        let ctx = get_context(self);
        let timer_id = timer_id_from_u64(timer_id);
        ctx.check_timer(timer_id)
    }

    fn get_client_state(&mut self) -> gromnie::scripting::host::ClientState {
        use gromnie_client::client::SessionState;

        let ctx = get_context(self);
        let client_state = ctx.client();

        // Convert SessionState to WIT
        let session_state = match client_state.session.state {
            SessionState::AuthLoginRequest => {
                gromnie::scripting::host::SessionState::AuthLoginRequest
            }
            SessionState::AuthConnectResponse => {
                gromnie::scripting::host::SessionState::AuthConnectResponse
            }
            SessionState::AuthConnected => gromnie::scripting::host::SessionState::AuthConnected,
            SessionState::WorldConnected => gromnie::scripting::host::SessionState::WorldConnected,
            SessionState::TerminationStarted => {
                gromnie::scripting::host::SessionState::TerminationStarted
            }
        };

        let session = gromnie::scripting::host::ClientSession {
            state: session_state,
        };

        // Convert Scene to WIT
        let scene = convert_scene_to_wit(&client_state.scene);

        gromnie::scripting::host::ClientState { session, scene }
    }

    fn get_event_time_millis(&mut self) -> u64 {
        let ctx = get_context(self);
        let _event_time = ctx.event_time();

        // Convert Instant to milliseconds since UNIX epoch
        // Note: Instant doesn't have a direct conversion, so we use SystemTime
        use std::time::SystemTime;
        let now = SystemTime::now();
        let duration_since_epoch = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();

        duration_since_epoch.as_millis() as u64
    }
}

/// Convert TimerId to u64 for WASM ABI
fn timer_id_to_u64(timer_id: crate::TimerId) -> u64 {
    // TimerId is a newtype wrapper around u64
    // We need to extract the inner value
    // Since TimerId doesn't expose this, we use unsafe transmute
    unsafe { std::mem::transmute(timer_id) }
}

/// Convert u64 to TimerId
fn timer_id_from_u64(id: u64) -> crate::TimerId {
    unsafe { std::mem::transmute(id) }
}

/// Convert Rust Scene to WIT Scene
fn convert_scene_to_wit(scene: &gromnie_client::client::Scene) -> gromnie::scripting::host::Scene {
    use gromnie_client::client::{ConnectingProgress, PatchingProgress, Scene};

    match scene {
        Scene::Connecting(connecting) => {
            let connect_progress = match connecting.connect_progress {
                ConnectingProgress::Initial => {
                    gromnie::scripting::host::ConnectingProgress::Initial
                }
                ConnectingProgress::LoginRequestSent => {
                    gromnie::scripting::host::ConnectingProgress::LoginRequestSent
                }
                ConnectingProgress::ConnectRequestReceived => {
                    gromnie::scripting::host::ConnectingProgress::ConnectRequestReceived
                }
                ConnectingProgress::ConnectResponseSent => {
                    gromnie::scripting::host::ConnectingProgress::ConnectResponseSent
                }
            };

            let patch_progress = match connecting.patch_progress {
                PatchingProgress::NotStarted => {
                    gromnie::scripting::host::PatchingProgress::NotStarted
                }
                PatchingProgress::WaitingForDDD => {
                    gromnie::scripting::host::PatchingProgress::WaitingForDdd
                }
                PatchingProgress::ReceivedDDD => {
                    gromnie::scripting::host::PatchingProgress::ReceivedDdd
                }
                PatchingProgress::SentDDDResponse => {
                    gromnie::scripting::host::PatchingProgress::SentDddResponse
                }
                PatchingProgress::Complete => gromnie::scripting::host::PatchingProgress::Complete,
            };

            gromnie::scripting::host::Scene::Connecting(gromnie::scripting::host::ConnectingScene {
                connect_progress,
                patch_progress,
            })
        }
        Scene::CharacterSelect(char_select) => {
            let characters = char_select
                .characters
                .iter()
                .map(|c| gromnie::scripting::host::CharacterIdentity {
                    character_id: c.character_id.0,
                    name: c.name.clone(),
                    seconds_greyed_out: c.seconds_greyed_out,
                })
                .collect();

            let entering_world = char_select.entering_world.as_ref().map(|ew| {
                gromnie::scripting::host::EnteringWorldState {
                    character_id: ew.character_id,
                    character_name: ew.character_name.clone(),
                    account: ew.account.clone(),
                    login_complete: ew.login_complete,
                }
            });

            gromnie::scripting::host::Scene::CharacterSelect(
                gromnie::scripting::host::CharacterSelectScene {
                    account_name: char_select.account_name.clone(),
                    characters,
                    entering_world,
                },
            )
        }
        Scene::CharacterCreate(_) => gromnie::scripting::host::Scene::CharacterCreate(
            gromnie::scripting::host::CharacterCreateScene { placeholder: false },
        ),
        Scene::InWorld(in_world) => {
            gromnie::scripting::host::Scene::InWorld(gromnie::scripting::host::InWorldScene {
                character_id: in_world.character_id,
                character_name: in_world.character_name.clone(),
            })
        }
        Scene::Error(error) => {
            use gromnie_client::client::ClientError;

            let client_error = match &error.error {
                ClientError::CharacterError(err_type) => {
                    gromnie::scripting::host::ClientError::CharacterError(err_type.clone() as u32)
                }
                ClientError::ConnectionFailed(msg) => {
                    gromnie::scripting::host::ClientError::ConnectionFailed(msg.clone())
                }
                ClientError::PatchingFailed(msg) => {
                    gromnie::scripting::host::ClientError::PatchingFailed(msg.clone())
                }
                ClientError::LoginTimeout => gromnie::scripting::host::ClientError::LoginTimeout,
                ClientError::PatchingTimeout => {
                    gromnie::scripting::host::ClientError::PatchingTimeout
                }
            };

            gromnie::scripting::host::Scene::Error(gromnie::scripting::host::ErrorScene {
                error: client_error,
                can_retry: error.can_retry,
            })
        }
    }
}
