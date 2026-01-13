//! Concurrent batch processing utilities.

use futures::stream::{self, StreamExt};
use std::future::Future;

/// Processes items in batches with controlled concurrency.
///
/// This function takes a collection of items and processes them concurrently,
/// with a configurable limit on how many operations can run simultaneously.
///
/// # Arguments
///
/// * `items` - Vector of items to process
/// * `processor` - Async function that processes each item, taking the item and its index
/// * `concurrency` - Maximum number of concurrent operations
///
/// # Returns
///
/// A vector of successfully processed results. Items that return `None` are filtered out.
///
/// # Examples
///
/// ```no_run
/// # use rule_llms_txt_gen::batch::process_in_batches;
/// # async fn example() {
/// let urls = vec!["url1", "url2", "url3"];
/// let results = process_in_batches(
///     urls,
///     |url, index| Box::pin(async move {
///         // Process URL here
///         Some(format!("Processed: {}", url))
///     }),
///     5
/// ).await;
/// # }
/// ```
pub async fn process_in_batches<T, F, Fut, R>(
    items: Vec<T>,
    processor: F,
    concurrency: usize,
) -> Vec<R>
where
    T: Send + 'static,
    F: Fn(T, usize) -> Fut,
    Fut: Future<Output = Option<R>> + Send + 'static,
    R: Send + 'static,
{
    stream::iter(items.into_iter().enumerate())
        .map(|(index, item)| processor(item, index))
        .buffer_unordered(concurrency)
        .filter_map(|result| async move { result })
        .collect()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_process_in_batches() {
        let items = vec![1, 2, 3, 4, 5];
        let results = process_in_batches(
            items,
            |item, _index| Box::pin(async move { Some(item * 2) }),
            2,
        )
        .await;

        assert_eq!(results.len(), 5);
        // Results might be out of order due to concurrent processing
        let mut sorted = results;
        sorted.sort();
        assert_eq!(sorted, vec![2, 4, 6, 8, 10]);
    }

    #[tokio::test]
    async fn test_process_in_batches_with_none() {
        let items = vec![1, 2, 3, 4, 5];
        let results = process_in_batches(
            items,
            |item, _index| {
                Box::pin(async move {
                    if item % 2 == 0 {
                        Some(item)
                    } else {
                        None
                    }
                })
            },
            2,
        )
        .await;

        assert_eq!(results.len(), 2);
        let mut sorted = results;
        sorted.sort();
        assert_eq!(sorted, vec![2, 4]);
    }

    #[tokio::test]
    async fn test_process_in_batches_concurrency() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let concurrent_count = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));

        let items = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let concurrent_count_clone = concurrent_count.clone();
        let max_concurrent_clone = max_concurrent.clone();

        let results = process_in_batches(
            items,
            move |item, _index| {
                let concurrent_count = concurrent_count_clone.clone();
                let max_concurrent = max_concurrent_clone.clone();

                Box::pin(async move {
                    // Increment concurrent count
                    let current = concurrent_count.fetch_add(1, Ordering::SeqCst) + 1;

                    // Update max concurrent count
                    max_concurrent.fetch_max(current, Ordering::SeqCst);

                    // Simulate some work
                    sleep(Duration::from_millis(10)).await;

                    // Decrement concurrent count
                    concurrent_count.fetch_sub(1, Ordering::SeqCst);

                    Some(item)
                })
            },
            3, // Max concurrency of 3
        )
        .await;

        assert_eq!(results.len(), 8);
        // Max concurrent should not exceed the concurrency limit
        let max = max_concurrent.load(Ordering::SeqCst);
        assert!(max <= 3, "Max concurrent was {}, expected <= 3", max);
    }
}
