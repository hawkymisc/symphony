# Symphony

Symphony turns project work into isolated, autonomous implementation runs, allowing teams to manage
work instead of supervising coding agents.

[![Symphony demo video preview](.github/media/symphony-demo-poster.jpg)](.github/media/symphony-demo.mp4)

_In this [demo video](.github/media/symphony-demo.mp4), Symphony monitors a Linear board for work and spawns agents to handle the tasks. The agents complete the tasks and provide proof of work: CI status, PR review feedback, complexity analysis, and walkthrough videos. When accepted, the agents land the PR safely. Engineers do not need to supervise Codex; they can manage the work at a higher level._

> [!WARNING]
> Symphony is a low-key engineering preview for testing in trusted environments.

---

## Implementations

| Implementation | Tracker | Agent | Status |
|---|---|---|---|
| [Elixir](elixir/) (reference) | Linear | Codex | Upstream original |
| [Rust](rust/) | GitHub Issues | Claude Code CLI | ✅ All phases complete |

---

## Rust Implementation (GitHub + Claude Code)

The Rust implementation connects **GitHub Issues** with the **Claude Code CLI** to automate coding tasks.

### Requirements

- Rust 1.75+
- [`claude` CLI](https://claude.ai/code) installed and authenticated
- GitHub personal access token with `repo` scope

### Quick Start

**1. Build**

```bash
cd rust
cargo build --release
```

**2. Create `WORKFLOW.md`**

```markdown
---
tracker:
  kind: github
  repo: "owner/your-repo"
  api_key: "$GITHUB_TOKEN"          # or literal token
  labels: ["symphony"]              # optional: only pick up issues with this label
agent:
  max_concurrent_agents: 3
polling:
  interval_ms: 30000                # poll every 30 s
---
You are a coding agent working on {{ issue.title }} (#{{ issue.identifier }}).

Repository: {{ repo }}

Issue description:
{{ issue.description }}

Please implement a solution, open a PR, and close the issue when done.
```

**3. Validate config (dry run)**

```bash
symphony ./WORKFLOW.md --dry-run
```

Expected output:
```
Config validated successfully
  Tracker: github (owner/your-repo)
  Model: claude-sonnet-4-20250514
  Max concurrent agents: 3
```

**4. Run**

```bash
export GITHUB_TOKEN=ghp_xxxxxxxxxxxx
symphony ./WORKFLOW.md
```

**With HTTP observability dashboard (optional):**

```bash
cargo build --release --features http-server
symphony ./WORKFLOW.md --port 8080
# Open http://127.0.0.1:8080 in browser
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Normal shutdown (SIGTERM / SIGINT) |
| 1 | Config / startup validation failure |
| 2 | CLI argument error |
| 3 | Workflow file error (missing / unreadable / invalid YAML) |

### WORKFLOW.md Reference

```yaml
---
tracker:
  kind: github               # required
  repo: "owner/repo"         # required (owner/repo format)
  api_key: "$GITHUB_TOKEN"   # required; $VAR resolves from env
  endpoint: "..."            # optional: override GitHub GraphQL URL
  labels: ["symphony"]       # optional: label filter

agent:
  max_concurrent_agents: 5   # default: 5
  max_retry_backoff_ms: 300000  # default: 5 min

polling:
  interval_ms: 30000         # default: 30 s

claude:
  command: "claude"          # default: claude
  model: "claude-sonnet-4-20250514"
  max_turns_per_invocation: 50
  skip_permissions: false    # set true only in trusted environments

workspace:
  root: "~/symphony-workspaces"  # default: $TMPDIR/symphony_workspaces

hooks:
  after_create: "./scripts/setup.sh"    # runs once when workspace is created
  before_run:   "./scripts/prepare.sh"  # runs before each agent invocation
  after_run:    "./scripts/cleanup.sh"  # runs after each agent invocation
---
Prompt template here. Available variables:

{{ issue.title }}        — issue title
{{ issue.identifier }}   — issue number (e.g. "42")
{{ issue.description }}  — issue body
{{ repo }}               — "owner/repo"
{{ attempt }}            — retry attempt number (1-indexed; absent on first run)
```

---

## Feature Status

### ✅ Implemented

| Feature | Notes |
|---|---|
| GitHub Issues polling | GraphQL v4, pagination, label filtering |
| Issue dispatch | Priority sort, concurrency limit, claim deduplication |
| Claude Code CLI integration | Subprocess, streaming JSON events, token tracking |
| Workspace management | Per-issue directories, hook scripts |
| Retry with exponential backoff | Configurable cap, consecutive failure tracking |
| Graceful shutdown | SIGTERM / SIGINT → cancel-safe exit |
| Dry-run mode | `--dry-run` validates config and exits |
| Observability snapshot | `OrchestratorMsg::SnapshotRequest` returns `RuntimeSnapshot` |
| HTTP dashboard | Feature-gated (`--features http-server`); `GET /`, `GET /api/status`, `POST /api/refresh` |
| Structured logging | `tracing` with JSON output; issue_id + identifier in every span |
| Token aggregation | Input/output/total tokens across all sessions |

### 🔲 Not Yet Implemented

| Feature | Notes |
|---|---|
| GitHub Projects v2 | Only GitHub Issues supported; Projects v2 custom fields not yet mapped |
| HTTP dashboard auth | Dashboard binds to loopback only; no bearer token / unix socket option |
| Windows graceful shutdown | SIGTERM test is `#[cfg(unix)]`; Windows `Ctrl+C` path untested |
| Real-GitHub CI gate | Integration tests use `MemoryTracker`; no staging smoke test |
| Config hot-reload | `ConfigReloaded` message exists but doesn't re-parse WORKFLOW.md |
| Per-state concurrency limits | Global limit only; no per-label / per-project slot control |
| Rate limit backoff | GitHub rate limit headers are tracked but don't auto-pause polling |

---

## Running the Original Elixir Implementation

Check out [elixir/README.md](elixir/README.md) for the upstream Linear + Codex reference implementation.

---

## License

This project is licensed under the [Apache License 2.0](LICENSE).
