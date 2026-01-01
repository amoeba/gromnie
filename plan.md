Go with Option 1 (Full Builder Pattern) because:

  1. Best UX: Clear, self-documenting API
  2. Flexibility: Easy to add new options without breaking changes
  3. Type safety: Builder catches missing required fields at compile time
  4. Rust idioms: Follows common Rust patterns (like tokio::net::TcpListener::bind())
  5. Removes complexity: No more None::<fn(u32) -> ClientConfig> type hints
  6. Clean consumer API: Factory trait is simpler than closures

  Implementation Plan

  1. Create new types (in new file client_runner_builder.rs):
    - ConsumerFactory trait
    - ConsumerContext struct
    - ClientMode enum
    - ClientRunnerBuilder struct
    - ClientRunner struct
  2. Update existing consumers to provide factory methods:
    - LoggingConsumer::from_factory()
    - TuiConsumer::from_factory(tx)
    - DiscordConsumer::from_factory(http, channel_id)
    - StatsConsumer::from_factory(stats, verbose)
    - AutoLoginConsumer::from_factory(character_name, verbose)
  3. Implement builder pattern:
    - Builder accumulates configuration
    - build() validates and constructs ClientRunner
    - ClientRunner::run() orchestrates execution
  4. Update call sites (gromnie-cli, gromnie-tui):
    - Replace run() calls with builder pattern
    - Remove FnConsumerBuilder usage
    - Simplify consumer creation
  5. Deprecate old APIs (optional):
    - Keep old functions for backward compatibility
    - Mark as #[deprecated]
    - Remove in next major version
  6. Internal cleanup:
    - EventBusManager created internally
    - Scripting integration handled internally (optional consumer)
    - All orchestration logic encapsulated

⏺ Summary

  I've analyzed your current client_runner architecture and worked up a comprehensive plan for simplification. Here's what I found:

  Current Issues

  - 5 different entry points (run(), run_client(), run_client_with_consumers(), etc.)
  - Type gymnastics with None::<fn(u32) -> ClientConfig>
  - Awkward consumer creation with FnConsumerBuilder requiring boxing
  - Inconsistent patterns between single and multi-client

  Proposed Solution: Full Builder Pattern

  The cleanest approach is Option 1 - a builder pattern that gives you exactly the API you described:

  // 1. Process CLI args, prepare objects
  let config = load_config();

  // 2. Create ClientRunner using builder
  let runner = ClientRunner::builder()
      .single_client(ClientConfig { ... })
      .consumer(LoggingConsumer::from_factory())
      .consumer_if(config.scripting.enabled, || ScriptConsumer::new(...))
      .with_shutdown(shutdown_rx)
      .build()?;

  // 3. Execute
  runner.run().await;

  Key improvements:
  1. ConsumerFactory trait - replaces the awkward closure-based FnConsumerBuilder
  2. Unified ClientMode enum - handles single, multi-dynamic, and multi-static configs
  3. Builder validation - catches missing configuration at compile time
  4. EventBus/scripting managed internally - no manual setup needed
  5. Clean type inference - no more None::<fn(u32) -> ClientConfig>
