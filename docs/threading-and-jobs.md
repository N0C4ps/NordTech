# Threading, jobs, and task scheduling

`nord_threading` owns an opaque worker pool. `nord_jobs` owns an independent
data-parallel pool. `nord_scheduler` owns a separate pool for its cancellable,
priority-scheduled tasks, and `nord_render` owns the workers it uses for graph
recording. None of these crates shares or discovers another crate's pool.
Consumers choose which capabilities to construct and how many workers each
should use.

## Worker pools

Rayon gives `nord_jobs` and `nord_scheduler` work-stealing execution. Each
crate creates and owns its pool rather than configuring the process-global
pool. The renderer has its own recording pool, independent of the job API.

`JobSystemConfig::default` uses the available-parallelism estimate minus one
logical processor, with a minimum of one worker. Pass an explicit
`JobSystemConfig` when creating a `JobSystem` to choose another count.

## Job API

`nordtech::jobs::JobSystem` offers:

- parallel read-only `for_each` over a slice;
- parallel disjoint `for_each_mut`;
- parallel `map`, whose collected output follows input order;
- owned-item `map_owned`, whose results follow input order;
- indexed `for_each_chunk_mut` for independent chunk-sized data;
- `join` for two independent computations.

`nord_render` implements its own recording pool and does not depend on
`nord_jobs`. Small callback sets stay on the calling thread to avoid dispatch
overhead.

Work execution order and worker identity are unspecified. Jobs should be
CPU-bound, reasonably sized, and independent where possible. Rust's `Send` and
`Sync` bounds prevent data races at the API boundary; mutable slices are divided
into disjoint regions by the scheduler. Avoid blocking waits, long synchronous
I/O, and locks with long critical sections inside worker operations, since they
reduce throughput and can stall all work sharing the pool.

## Nord task scheduler

Rayon does not expose priority scheduling or task cancellation. `nord_scheduler`
therefore owns a bounded, multi-priority waiting queue and dispatches up to a
configured in-flight limit into its own Rayon pool. A dedicated dispatcher
keeps queue policy outside worker execution. Queue admission is non-blocking:
when capacity is exhausted, `submit` returns `QueueFull` so the caller can
retry, lower priority, or discard stale work without stalling a frame.

Priorities are `Critical`, `High`, `Normal`, `Low`, and `Background`. They affect
queued work only; running CPU tasks are never preempted. FIFO order is preserved
within each priority. Priority aging promotes waiting work at a configurable
interval, preventing a constant stream of foreground jobs from starving old
background tasks. This is a scheduling preference, not a hard real-time
deadline guarantee.

Each accepted task gets a stable scheduler-local ID and a completion handle.
Dropping the handle only drops result observation. `cancel()` prevents a queued
task from starting or signals a running task through its `TaskContext`; running
work must call `checkpoint()` or inspect its cancellation token at safe points.
Synchronous CPU work cannot be safely force-killed in Rust, so cancellation is
cooperative. The handle reports cancellation, panic (when panic unwinding is
enabled), or a result; production's `panic = "abort"` policy still treats a
panic as process-fatal and lets crash diagnostics handle it.

Shutdown stops submissions and can either drain queued work or cancel pending
work, then joins the dispatcher and waits for running tasks. It must be called
from outside the scheduler's worker pool. Implicit drop cancels pending work and
detaches dispatcher cleanup to avoid blocking a worker destructor; the
consuming program can call `CancelPending` explicitly during shutdown.

An independently cancellable operation uses the application facade and polls
at safe boundaries:

```rust
let handle = scheduler.submit(nordtech::scheduler::TaskPriority::Background, move |context| {
    let mut result = Vec::new();
    for input in inputs {
        context.checkpoint()?;
        result.push(build_part(input));
    }
    Ok::<_, nordtech::scheduler::TaskExecutionError>(result)
})?;

// Later, poll without blocking the game thread. The outer Result represents
// scheduler cancellation/shutdown; the inner Result is the task's own outcome.
if let Some(task_result) = handle.try_take()? {
    let built_parts = task_result?;
}
```

Task closures should be CPU-bound and coarse enough to amortize queue/handle
overhead. Use `nord_jobs` for fine-grained parallel work. Construct the
scheduler only when the consuming program has a background workload that needs
independent cancellation or priority, such as cancellable asset preparation.

## Sources

- [Rayon crate documentation](https://docs.rs/rayon/latest/rayon/)
- [Rayon thread pool builder](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html)
- [Rayon scoped tasks and panic behavior](https://docs.rs/rayon/latest/rayon/fn.scope.html)
- [Rust `available_parallelism`](https://doc.rust-lang.org/std/thread/fn.available_parallelism.html)
- [Tokio task cancellation semantics](https://docs.rs/tokio/latest/tokio/task/)
- [Crossbeam bounded queues and non-blocking submission](https://docs.rs/crossbeam-channel/latest/crossbeam_channel/)
