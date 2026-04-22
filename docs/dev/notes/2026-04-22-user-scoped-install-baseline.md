# User-Scoped Install Baseline

Date: 2026-04-22

## Scope

This note records the user-scoped `agent-browser` install state before service
roadmap development begins.

Active agents may call `agent-browser` from the user's PATH. That command
should remain stable while the repository is under active development.

## Finding

The global pnpm install had previously been linked to this workspace:

```text
agent-browser@link:../../../../../workspace.local/agent-browser
```

That meant agents invoking `agent-browser` could pick up repository edits
immediately.

The registry packages checked during this review did not match the local
working functionality:

- `agent-browser@0.26.0` ran, but did not recognize `runtime`.
- `agent-browser@0.25.3` ran, but did not recognize `runtime`.
- the current source-built release binary reported `agent-browser 0.25.3` and
  did support `runtime list`.

## Baseline Installed

The user-scoped install was replaced with a local release tarball built from
the current source:

```text
/home/ecochran76/.agent-browser/releases/agent-browser-0.25.3-service-baseline.tgz
```

The pnpm global command now resolves through pnpm's global package store, not
through the repository workspace.

Validation at install time:

```text
command -v agent-browser
/home/ecochran76/.local/share/pnpm/agent-browser

agent-browser --version
agent-browser 0.25.3

agent-browser runtime list
Runtime profiles: ...
```

## Development Rule

Do not use `pnpm link -g`, `pnpm add -g .`, or an equivalent workspace link for
the user-scoped `agent-browser` command while other agents depend on it.

For repository development, use explicit dev commands instead:

```bash
cargo run --manifest-path cli/Cargo.toml -- <args>
./cli/target/debug/agent-browser <args>
./cli/target/release/agent-browser <args>
```

If the user-scoped command must be upgraded again, build and install a new
durable tarball under:

```text
/home/ecochran76/.agent-browser/releases/
```

Then verify at minimum:

```bash
agent-browser --version
agent-browser runtime list
pnpm list -g --depth 0
```

## Reason

Keeping the user-scoped command isolated lets active agents continue using a
known-good browser automation build while service-mode development proceeds in
the repository.

