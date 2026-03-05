//! Retry queue and backoff computation

/// Exit type for retry backoff calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitType {
    /// Normal exit (success or expected end) - use short delay
    Normal,
    /// Failure exit - use exponential backoff
    Failure,
}

/// Compute backoff delay in milliseconds
///
/// # Arguments
/// * `exit_type` - Whether the exit was normal or a failure
/// * `attempt` - The retry attempt number (1-indexed)
/// * `max_backoff_ms` - Maximum backoff in milliseconds (only applies to failures)
///
/// # Returns
/// Delay in milliseconds
///
/// # Formula
/// - Normal exit: 1,000 ms (fixed)
/// - Failure exit: min(10,000 * 2^(attempt-1), max_backoff_ms)
pub fn compute_backoff(exit_type: ExitType, attempt: u32, max_backoff_ms: u64) -> u64 {
    match exit_type {
        ExitType::Normal => 1_000, // 1 second for normal continuation
        ExitType::Failure => compute_failure_backoff(attempt, max_backoff_ms),
    }
}

/// Compute failure backoff
///
/// Formula: min(10,000 * 2^(attempt-1), max_backoff_ms)
///
/// # Arguments
/// * `attempt` - The retry attempt number (1-indexed)
/// * `max_backoff_ms` - Maximum backoff in milliseconds
///
/// # Returns
/// Delay in milliseconds
pub fn compute_failure_backoff(attempt: u32, max_backoff_ms: u64) -> u64 {
    let base_ms: u64 = 10_000; // 10 seconds
    let exponential = base_ms * (2u64.pow(attempt.saturating_sub(1)));
    exponential.min(max_backoff_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_normal_backoff() {
        // Normal exit always gets 1 second
        assert_eq!(compute_backoff(ExitType::Normal, 1, 300_000), 1_000);
        assert_eq!(compute_backoff(ExitType::Normal, 5, 300_000), 1_000);
    }

    #[test]
    fn compute_failure_backoff_basic() {
        // Attempt 1: 10s
        assert_eq!(compute_backoff(ExitType::Failure, 1, 300_000), 10_000);
        // Attempt 2: 20s
        assert_eq!(compute_backoff(ExitType::Failure, 2, 300_000), 20_000);
        // Attempt 3: 40s
        assert_eq!(compute_backoff(ExitType::Failure, 3, 300_000), 40_000);
        // Attempt 4: 80s
        assert_eq!(compute_backoff(ExitType::Failure, 4, 300_000), 80_000);
    }

    #[test]
    fn compute_failure_backoff_cap() {
        // With 60 second cap
        assert_eq!(compute_backoff(ExitType::Failure, 1, 60_000), 10_000);
        assert_eq!(compute_backoff(ExitType::Failure, 2, 60_000), 20_000);
        assert_eq!(compute_backoff(ExitType::Failure, 3, 60_000), 40_000);
        // Attempt 4 would be 80s, capped at 60s
        assert_eq!(compute_backoff(ExitType::Failure, 4, 60_000), 60_000);
        assert_eq!(compute_backoff(ExitType::Failure, 10, 60_000), 60_000);
    }

    #[test]
    fn compute_failure_backoff_5_minute_cap() {
        // Default cap: 5 minutes (300 seconds = 300,000 ms)
        // 2^5 = 32, so attempt 6: 10 * 32 = 320s > 300s
        assert_eq!(compute_backoff(ExitType::Failure, 5, 300_000), 160_000);
        assert_eq!(compute_backoff(ExitType::Failure, 6, 300_000), 300_000);
    }

    #[test]
    fn direct_failure_backoff_function() {
        // Test the standalone function
        assert_eq!(compute_failure_backoff(1, 300_000), 10_000);
        assert_eq!(compute_failure_backoff(2, 300_000), 20_000);
    }
}
