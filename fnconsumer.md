# FnConsumerBuilder Pattern - Alternatives Analysis

## Current Approach

```rust
// What we have now
let consumer_builder = if config.scripting.enabled {
    let scripting_config = config.scripting.clone();
    FnConsumerBuilder::new(move |_, _, action_tx| {
        Box::new(CompositeConsumer::new(vec![
            Box::new(LoggingConsumer::new(action_tx.clone())),
            Box::new(create_script_consumer(action_tx, &scripting_config)),
        ]))
    })
} else {
    FnConsumerBuilder::new(|_, _, action_tx| {
        Box::new(LoggingConsumer::new(action_tx))
    })
};
```

**Issues:**
- Requires boxing every closure (heap allocation)
- Complex nested closure syntax
- Inconsistent with multi-client (single uses `None`, multi uses `Some(closure)`)
- Type gymnastics with `None::<fn(_)->_>` needed

---

## Option 1: Separate APIs (Revert partially)

Keep `run_client_with_consumers` for single-client, `run_multi_client()` for multi-client.

```rust
// Single client - what we had before
if config.scripting.enabled {
    run_client_with_consumers(client_config, event_bus_manager, |action_tx| {
        vec![
            Box::new(LoggingConsumer::new(action_tx.clone())),
            Box::new(create_script_consumer(action_tx, &config.scripting)),
        ]
    }, None).await;
} else {
    run_client(client_config, event_bus_manager, LoggingConsumer::new, None).await;
}

// Multi-client - unchanged
run_multi_client(config, factory, generator, None).await;
```

**Pros:**
- Simple, no boxing overhead
- Clear, familiar API
- No type gymnastics
- Compile errors are straightforward

**Cons:**
- Two different APIs to maintain
- Code duplication in caller (need if/else for runner selection)
- Less "unified"

---

## Option 2: Accept closures directly in `run()`

Instead of a trait, make `run()` accept closures directly via overloaded functions or different entry points.

```rust
// Single-client entry point
pub async fn run_single<F>(
    client_config: ClientConfig,
    consumer_fn: F,
    shutdown_rx: Option<watch::Receiver<bool>>,
) -> RunResult
where
    F: FnOnce(mpsc::UnboundedSender<ClientAction>) -> Box<dyn EventConsumer> + Send + 'static,
{
    // ... implementation
}

// Multi-client entry point
pub async fn run_multi<G>(
    config: MultiClientConfig,
    consumer_fn: Arc<dyn FnConsumerFactory>,  // Keep existing factory for multi-client
    client_config_generator: G,
    shutdown_rx: Option<watch::Receiver<bool>>,
) -> Arc<MultiClientStats>
where
    G: Fn(u32) -> ClientConfig + Send + Sync + 'static,
{
    // ... existing implementation
}

// Usage
if config.num_clients == 1 {
    run_single(client_config, |action_tx| {
        Box::new(LoggingConsumer::new(action_tx))
    }, None).await;
} else {
    run_multi(config, factory, generator, None).await;
}
```

**Pros:**
- No boxing needed for single-client
- Cleaner single-client API
- Multi-client unchanged

**Cons:**
- Still two APIs
- Users need to decide which to call

---

## Option 3: Simplified FnConsumerBuilder (small refactor)

Remove boxing by using `Arc<dyn Fn>` but add convenience methods:

```rust
pub struct FnConsumerBuilder {
    f: Arc<dyn Fn(u32, &ClientConfig, mpsc::UnboundedSender<ClientAction>) -> Box<dyn EventConsumer> + Send + Sync>,
}

impl FnConsumerBuilder {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(...) -> Box<dyn EventConsumer> + Send + Sync + 'static,
    {
        Self { f: Arc::new(f) }
    }

    /// Convenience for simple single-consumer closures that don't need client_id
    pub fn new_simple<F>(f: F) -> Self
    where
        F: Fn(mpsc::UnboundedSender<ClientAction>) -> Box<dyn EventConsumer> + Send + Sync + 'static,
    {
        Self { f: Arc::new(move |client_id, config, tx| (f)(tx)) }
    }
}

// Usage - no boxing needed for simple cases
let consumer_builder = FnConsumerBuilder::new_simple(|action_tx| {
    Box::new(LoggingConsumer::new(action_tx))
});

// Still uses boxing internally but hides it
```

**Pros:**
- Hides boxing complexity
- Convenience method for common case
- Multi-client can still use full closure

**Cons:**
- Still boxing internally
- Slightly more complex API

---

## Option 4: Defer consumer creation to a struct that holds captured vars

Create a concrete struct that holds captured state, avoiding boxing the closure:

```rust
pub struct ConsumerBuilderState {
    script_config: Option<ScriptingConfig>,
    verbose: bool,
}

pub struct ConsumerBuilder {
    state: Arc<ConsumerBuilderState>,
    f: fn(&ConsumerBuilderState, u32, &ClientConfig, mpsc::UnboundedSender<ClientAction>) -> Box<dyn EventConsumer> + Send + Sync + 'static,
}

impl ConsumerBuilder {
    pub fn new<F>(f: F) -> Self where F: ... + 'static {
        Self { state: Arc::new(ConsumerBuilderState::default()), f }
    }

    pub fn with_config(mut self, config: ScriptingConfig) -> Self {
        self.state = Arc::new(ConsumerBuilderState { script_config: Some(config), ..Default::default() });
        self
    }
}

impl ConsumerBuilder for ConsumerBuilder {
    fn build(&self, ...) -> Box<dyn EventConsumer> {
        (self.f)(&self.state, ...)
    }
}

// Usage
let consumer_builder = ConsumerBuilder::new(|state, _, _, action_tx| {
    let mut consumers = vec![Box::new(LoggingConsumer::new(action_tx.clone()))];
    if let Some(scripting_config) = &state.scripting_config {
        consumers.push(Box::new(create_script_consumer(action_tx, scripting_config)));
    }
    Box::new(CompositeConsumer::new(consumers))
}).with_config(scripting_config);
```

**Pros:**
- Avoids boxing the closure
- State is stored in a struct, not captured
- Cleaner than boxed closures

**Cons:**
- Still complex
- Requires boilerplate struct to hold state
- Over-engineered for simple cases

---

## Option 5: Just keep the original if/else (simplest)

Revert to the original approach - different branches call different runners:

```rust
// Single with one consumer
run_client(client_config, event_bus_manager, LoggingConsumer::new, None).await;

// Single with multiple consumers
run_client_with_consumers(client_config, event_bus_manager, |action_tx| {
    vec![Box::new(LoggingConsumer::new(action_tx)), Box::new(ScriptConsumer::new(action_tx))]
}, None).await;

// Multi-client
run_multi_client(config, factory, generator, None).await;
```

**Pros:**
- Simple, clear, no boxing
- No type gymnastics
- Each API is optimized for its use case

**Cons:**
- Three different APIs
- Code duplication when branching between them
- Less "unified"

---

## Option 6: Builder pattern with chaining

```rust
pub struct ConsumerList {
    consumers: Vec<Box<dyn EventConsumer>>,
}

impl ConsumerList {
    pub fn new() -> Self { Self { consumers: vec![] } }
    pub fn add(mut self, consumer: Box<dyn EventConsumer>) -> Self {
        self.consumers.push(consumer);
        self
    }
    pub fn build(self, action_tx: UnboundedSender<ClientAction>) -> Box<dyn EventConsumer> {
        Box::new(CompositeConsumer::new(self.consumers))
    }
}

// Usage
let consumer_builder = FnConsumerBuilder::new(|_, _, action_tx| {
    let mut list = ConsumerList::new();
    list = list.add(Box::new(LoggingConsumer::new(action_tx.clone())));
    if config.scripting.enabled {
        list = list.add(Box::new(create_script_consumer(action_tx, &config.scripting)));
    }
    list.build(action_tx)
});
```

**Pros:**
- Familiar builder pattern
- No boxing if we return the struct directly

**Cons:**
- More verbose than current approach
- Adds another struct to the API surface
- Same core issue (boxing for FnConsumerBuilder wrapper)

---

## Summary Table

| Option | Boxing? | Unified API? | Complexity | Verdict |
|--------|----------|--------------|------------|----------|
| 1. Separate APIs | No | No | Low | Simple, but duplicated |
| 2. Overloaded functions | No | No | Low | Clean, but still two APIs |
| 3. Simplified FnConsumerBuilder | Yes (hidden) | Yes | Medium | Good compromise |
| 4. Struct-based state | No | Yes | High | Over-engineered |
| 5. Original if/else | No | No | Low | Simplest, but not unified |
| 6. Builder chaining | Maybe | Yes | Medium | Verbose, but clean |

---

## Recommendation

**Option 3 (Simplified FnConsumerBuilder)** seems like the best middle ground:
- Hides the boxing complexity
- Provides convenience method for simple cases (`new_simple()`)
- Still flexible for complex cases

Or, if we're being pragmatic, **Option 5 (Original if/else)** is actually fine:
- The "unification" is really just moving code from if/else in caller into a single run() function
- The complexity hasn't been reduced, just relocated
- Sometimes simpler is better.
