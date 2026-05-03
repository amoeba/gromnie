use anyhow::Result;
use std::sync::Arc;
use wasmtime::component::Linker;

use super::wasm_script::{WasmScriptState, gromnie};
use crate::ScriptContext;
use gromnie_events::SimpleClientAction;

/// Add all host imports to the linker
pub fn add_host_imports(linker: &mut Linker<WasmScriptState>) -> Result<()> {
    // Link the host interface
    gromnie::scripting::host::add_to_linker::<WasmScriptState, wasmtime::component::HasSelf<WasmScriptState>>(linker, |state| state)?;

    Ok(())
}

/// Get the current ScriptContext from the WasmScriptState.
fn get_context(state: &WasmScriptState) -> Arc<ScriptContext> {
    Arc::clone(
        state
            .host_context
            .as_ref()
            .expect("host_context not set - this is a bug in WasmScript"),
    )
}

impl gromnie::scripting::host::Host for WasmScriptState {
    async fn send_chat(&mut self, message: String) {
        let ctx = get_context(self);
        ctx.send_chat(message);
    }

    async fn send_tell(&mut self, recipient: String, message: String) {
        let ctx = get_context(self);
        ctx.send_tell(recipient, message);
    }

    async fn open_trade(&mut self, partner_id: u32) {
        let ctx = get_context(self);
        ctx.open_trade(partner_id);
    }

    async fn add_to_trade(&mut self, item_id: u32, slot: u32) {
        let ctx = get_context(self);
        ctx.add_to_trade(item_id, slot);
    }

    async fn accept_trade(&mut self) {
        let ctx = get_context(self);
        ctx.accept_trade();
    }

    async fn decline_trade(&mut self) {
        let ctx = get_context(self);
        ctx.decline_trade();
    }

    async fn reset_trade(&mut self) {
        let ctx = get_context(self);
        ctx.reset_trade();
    }

    async fn close_trade(&mut self) {
        let ctx = get_context(self);
        ctx.close_trade();
    }

    async fn cast_targeted_spell(&mut self, target_id: u32, spell_id: u32) {
        let ctx = get_context(self);
        ctx.cast_targeted_spell(target_id, spell_id);
    }

    async fn cast_untargeted_spell(&mut self, spell_id: u32) {
        let ctx = get_context(self);
        ctx.cast_untargeted_spell(spell_id);
    }

    async fn login_character(
        &mut self,
        account_name: String,
        character_id: u32,
        character_name: String,
    ) {
        let ctx = get_context(self);
        ctx.send_action(SimpleClientAction::LoginCharacter {
            character_id,
            character_name,
            account: account_name,
        });
    }

    async fn log(&mut self, message: String) {
        let script_id = self.script_id.clone();
        let ctx = get_context(self);
        ctx.send_action(SimpleClientAction::LogScriptMessage { script_id, message });
    }

    async fn do_movement_command(&mut self, motion: u32, speed: f32, hold_key: u32) {
        let ctx = get_context(self);
        ctx.send_action(SimpleClientAction::DoMovementCommand {
            motion,
            speed,
            hold_key,
        });
    }

    async fn stop_movement_command(&mut self, motion: u32, hold_key: u32) {
        let ctx = get_context(self);
        ctx.send_action(SimpleClientAction::StopMovementCommand { motion, hold_key });
    }

    async fn schedule_timer(&mut self, delay_secs: u64, name: String) -> u64 {
        let ctx = get_context(self);
        let timer_id = ctx.schedule_timer(delay_secs, name);
        timer_id_to_u64(timer_id)
    }

    async fn schedule_recurring(&mut self, interval_secs: u64, name: String) -> u64 {
        let ctx = get_context(self);
        let timer_id = ctx.schedule_recurring(interval_secs, name);
        timer_id_to_u64(timer_id)
    }

    async fn cancel_timer(&mut self, timer_id: u64) -> bool {
        let ctx = get_context(self);
        let timer_id = timer_id_from_u64(timer_id);
        ctx.cancel_timer(timer_id)
    }

    async fn check_timer(&mut self, timer_id: u64) -> bool {
        let ctx = get_context(self);
        let timer_id = timer_id_from_u64(timer_id);
        ctx.check_timer(timer_id)
    }

    async fn get_client_state(&mut self) -> gromnie::scripting::host::ClientState {
        use gromnie_client::client::SessionState;

        let ctx = get_context(self);
        let client_state = ctx.client_sync();

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

        let scene = convert_scene_to_wit(&client_state.scene);

        gromnie::scripting::host::ClientState { session, scene }
    }

    async fn get_event_time_millis(&mut self) -> u64 {
        use std::time::SystemTime;
        let now = SystemTime::now();
        now.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Convert TimerId to u64 for WASM ABI
fn timer_id_to_u64(timer_id: crate::TimerId) -> u64 {
    timer_id.into()
}

/// Convert u64 to TimerId
fn timer_id_from_u64(id: u64) -> crate::TimerId {
    id.into()
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
