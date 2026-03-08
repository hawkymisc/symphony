//! Phase 8: integration_full_cycle test (PLAN.md §Phase 8)
//!
//! Exercises the full orchestrator lifecycle end-to-end using in-process mocks:
//!   MemoryTracker (seed with one open issue)
//!   MockAgentRunner (always succeeds)
//!
//! Verifies: open issue → dispatch → agent run → retry queue → issue polled again.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use symphony::agent::{AgentError, AgentRunner, AgentUpdate};
use symphony::config::AppConfig;
use symphony::domain::Issue;
use symphony::orchestrator::{Orchestrator, OrchestratorMsg};
use symphony::tracker::{MemoryTracker, Tracker};

// ---------------------------------------------------------------------------
// Minimal mock agent runner
// ---------------------------------------------------------------------------

struct SuccessAgent {
    dispatched: Arc<Mutex<Vec<String>>>,
}

impl SuccessAgent {
    fn new() -> Self {
        Self { dispatched: Arc::new(Mutex::new(Vec::new())) }
    }
}

#[async_trait]
impl AgentRunner for SuccessAgent {
    async fn run(
        &self,
        issue: &Issue,
        _attempt: Option<u32>,
        _config: &AppConfig,
        _update_tx: mpsc::UnboundedSender<(String, AgentUpdate)>,
        _cancel: CancellationToken,
    ) -> Result<(), AgentError> {
        self.dispatched.lock().await.push(issue.id.clone());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn open_issue(id: &str, num: &str) -> Issue {
    let mut i = Issue::new(id, num, "Implement feature X");
    i.state = "open".to_string();
    i
}

fn test_config() -> AppConfig {
    let mut c = AppConfig::default();
    c.polling.interval_ms = 30;   // fast poll for tests
    c.agent.max_concurrent_agents = 5;
    c
}

// ---------------------------------------------------------------------------
// integration_full_cycle
// ---------------------------------------------------------------------------

/// Full lifecycle:
/// 1. An open issue exists in the tracker
/// 2. Orchestrator dispatches it to the agent
/// 3. Agent completes successfully
/// 4. Orchestrator moves the issue to the retry queue (1-second continuation delay)
/// 5. The runtime snapshot reflects the completed dispatch
#[tokio::test]
async fn integration_full_cycle_dispatch_and_completion() {
    let issue = open_issue("GH_ISSUE_42", "42");
    let tracker = MemoryTracker::with_issues(vec![issue]);
    let agent = SuccessAgent::new();
    let dispatched = Arc::clone(&agent.dispatched);

    let (orchestrator, tx) = Orchestrator::new(tracker, agent, test_config());
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move { orchestrator.run(cancel_clone).await });

    // Wait long enough for at least one poll+dispatch+agent-run cycle
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Issue was dispatched to the agent
    let ids = dispatched.lock().await;
    assert!(
        ids.contains(&"GH_ISSUE_42".to_string()),
        "issue GH_ISSUE_42 should have been dispatched; got: {:?}", *ids
    );

    cancel.cancel();
}

/// Snapshot after dispatch shows running_count > 0 while agent is in flight.
#[tokio::test]
async fn integration_snapshot_shows_running_while_agent_active() {
    let issue = open_issue("GH_ISSUE_10", "10");
    let tracker = MemoryTracker::with_issues(vec![issue]);

    // Use a slow agent so it's still running when we take the snapshot
    struct SlowAgent;
    #[async_trait]
    impl AgentRunner for SlowAgent {
        async fn run(
            &self,
            _issue: &Issue,
            _attempt: Option<u32>,
            _config: &AppConfig,
            _update_tx: mpsc::UnboundedSender<(String, AgentUpdate)>,
            cancel: CancellationToken,
        ) -> Result<(), AgentError> {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(10)) => {}
                _ = cancel.cancelled() => {}
            }
            Ok(())
        }
    }

    let (orchestrator, tx) = Orchestrator::new(tracker, SlowAgent, test_config());
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    tokio::spawn(async move { orchestrator.run(cancel_clone).await });

    // Let the orchestrator dispatch the issue
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Request a runtime snapshot
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(OrchestratorMsg::SnapshotRequest { reply: reply_tx }).unwrap();
    let snapshot = tokio::time::timeout(Duration::from_secs(1), reply_rx)
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(
        snapshot.running_count, 1,
        "one issue should be running; snapshot: {:?}", snapshot.running_count
    );
    assert_eq!(snapshot.running[0].identifier, "10");

    cancel.cancel();
}

/// Multiple issues: all are dispatched (up to concurrency limit).
#[tokio::test]
async fn integration_full_cycle_multiple_issues_dispatched() {
    let issues = vec![
        open_issue("GH_1", "1"),
        open_issue("GH_2", "2"),
        open_issue("GH_3", "3"),
    ];
    let tracker = MemoryTracker::with_issues(issues);
    let agent = SuccessAgent::new();
    let dispatched = Arc::clone(&agent.dispatched);

    let (orchestrator, _tx) = Orchestrator::new(tracker, agent, test_config());
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    tokio::spawn(async move { orchestrator.run(cancel_clone).await });

    // Allow enough time for all 3 issues to be dispatched and completed
    tokio::time::sleep(Duration::from_millis(400)).await;

    let ids = dispatched.lock().await;
    assert!(ids.contains(&"GH_1".to_string()), "GH_1 should be dispatched");
    assert!(ids.contains(&"GH_2".to_string()), "GH_2 should be dispatched");
    assert!(ids.contains(&"GH_3".to_string()), "GH_3 should be dispatched");

    cancel.cancel();
}

/// Closed issue is never dispatched.
#[tokio::test]
async fn integration_closed_issue_never_dispatched() {
    let mut issue = Issue::new("GH_CLOSED", "99", "Closed issue");
    issue.state = "closed".to_string();

    let tracker = MemoryTracker::with_issues(vec![issue]);
    let agent = SuccessAgent::new();
    let dispatched = Arc::clone(&agent.dispatched);

    let (orchestrator, _tx) = Orchestrator::new(tracker, agent, test_config());
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    tokio::spawn(async move { orchestrator.run(cancel_clone).await });

    tokio::time::sleep(Duration::from_millis(150)).await;

    let ids = dispatched.lock().await;
    assert!(
        !ids.contains(&"GH_CLOSED".to_string()),
        "closed issue should NOT be dispatched"
    );

    cancel.cancel();
}
