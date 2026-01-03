//! Integration tests for the scene-based client architecture
//!
//! These tests verify that the scene-based state management is working correctly,
//! including scene transitions, state consistency, and event emission.

#[cfg(test)]
mod scene_tests {
    use gromnie_client::client::{
        Scene, ConnectingScene, ConnectingProgress, PatchingProgress, 
        CharacterSelectScene, InWorldScene, ErrorScene, ClientError, EnteringWorldState
    };
    use gromnie_events::CharacterInfo;

    // ============ ConnectingScene Tests ============

    #[test]
    fn test_connecting_scene_creation() {
        let scene = ConnectingScene::new();
        assert_eq!(scene.connect_progress, ConnectingProgress::Initial);
        assert_eq!(scene.patch_progress, PatchingProgress::NotStarted);
    }

    #[test]
    fn test_connecting_progress_transitions() {
        let mut scene = ConnectingScene::new();
        
        // Initial
        assert_eq!(scene.connect_progress, ConnectingProgress::Initial);
        
        // LoginRequestSent
        scene.connect_progress = ConnectingProgress::LoginRequestSent;
        assert_eq!(scene.connect_progress, ConnectingProgress::LoginRequestSent);
        
        // ConnectRequestReceived
        scene.connect_progress = ConnectingProgress::ConnectRequestReceived;
        assert_eq!(scene.connect_progress, ConnectingProgress::ConnectRequestReceived);
        
        // ConnectResponseSent
        scene.connect_progress = ConnectingProgress::ConnectResponseSent;
        assert_eq!(scene.connect_progress, ConnectingProgress::ConnectResponseSent);
    }

    #[test]
    fn test_patching_progress_transitions() {
        let mut scene = ConnectingScene::new();
        
        // NotStarted
        assert_eq!(scene.patch_progress, PatchingProgress::NotStarted);
        
        // WaitingForDDD
        scene.patch_progress = PatchingProgress::WaitingForDDD;
        assert_eq!(scene.patch_progress, PatchingProgress::WaitingForDDD);
        
        // ReceivedDDD
        scene.patch_progress = PatchingProgress::ReceivedDDD;
        assert_eq!(scene.patch_progress, PatchingProgress::ReceivedDDD);
        
        // SentDDDResponse
        scene.patch_progress = PatchingProgress::SentDDDResponse;
        assert_eq!(scene.patch_progress, PatchingProgress::SentDDDResponse);
        
        // Complete
        scene.patch_progress = PatchingProgress::Complete;
        assert_eq!(scene.patch_progress, PatchingProgress::Complete);
    }

    #[test]
    fn test_connecting_scene_timeout_detection() {
        let scene = ConnectingScene::new();
        let timeout = std::time::Duration::from_millis(1);
        
        // Should not timeout immediately
        assert!(!scene.has_timed_out(timeout));
        
        // Wait a bit and check again
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(scene.has_timed_out(timeout));
    }

    #[test]
    fn test_connecting_scene_retry_timing() {
        let mut scene = ConnectingScene::new();
        let retry_interval = std::time::Duration::from_millis(1);
        
        // Should not retry immediately
        assert!(!scene.should_retry(retry_interval));
        
        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(5));
        
        // Should retry now
        assert!(scene.should_retry(retry_interval));
        
        // Update retry time
        scene.update_retry_time();
        
        // Should not retry immediately after update
        assert!(!scene.should_retry(retry_interval));
    }

    #[test]
    fn test_connecting_scene_reset() {
        let mut scene = ConnectingScene::new();
        
        // Progress through states
        scene.connect_progress = ConnectingProgress::LoginRequestSent;
        scene.patch_progress = PatchingProgress::WaitingForDDD;
        
        // Reset
        scene.reset();
        
        // Should be back to initial state
        assert_eq!(scene.connect_progress, ConnectingProgress::Initial);
        assert_eq!(scene.patch_progress, PatchingProgress::NotStarted);
    }

    // ============ CharacterSelectScene Tests ============

    #[test]
    fn test_character_select_scene_creation() {
        let account_name = "TestAccount".to_string();
        let characters = vec![
            CharacterInfo {
                name: "Char1".to_string(),
                id: 1,
                delete_pending: false,
            },
            CharacterInfo {
                name: "Char2".to_string(),
                id: 2,
                delete_pending: false,
            },
        ];
        
        let scene = CharacterSelectScene::new(account_name.clone(), characters.clone());
        
        assert_eq!(scene.account_name, account_name);
        assert_eq!(scene.characters.len(), 2);
        assert!(scene.entering_world.is_none());
    }

    #[test]
    fn test_character_select_begin_entering_world() {
        let scene = CharacterSelectScene::new("TestAccount".to_string(), vec![]);
        let mut scene = scene;
        
        assert!(!scene.is_entering_world());
        
        scene.begin_entering_world(123, "MyChar".to_string(), "TestAccount".to_string());
        
        assert!(scene.is_entering_world());
        let entering = scene.entering_world.as_ref().unwrap();
        assert_eq!(entering.character_id, 123);
        assert_eq!(entering.character_name, "MyChar");
        assert!(!entering.login_complete);
    }

    #[test]
    fn test_character_select_mark_login_complete() {
        let mut scene = CharacterSelectScene::new("TestAccount".to_string(), vec![]);
        scene.begin_entering_world(123, "MyChar".to_string(), "TestAccount".to_string());
        
        assert!(!scene.entering_world.as_ref().unwrap().login_complete);
        
        scene.mark_login_complete();
        
        assert!(scene.entering_world.as_ref().unwrap().login_complete);
    }

    #[test]
    fn test_character_select_clear_entering_world() {
        let mut scene = CharacterSelectScene::new("TestAccount".to_string(), vec![]);
        scene.begin_entering_world(123, "MyChar".to_string(), "TestAccount".to_string());
        
        assert!(scene.is_entering_world());
        
        scene.clear_entering_world();
        
        assert!(!scene.is_entering_world());
    }

    // ============ InWorldScene Tests ============

    #[test]
    fn test_in_world_scene_creation() {
        let scene = InWorldScene::new(456, "MyCharacter".to_string());
        
        assert_eq!(scene.character_id, 456);
        assert_eq!(scene.character_name, "MyCharacter");
    }

    // ============ ErrorScene Tests ============

    #[test]
    fn test_error_scene_creation_with_retry() {
        let error = ClientError::ConnectionFailed("Connection lost".to_string());
        let scene = ErrorScene::new(error, true);
        
        assert!(scene.can_retry);
    }

    #[test]
    fn test_error_scene_creation_without_retry() {
        let error = ClientError::LoginTimeout;
        let scene = ErrorScene::new(error, false);
        
        assert!(!scene.can_retry);
    }

    // ============ Scene Enum Tests ============

    #[test]
    fn test_scene_as_connecting() {
        let connecting = ConnectingScene::new();
        let scene = Scene::Connecting(connecting);
        
        assert!(scene.as_connecting().is_some());
        assert!(scene.as_character_select().is_none());
        assert!(scene.as_in_world().is_none());
        assert!(scene.as_error().is_none());
    }

    #[test]
    fn test_scene_as_character_select() {
        let char_select = CharacterSelectScene::new("Account".to_string(), vec![]);
        let scene = Scene::CharacterSelect(char_select);
        
        assert!(scene.as_connecting().is_none());
        assert!(scene.as_character_select().is_some());
        assert!(scene.as_in_world().is_none());
        assert!(scene.as_error().is_none());
    }

    #[test]
    fn test_scene_as_in_world() {
        let in_world = InWorldScene::new(789, "Char".to_string());
        let scene = Scene::InWorld(in_world);
        
        assert!(scene.as_connecting().is_none());
        assert!(scene.as_character_select().is_none());
        assert!(scene.as_in_world().is_some());
        assert!(scene.as_error().is_none());
    }

    #[test]
    fn test_scene_as_error() {
        let error = ClientError::PatchingFailed("Patch failed".to_string());
        let error_scene = ErrorScene::new(error, true);
        let scene = Scene::Error(error_scene);
        
        assert!(scene.as_connecting().is_none());
        assert!(scene.as_character_select().is_none());
        assert!(scene.as_in_world().is_none());
        assert!(scene.as_error().is_some());
    }

    #[test]
    fn test_scene_can_retry_error() {
        let error = ClientError::ConnectionFailed("Lost".to_string());
        let error_scene = ErrorScene::new(error, true);
        let scene = Scene::Error(error_scene);
        
        assert!(scene.can_retry());
    }

    #[test]
    fn test_scene_cannot_retry_non_error() {
        let connecting = ConnectingScene::new();
        let scene = Scene::Connecting(connecting);
        
        assert!(!scene.can_retry());
    }

    // ============ Mutable Scene Access Tests ============

    #[test]
    fn test_scene_as_connecting_mut() {
        let connecting = ConnectingScene::new();
        let mut scene = Scene::Connecting(connecting);
        
        if let Some(connecting) = scene.as_connecting_mut() {
            connecting.connect_progress = ConnectingProgress::LoginRequestSent;
        }
        
        let connecting = scene.as_connecting().unwrap();
        assert_eq!(connecting.connect_progress, ConnectingProgress::LoginRequestSent);
    }

    #[test]
    fn test_scene_as_character_select_mut() {
        let char_select = CharacterSelectScene::new("Account".to_string(), vec![]);
        let mut scene = Scene::CharacterSelect(char_select);
        
        if let Some(char_select) = scene.as_character_select_mut() {
            char_select.begin_entering_world(999, "TestChar".to_string(), "TestAccount".to_string());
        }
        
        let char_select = scene.as_character_select().unwrap();
        assert!(char_select.is_entering_world());
    }

    // ============ ClientError Enum Tests ============

    #[test]
    fn test_client_error_variants() {
        // Test that different error variants can be created and used
        let conn_failed = ClientError::ConnectionFailed("failed".to_string());
        let patch_failed = ClientError::PatchingFailed("failed".to_string());
        let login_timeout = ClientError::LoginTimeout;
        let patch_timeout = ClientError::PatchingTimeout;
        
        // Verify they are different variants
        match conn_failed {
            ClientError::ConnectionFailed(_) => assert!(true),
            _ => panic!("Expected ConnectionFailed variant"),
        }
        
        match patch_failed {
            ClientError::PatchingFailed(_) => assert!(true),
            _ => panic!("Expected PatchingFailed variant"),
        }
        
        match login_timeout {
            ClientError::LoginTimeout => assert!(true),
            _ => panic!("Expected LoginTimeout variant"),
        }
        
        match patch_timeout {
            ClientError::PatchingTimeout => assert!(true),
            _ => panic!("Expected PatchingTimeout variant"),
        }
    }

    // ============ EnteringWorldState Tests ============

    #[test]
    fn test_entering_world_state_creation() {
        let state = EnteringWorldState {
            character_id: 111,
            character_name: "TestChar".to_string(),
            account: "TestAccount".to_string(),
            login_complete: false,
        };
        
        assert_eq!(state.character_id, 111);
        assert_eq!(state.character_name, "TestChar");
        assert_eq!(state.account, "TestAccount");
        assert!(!state.login_complete);
    }
}

// ============ Session Architecture Tests ============

#[cfg(test)]
mod session_tests {
    use gromnie_client::client::{SessionState, ClientSession};

    #[test]
    fn test_session_state_enum_variants() {
        let _auth_login = SessionState::AuthLoginRequest;
        let _auth_connect = SessionState::AuthConnectResponse;
        let _auth_connected = SessionState::AuthConnected;
        let _world_connected = SessionState::WorldConnected;
        let _termination = SessionState::TerminationStarted;
        
        // Just verify they can be created
        assert!(true);
    }

    #[test]
    fn test_client_session_creation() {
        let session = ClientSession::new(SessionState::AuthLoginRequest);
        
        assert_eq!(session.state, SessionState::AuthLoginRequest);
        assert!(session.connection.is_none());
        assert!(session.metadata.started_at.is_some());
        assert_eq!(session.metadata.connect_attempt_count, 0);
    }

    #[test]
    fn test_client_session_transition() {
        let mut session = ClientSession::new(SessionState::AuthLoginRequest);
        
        assert_eq!(session.state, SessionState::AuthLoginRequest);
        
        session.transition_to(SessionState::AuthConnectResponse);
        assert_eq!(session.state, SessionState::AuthConnectResponse);
        
        session.transition_to(SessionState::AuthConnected);
        assert_eq!(session.state, SessionState::AuthConnected);
        
        session.transition_to(SessionState::WorldConnected);
        assert_eq!(session.state, SessionState::WorldConnected);
    }
}

// ============ Scene Transition Flow Tests ============

#[cfg(test)]
mod transition_flow_tests {
    use gromnie_client::client::{
        Scene, ConnectingScene, CharacterSelectScene, InWorldScene,
        SessionState, ClientSession, ConnectingProgress, PatchingProgress,
    };
    use gromnie_events::CharacterInfo;

    #[test]
    fn test_full_login_flow_transitions() {
        // Step 1: Initial state - Connecting scene
        let mut session = ClientSession::new(SessionState::AuthLoginRequest);
        let mut scene = Scene::Connecting(ConnectingScene::new());
        
        // Verify initial state
        assert_eq!(session.state, SessionState::AuthLoginRequest);
        assert!(scene.as_connecting().is_some());
        
        // Step 2: Progress connecting
        if let Some(connecting) = scene.as_connecting_mut() {
            connecting.connect_progress = ConnectingProgress::LoginRequestSent;
        }
        
        // Step 3: Receive ConnectRequest
        if let Some(connecting) = scene.as_connecting_mut() {
            connecting.connect_progress = ConnectingProgress::ConnectRequestReceived;
        }
        session.transition_to(SessionState::AuthConnectResponse);
        
        // Step 4: Send ConnectResponse, start patching
        if let Some(connecting) = scene.as_connecting_mut() {
            connecting.connect_progress = ConnectingProgress::ConnectResponseSent;
            connecting.patch_progress = PatchingProgress::WaitingForDDD;
        }
        session.transition_to(SessionState::AuthConnected);
        
        // Step 5: Receive DDD
        if let Some(connecting) = scene.as_connecting_mut() {
            connecting.patch_progress = PatchingProgress::ReceivedDDD;
        }
        
        // Step 6: Send DDD response
        if let Some(connecting) = scene.as_connecting_mut() {
            connecting.patch_progress = PatchingProgress::SentDDDResponse;
        }
        
        // Step 7: Receive character list - transition to CharacterSelect scene
        let characters = vec![
            CharacterInfo {
                name: "TestChar".to_string(),
                id: 1,
                delete_pending: false,
            },
        ];
        scene = Scene::CharacterSelect(CharacterSelectScene::new(
            "TestAccount".to_string(),
            characters.clone(),
        ));
        
        // Verify we're in the right state
        assert_eq!(session.state, SessionState::AuthConnected);
        assert!(scene.as_character_select().is_some());
        
        // Step 8: User selects character
        if let Some(char_select) = scene.as_character_select_mut() {
            char_select.begin_entering_world(1, "TestChar".to_string(), "TestAccount".to_string());
        }
        
        // Step 9: Receive Character_LoginCompleteNotification
        if let Some(char_select) = scene.as_character_select_mut() {
            char_select.mark_login_complete();
        }
        
        // Step 10: Transition to InWorld
        scene = Scene::InWorld(InWorldScene::new(1, "TestChar".to_string()));
        session.transition_to(SessionState::WorldConnected);
        
        // Verify final state
        assert_eq!(session.state, SessionState::WorldConnected);
        assert!(scene.as_in_world().is_some());
        
        let in_world = scene.as_in_world().unwrap();
        assert_eq!(in_world.character_id, 1);
        assert_eq!(in_world.character_name, "TestChar");
    }

    #[test]
    fn test_connection_timeout_error_recovery() {
        let mut scene = Scene::Connecting(ConnectingScene::new());
        
        // Simulate timeout
        if let Some(connecting) = scene.as_connecting_mut() {
            // Artificially age the start time to trigger timeout
            connecting.started_at = std::time::Instant::now() - std::time::Duration::from_secs(25);
        }
        
        // Verify timeout is detected
        let timeout_detected = scene.as_connecting()
            .map(|c| c.has_timed_out(std::time::Duration::from_secs(20)))
            .unwrap_or(false);
        assert!(timeout_detected);
        
        // Retry: transition back to Connecting for reconnection
        let session = ClientSession::new(SessionState::AuthLoginRequest);
        let scene = Scene::Connecting(ConnectingScene::new());
        
        // Verify we're ready to retry
        assert_eq!(session.state, SessionState::AuthLoginRequest);
        assert!(scene.as_connecting().is_some());
    }
}
