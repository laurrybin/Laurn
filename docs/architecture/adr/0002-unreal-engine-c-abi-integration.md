# ADR 0002: Unreal Engine C ABI Integration

**Date:** 2026-08-25  
**Status:** Accepted  

## Context
Unreal Engine is the primary target for LAURN integration. Unreal Engine operates natively in C++. We must establish a boundary between the Rust core and the Unreal Engine host.

## Decision
We will expose a flat, memory-safe C ABI (`bindings/c`) from the Rust core, rather than attempting to generate native C++ bindings or using Unreal-specific C++ macros within the core repository. The Unreal Engine plugin (`unreal/Laurn`) will consume this C ABI dynamically/statically.

## Consequences
- **Positive:** The Rust core remains completely decoupled from Unreal Engine headers and build systems (UnrealBuildTool).
- **Positive:** The C ABI can easily be reused for other engines (e.g., Unity via C#, Godot via GDExtension).
- **Negative:** Crossing the FFI boundary requires manual memory management rules and boilerplate serialization.
- **Negative:** Structs must be declared with `#[repr(C)]`, limiting the use of advanced Rust enums at the boundary.
