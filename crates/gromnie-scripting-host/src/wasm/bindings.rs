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
    fn send_chat<'life0, 'async_trait>(
        &'life0 mut self,
        message: wasmtime::component::__internal::String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.send_chat(message);
        Box::pin(std::future::ready(()))
    }

    fn send_tell<'life0, 'async_trait>(
        &'life0 mut self,
        recipient: wasmtime::component::__internal::String,
        message: wasmtime::component::__internal::String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.send_tell(recipient, message);
        Box::pin(std::future::ready(()))
    }

    fn open_trade<'life0, 'async_trait>(
        &'life0 mut self,
        partner_id: u32,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.open_trade(partner_id);
        Box::pin(std::future::ready(()))
    }

    fn add_to_trade<'life0, 'async_trait>(
        &'life0 mut self,
        item_id: u32,
        slot: u32,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.add_to_trade(item_id, slot);
        Box::pin(std::future::ready(()))
    }

    fn accept_trade<'life0, 'async_trait>(
        &'life0 mut self,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.accept_trade();
        Box::pin(std::future::ready(()))
    }

    fn decline_trade<'life0, 'async_trait>(
        &'life0 mut self,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.decline_trade();
        Box::pin(std::future::ready(()))
    }

    fn reset_trade<'life0, 'async_trait>(
        &'life0 mut self,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.reset_trade();
        Box::pin(std::future::ready(()))
    }

    fn close_trade<'life0, 'async_trait>(
        &'life0 mut self,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.close_trade();
        Box::pin(std::future::ready(()))
    }

    fn cast_targeted_spell<'life0, 'async_trait>(
        &'life0 mut self,
        target_id: u32,
        spell_id: u32,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.cast_targeted_spell(target_id, spell_id);
        Box::pin(std::future::ready(()))
    }

    fn cast_untargeted_spell<'life0, 'async_trait>(
        &'life0 mut self,
        spell_id: u32,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.cast_untargeted_spell(spell_id);
        Box::pin(std::future::ready(()))
    }

    fn login_character<'life0, 'async_trait>(
        &'life0 mut self,
        account_name: wasmtime::component::__internal::String,
        character_id: u32,
        character_name: wasmtime::component::__internal::String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.send_action(SimpleClientAction::LoginCharacter {
            character_id,
            character_name,
            account: account_name,
        });
        Box::pin(std::future::ready(()))
    }

    fn log<'life0, 'async_trait>(
        &'life0 mut self,
        message: wasmtime::component::__internal::String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let script_id = self.script_id.clone();
        let ctx = get_context(self);
        ctx.send_action(SimpleClientAction::LogScriptMessage { script_id, message });
        Box::pin(std::future::ready(()))
    }

    fn do_movement_command<'life0, 'async_trait>(
        &'life0 mut self,
        motion: u32,
        speed: f32,
        hold_key: u32,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.send_action(SimpleClientAction::DoMovementCommand {
            motion,
            speed,
            hold_key,
        });
        Box::pin(std::future::ready(()))
    }

    fn stop_movement_command<'life0, 'async_trait>(
        &'life0 mut self,
        motion: u32,
        hold_key: u32,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        ctx.send_action(SimpleClientAction::StopMovementCommand { motion, hold_key });
        Box::pin(std::future::ready(()))
    }

    fn schedule_timer<'life0, 'async_trait>(
        &'life0 mut self,
        delay_secs: u64,
        name: wasmtime::component::__internal::String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = u64> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        let timer_id = ctx.schedule_timer(delay_secs, name);
        Box::pin(std::future::ready(timer_id_to_u64(timer_id)))
    }

    fn schedule_recurring<'life0, 'async_trait>(
        &'life0 mut self,
        interval_secs: u64,
        name: wasmtime::component::__internal::String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = u64> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        let timer_id = ctx.schedule_recurring(interval_secs, name);
        Box::pin(std::future::ready(timer_id_to_u64(timer_id)))
    }

    fn cancel_timer<'life0, 'async_trait>(
        &'life0 mut self,
        timer_id: u64,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = bool> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        let timer_id = timer_id_from_u64(timer_id);
        Box::pin(std::future::ready(ctx.cancel_timer(timer_id)))
    }

    fn check_timer<'life0, 'async_trait>(
        &'life0 mut self,
        timer_id: u64,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = bool> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let ctx = get_context(self);
        let timer_id = timer_id_from_u64(timer_id);
        Box::pin(std::future::ready(ctx.check_timer(timer_id)))
    }

    fn get_client_state<'life0, 'async_trait>(
        &'life0 mut self,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = gromnie::scripting::host::ClientState>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
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

        let result = gromnie::scripting::host::ClientState { session, scene };
        Box::pin(std::future::ready(result))
    }

    fn get_event_time_millis<'life0, 'async_trait>(
        &'life0 mut self,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = u64> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        use std::time::SystemTime;
        let now = SystemTime::now();
        let millis = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Box::pin(std::future::ready(millis))
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
