# Gromnie Scripting System Cleanup Plan

## Overview
This document outlines the plan to improve code cleanliness and reduce duplication in Gromnie's scripting system while maintaining functionality through comprehensive testing.

## Phase 1: Test Infrastructure Setup

### Goal: Create a robust test system to verify scripting functionality before making changes

#### Tasks:
1. **Create a test WASM script** that exercises all major functionality:
   - Script lifecycle (load/unload)
   - Event handling
   - Timer operations
   - Host function calls (chat, login, etc.)
   - State access

2. **Build a test harness** in the scripting host:
   - Mock client state
   - Mock event system
   - Mock action channel
   - Timer simulation

3. **Create integration tests** that:
   - Load and execute WASM scripts
   - Verify event dispatching
   - Test timer functionality
   - Validate host function calls
   - Test script lifecycle

4. **Establish baseline** by running tests against current implementation

## Phase 2: Code Cleanup Implementation

### Goal: Address duplication and improve code quality while maintaining test coverage

#### Specific Changes:

**1. Context Creation Refactoring**
- Create `with_script_context()` helper method
- Replace all manual `unsafe { ScriptContext::new(...) }` calls
- Ensure proper context cleanup

**2. WASM Call Pattern Consolidation**
- Create `with_context()` private method in `WasmScript`
- Standardize error handling for WASM calls
- Ensure consistent logging

**3. Event System Improvements**
- Move discriminant logic into `EventFilter` methods
- Pre-compute event subscriptions during registration
- Improve type safety in event conversion

**4. Safety Improvements**
- Replace unsafe transmutes with proper newtype patterns
- Better isolate unsafe code blocks
- Add safety documentation

**5. Script Lifecycle Management**
- Make unload/reload logic generic (not WASM-specific)
- Improve polymorphism in script handling
- Better document limitations

## Phase 3: Verification and Documentation

### Goal: Ensure all changes work correctly and are well-documented

#### Tasks:
1. Run all tests after each change
2. Update documentation to reflect new patterns
3. Add inline documentation for new helper methods
4. Create examples showing proper usage
5. Update README with development guidelines

## Implementation Strategy

### Branch Strategy:
- Create branch: `scripting-cleanup-test-infra`
- Commit test infrastructure first
- Verify baseline functionality
- Then create branch: `scripting-cleanup-implementation`
- Implement changes incrementally
- Commit after each logical change with descriptive messages

### Commit Guidelines:
- Small, focused commits
- Descriptive commit messages following: "[Area] What changed and why"
- Reference this cleanup plan in commit messages
- Include test results where applicable

## Success Criteria

1. All existing functionality preserved (verified by tests)
2. Code duplication significantly reduced
3. Code quality metrics improved
4. Comprehensive test coverage established
5. Clear documentation of new patterns and limitations

## Timeline

1. **Day 1-2**: Test infrastructure setup and baseline verification
2. **Day 3-5**: Incremental cleanup implementation
3. **Day 6**: Final testing and documentation
4. **Day 7**: Code review and merge preparation

## Risk Mitigation

- Frequent testing after each change
- Small, isolated commits for easy rollback
- Comprehensive test coverage before making changes
- Clear documentation of breaking changes (if any)
