pub mod debug;
pub mod game;

// Re-export the main view rendering functions
pub use debug::render_debug_view;
pub use game::render_game_view;
