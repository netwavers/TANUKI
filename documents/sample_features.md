# 🐾 T.A.N.U.K.I. Engine Features

This sample document describes the core features of the TANUKI engine for testing the AST compiler.

## 🧠 Memory-Mapped Search (mmap)

The engine compiles the abstract syntax tree (AST) into a binary file format (`knowledge.bin`) using FlatBuffers.
- O(1) subtree skipping is enabled for ultra-fast queries.
- Utilizes `mmap_memory.rs` to allow zero-copy mapping of the database into the processes.

## 🛡️ VRAM Defense Protocol

To prevent CUDA memory fragmentation and out-of-memory errors on consumer GPUs, the engine enforces:
- Lock-based sequential processing using `asyncio.Lock` or Rust semaphores.
- Explicit batch model unloading at the end of compilation sessions.
