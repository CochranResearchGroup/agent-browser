# Code discovery

Prefer configured CodeGraph or codebase-memory MCP tools for structural questions: definitions, calls, flows and impact. Use current indexed bodies; read specifically flagged stale files. If unavailable or insufficient, use native search and reads.

For literal text, error strings, configuration and documentation, use rg or repository-native Git reads. When the canonical checkout is dirty or behind, source qualification must use the reviewed canonical commit, not unrelated working-tree bytes.

Use file-searcher first for unknown locations outside the repository. Use SysRAG for semantic filesystem recall after location discovery and Graphiti for advisory durable decisions, verifying retrieved claims against current source and receipts.
