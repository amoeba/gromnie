use anyhow::Result;
use wasmtime::component::Linker;

use super::wasm_script::{WasmScriptState, gromnie};
use crate::ScriptContext;
use gromnie_client::client::events::ClientAction;

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
        ctx.send_action(ClientAction::LoginCharacter {
            character_id,
            character_name,
            account: account_name,
        });
    }

    fn log(&mut self, message: String) {
        let script_id = self.script_id.clone();
        let ctx = get_context(self);
        ctx.send_action(ClientAction::LogScriptMessage { script_id, message });
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

    fn get_client_state(&mut self) -> gromnie::scripting::host::Client {
        let ctx = get_context(self);
        let state = ctx.client_state();

        // Map from boolean-based internal state to enum-based WIT state
        let connection_state = if state.is_ingame {
            gromnie::scripting::host::ClientState::Ingame
        } else if state.is_authenticated {
            gromnie::scripting::host::ClientState::Charselect
        } else {
            // TODO: Distinguish between connecting and patching
            gromnie::scripting::host::ClientState::Connecting
        };

        // Build character if we have one
        let character = if let (Some(id), Some(name)) = (state.character_id, &state.character_name)
        {
            Some(gromnie::scripting::host::WorldObject {
                id,
                name: name.clone(),
            })
        } else {
            None
        };

        gromnie::scripting::host::Client {
            state: connection_state,
            account: None, // TODO: Need to track account info in ClientStateSnapshot
            character,
        }
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
