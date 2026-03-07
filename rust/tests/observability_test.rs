//! Phase 6: Observability tests (PLAN.md §Phase 6)
//!
//! Tests for logging context, token aggregation, and runtime snapshots.

use chrono::{Duration, Utc};

use symphony::domain::{TokenTotals, TokenUsage};
use symphony::observability::{RateLimitInfo, RunningEntrySnapshot, RuntimeSnapshot};

// ─── 1. log_includes_issue_context ───────────────────────────────────────────

/// RunningEntrySnapshot carries issue_id and identifier for structured logging.
#[test]
fn log_includes_issue_context() {
    let entry = RunningEntrySnapshot {
        issue_id: "gid://github/Issue/42".to_string(),
        identifier: "42".to_string(),
        session_id: None,
        turn_count: 0,
        input_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
        last_event: None,
        last_event_message: None,
        started_at: Utc::now(),
        seconds_running: 0.0,
    };

    assert_eq!(entry.issue_id, "gid://github/Issue/42");
    assert_eq!(entry.identifier, "42");
}

// ─── 2. log_includes_session_context ─────────────────────────────────────────

/// RunningEntrySnapshot includes session_id for per-session log correlation.
#[test]
fn log_includes_session_context() {
    let entry = RunningEntrySnapshot {
        issue_id: "gid://github/Issue/42".to_string(),
        identifier: "42".to_string(),
        session_id: Some("gid://github/Issue/42-1".to_string()),
        turn_count: 1,
        input_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
        last_event: None,
        last_event_message: None,
        started_at: Utc::now(),
        seconds_running: 0.0,
    };

    assert!(entry.session_id.is_some());
    assert_eq!(entry.session_id.as_deref(), Some("gid://github/Issue/42-1"));
}

// ─── 3. token_aggregation_across_sessions ────────────────────────────────────

/// TokenTotals correctly accumulates tokens from multiple independent sessions.
#[test]
fn token_aggregation_across_sessions() {
    let mut totals = TokenTotals::new();

    // Session 1 finishes
    totals.add(&TokenUsage {
        input_tokens: 100,
        output_tokens: 50,
        cache_read_tokens: None,
        cache_creation_tokens: None,
    });

    // Session 2 finishes
    totals.add(&TokenUsage {
        input_tokens: 200,
        output_tokens: 100,
        cache_read_tokens: Some(30),
        cache_creation_tokens: None,
    });

    assert_eq!(totals.input_tokens, 300);
    assert_eq!(totals.output_tokens, 150);
    assert_eq!(totals.total_tokens, 450);
    assert_eq!(totals.cache_read_tokens, 30);
}

// ─── 4. token_no_double_count ────────────────────────────────────────────────

/// Using compute_delta() prevents double-counting when the same absolute total
/// is reported more than once (e.g. multiple "result" events from Claude).
#[test]
fn token_no_double_count() {
    let mut last_input = 0u64;
    let mut last_output = 0u64;
    let mut totals = TokenTotals::new();

    // First report: absolute counts (100 in, 50 out)
    {
        let (in_delta, out_delta, _) =
            TokenTotals::compute_delta(100, 50, last_input, last_output);
        totals.add(&TokenUsage {
            input_tokens: in_delta,
            output_tokens: out_delta,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        });
        last_input = 100;
        last_output = 50;
    }

    // Second report: same absolute counts again (should contribute 0 delta)
    {
        let (in_delta, out_delta, _) =
            TokenTotals::compute_delta(100, 50, last_input, last_output);
        totals.add(&TokenUsage {
            input_tokens: in_delta,
            output_tokens: out_delta,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        });
    }

    // Must not double-count
    assert_eq!(totals.input_tokens, 100);
    assert_eq!(totals.output_tokens, 50);
    assert_eq!(totals.total_tokens, 150);
}

// ─── 5. runtime_seconds_includes_active ──────────────────────────────────────

/// Snapshot of a running entry includes the elapsed wall-clock seconds.
#[test]
fn runtime_seconds_includes_active() {
    let started = Utc::now() - Duration::seconds(10);
    let elapsed = (Utc::now() - started).num_milliseconds() as f64 / 1000.0;

    let snapshot = RunningEntrySnapshot {
        issue_id: "gid://github/Issue/1".to_string(),
        identifier: "1".to_string(),
        session_id: None,
        turn_count: 1,
        input_tokens: 100,
        output_tokens: 50,
        total_tokens: 150,
        last_event: Some("result".to_string()),
        last_event_message: None,
        started_at: started,
        seconds_running: elapsed,
    };

    assert!(
        snapshot.seconds_running >= 10.0,
        "Expected at least 10s elapsed, got {}",
        snapshot.seconds_running
    );
}

// ─── 6. rate_limit_tracking ──────────────────────────────────────────────────

/// The RuntimeSnapshot preserves the most recent rate-limit info from the tracker.
#[test]
fn rate_limit_tracking() {
    let rate_limits = RateLimitInfo {
        remaining: 4500,
        limit: 5000,
        reset_at: Utc::now() + Duration::hours(1),
        source: "github".to_string(),
    };

    let snapshot = RuntimeSnapshot {
        generated_at: Utc::now(),
        running_count: 0,
        retrying_count: 0,
        completed_count: 0,
        running: vec![],
        retrying: vec![],
        agent_totals: TokenTotals::new(),
        rate_limits: Some(rate_limits),
    };

    let rl = snapshot.rate_limits.as_ref().expect("rate_limits should be Some");
    assert_eq!(rl.remaining, 4500);
    assert_eq!(rl.limit, 5000);
    assert_eq!(rl.source, "github");
}
