---
title: Performance
---

# Performance

> **⚠️ Work in Progress:** Vize is under active development and is not yet ready for production use. Benchmark numbers are from development builds and may change.

Vize achieves significant performance improvements over the standard JavaScript-based Vue compiler by leveraging Rust's zero-cost abstractions and native multi-threading. Speed is not a nice-to-have — it is a prerequisite for developer experience.

## Benchmark Environment

| | |
|--|--|
| **Machine** | MacBook Pro (M2 Max, 12 cores, 96 GB RAM) |
| **OS** | macOS (Darwin 24.3.0) |
| **Node.js** | v24.13.0 |
| **Vite** | v8.0.0-beta.15 (Rolldown) |
| **Vue** | v3.6.0-beta.1 |

## Benchmark: 15,000 SFC Files

Compiling **15,000 Vue SFC files** (36.9 MB total):

| | @vue/compiler-sfc | Vize | Speedup |
|--|-------------------|------|---------|
| **Single Thread** | 10.52s | 3.82s | **2.8x** |
| **Multi Thread** | 3.71s | 380ms | **9.8x** |
| **compiler-sfc ST vs Vize MT** | 10.52s | 380ms | **27.7x** |

The single-threaded improvement comes from Rust's zero-cost abstractions (no GC, no JIT warmup, cache-friendly memory layout). The multi-threaded improvement comes from Rayon's work-stealing thread pool, which scales near-linearly with CPU core count.

### Scaling Behavior

| Files | Vize (1 thread) | Vize (8 threads) | Parallel Speedup |
|-------|-----------------|-----------------|-----------------|
| 100 | 44ms | 12ms | 3.7x |
| 1,000 | 443ms | 73ms | 6.1x |
| 5,000 | 2.2s | 198ms | 11.1x |
| 15,000 | 6.65s | 498ms | 13.4x |

The super-linear scaling at higher file counts is due to better amortization of thread pool startup costs and improved CPU cache utilization when all cores are saturated.

## Why Rust?

### Zero-Cost Abstractions

Rust's ownership model eliminates garbage collection pauses. The compiler processes AST nodes through arena allocation (`vize_carton`), avoiding per-node heap allocations. This means:

- **No GC pauses** — In V8-based compilers, garbage collection can cause unpredictable latency spikes. Vize has zero GC overhead.
- **No JIT warmup** — V8's JIT compiler needs time to optimize hot paths. Vize runs at full speed from the first instruction.
- **Predictable performance** — Rust's ahead-of-time compilation means performance is consistent across runs, not dependent on V8's optimization heuristics.

### Native Multi-Threading

Vize uses [Rayon](https://docs.rs/rayon) for data-parallel compilation. Each SFC file is compiled independently, making the workload embarrassingly parallel. Rayon's work-stealing scheduler ensures optimal core utilization:

```rust
// Simplified: parallel compilation of all .vue files
files.par_iter().map(|file| {
    let arena = Bump::new();
    let ast = parse(file, &arena);
    let analyzed = analyze(ast, &arena);
    compile(analyzed, &arena)
}).collect()
```

The work-stealing approach means that if one file is significantly larger than others, idle threads will steal work from the busy thread's queue, maintaining near-perfect load balancing.

### Efficient Memory Layout

Rust's struct layout and enum discriminants are compact. The AST representation in `vize_relief` is cache-friendly, reducing memory bandwidth bottlenecks:

- **Enum discriminants** — Rust enums are sized to the smallest type that fits the discriminant. A `NodeKind` with 20 variants uses a single byte, not a heap-allocated string.
- **Struct packing** — Rust automatically reorders struct fields for optimal alignment, minimizing padding bytes.
- **No object headers** — Unlike JavaScript objects (which carry prototype chains, property maps, and hidden class pointers), Rust structs are pure data with zero overhead.

### No Runtime Overhead

Unlike JavaScript-based compilers that run in V8, Vize compiles directly to native code. There's no JIT warmup, no garbage collector, and no event loop contention. The compiler binary is a single, statically-linked executable that starts and runs at full speed.

## Architecture Choices for Performance

### Arena Allocation

`vize_carton` provides a bump allocator for AST nodes using [bumpalo](https://docs.rs/bumpalo). This means:

- **Allocation is O(1)** — Just bump a pointer forward. No free list traversal, no fragmentation management.
- **Deallocation is O(1)** — Drop the entire arena at once when compilation is complete. No per-node deallocation overhead.
- **Memory locality is excellent** — Nodes are packed contiguously in memory, maximizing L1/L2 cache hits during tree traversal.

This is a fundamental advantage over V8's generational garbage collector, which must trace reachable objects and compact memory periodically.

### Streaming Tokenizer

`vize_armature`'s tokenizer processes input as a stream of bytes, avoiding the need to build intermediate token arrays. The parser consumes tokens lazily — each token is produced on demand and immediately consumed. This reduces peak memory usage and improves cache behavior.

### String Interning

Common strings (directive names, attribute names, HTML tag names) are interned via `compact_str` and perfect hash tables (`phf`). This means:

- String comparison is pointer comparison (O(1)) instead of character-by-character comparison (O(n))
- Duplicate strings share a single allocation
- Hash lookups for known strings are compile-time computed

### Incremental Compilation

The Vite plugin (`@vizejs/vite-plugin`) uses file-level caching. Only modified files are recompiled during development, minimizing HMR latency. The cache key is the file content hash, ensuring that unchanged files are never recompiled.

## Benchmark: Linter — patina vs eslint-plugin-vue

Linting **15,000 Vue SFC files**:

| | eslint-plugin-vue (ST) | Vize patina (ST) | Speedup | eslint-plugin-vue (MT) | Vize patina (MT) | Speedup | **eslint ST vs Vize MT** |
|--|------------------------|------------------|---------|------------------------|------------------|---------|--------------------------|
| **Time** | 65.30s | 5.45s | **12.0x** | 26.82s | 5.48s | **4.9x** | **11.9x** |

Run `mise run bench:lint` to reproduce.

## Benchmark: Formatter — glyph vs Prettier

Formatting **15,000 Vue SFC files**:

| | Prettier (ST) | Vize glyph (ST) | Speedup | Prettier (MT) | Vize glyph (MT) | Speedup | **Prettier ST vs Vize MT** |
|--|---------------|-----------------|---------|---------------|-----------------|---------|----------------------------|
| **Time** | 82.69s | 36ms | **2,303x** | 19.66s | 23ms | **872x** | **3,666x** |

Run `mise run bench:fmt` to reproduce.

## Benchmark: Type Checker — canon vs vue-tsc

Type checking **15,000 Vue SFC files**:

| | vue-tsc (ST) | Vize canon (ST) | Speedup | vue-tsc (MT) | Vize canon (MT) | Speedup | **vue-tsc ST vs Vize MT** |
|--|--------------|-----------------|---------|--------------|-----------------|---------|---------------------------|
| **Time** | 35.69s | 369ms | **96.7x** | 26.76s | 472ms | **56.7x** | **75.5x** |

> **Note:** Vize canon is still in early development and does not yet cover the full range of type checking features that vue-tsc provides. The speed difference partly reflects the difference in the amount of work each tool currently performs. These numbers will change as canon's feature set matures.

Run `mise run bench:check` to reproduce.

## Benchmark: Vite Plugin — @vizejs/vite-plugin vs @vitejs/plugin-vue

Vite build with **15,000 Vue SFC imports** (all imported in a single entry):

| | @vitejs/plugin-vue | @vizejs/vite-plugin | Speedup |
|--|-------------------|---------------------|---------|
| **Build Time** | 16.98s | 6.90s | **2.5x** |

> Note: `@vizejs/vite-plugin` replaces only the Vue SFC compilation step — the performance difference comes entirely from that part. Dependency resolution, module graph construction, bundling (Rolldown), and all other Vite internals are identical to `@vitejs/plugin-vue`. For pure compilation performance, see the [Compiler benchmark](#benchmark-15000-sfc-files) above. `@vizejs/vite-plugin` eagerly pre-compiles `.vue` files using native multi-threaded compilation, which also enables faster HMR.

Run `mise run bench:vite` to reproduce.
