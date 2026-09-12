// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Compile and call the public macro as a consumer would. These are behavioral
//! controls: token snapshots alone cannot tell whether generated Rust is valid.

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use wrap_map_err::wrap_map_err;

#[derive(Debug, PartialEq)]
struct SourceError(&'static str);

#[derive(Debug, PartialEq)]
struct TaskError(&'static str);

impl From<SourceError> for TaskError {
    fn from(error: SourceError) -> Self {
        Self(error.0)
    }
}

// Windmill imports a Result alias with a default error type and often writes
// Result<()> in task signatures. Preserve that success type for any T.
type Result<T, E = SourceError> = std::result::Result<T, E>;

#[wrap_map_err(TaskError)]
fn alias_result(fail: bool) -> Result<u32> {
    if fail {
        Err(SourceError("alias"))
    } else {
        Ok(17)
    }
}

#[wrap_map_err(TaskError)]
fn early_return(fail: bool) -> Result<u32, SourceError> {
    if fail {
        return Err(SourceError("early"));
    }
    Ok(23)
}

#[wrap_map_err(TaskError)]
fn qualified_result(fail: bool) -> core::result::Result<u32, SourceError> {
    let first: Result<u32> = if fail {
        Err(SourceError("question mark"))
    } else {
        Ok(7)
    };
    Ok(first? + 1)
}

#[wrap_map_err(TaskError)]
fn borrowed_generic<'a, T: PartialEq>(values: &'a [T], wanted: &T) -> Result<Option<&'a T>> {
    Ok(values.iter().find(|value| *value == wanted))
}

#[wrap_map_err(TaskError)]
async fn async_result(fail: bool) -> Result<u32> {
    let number = async { Ok::<_, SourceError>(31) }.await?;
    if fail {
        return Err(SourceError("async early"));
    }
    Ok(number)
}

#[wrap_map_err(TaskError)]
fn boolean() -> bool {
    true
}

#[wrap_map_err(TaskError)]
fn optional() -> Option<u32> {
    Some(41)
}

#[wrap_map_err(TaskError)]
fn pair() -> (u32, u32) {
    (3, 5)
}

#[wrap_map_err(TaskError)]
fn unit(value: &mut u32) {
    *value += 1;
}

#[test]
fn a_default_error_alias_retains_its_success_value_and_converts_errors() {
    assert_eq!(alias_result(false), Ok(17));
    assert_eq!(alias_result(true), Err(TaskError("alias")));
}

#[test]
fn early_returns_and_question_marks_use_the_original_error_context() {
    assert_eq!(early_return(false), Ok(23));
    assert_eq!(early_return(true), Err(TaskError("early")));
    assert_eq!(qualified_result(false), Ok(8));
    assert_eq!(qualified_result(true), Err(TaskError("question mark")));
}

#[test]
fn nested_generics_and_borrowed_success_values_keep_their_types() {
    let values = ["first", "second"];
    assert_eq!(borrowed_generic(&values, &"second"), Ok(Some(&values[1])));
    assert_eq!(borrowed_generic(&values, &"absent"), Ok(None));
}

/// These futures contain only ready awaits. A single poll is intentional:
/// unexpected Pending fails instead of hanging or introducing a runtime.
fn ready<F: Future>(future: F) -> F::Output {
    struct NoWake;
    impl Wake for NoWake {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(NoWake));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("test future unexpectedly suspended"),
    }
}

#[test]
fn async_functions_preserve_await_and_early_return_semantics() {
    assert_eq!(ready(async_result(false)), Ok(31));
    assert_eq!(ready(async_result(true)), Err(TaskError("async early")));
}

#[test]
fn non_result_functions_keep_their_original_contracts() {
    assert!(boolean());
    assert_eq!(optional(), Some(41));
    assert_eq!(pair(), (3, 5));
    let mut value = 2;
    unit(&mut value);
    assert_eq!(value, 3);
}

#[derive(Debug)]
struct LeafError;

impl From<LeafError> for SourceError {
    fn from(_: LeafError) -> Self {
        Self("leaf")
    }
}

#[wrap_map_err(TaskError)]
fn two_stage_conversion() -> Result<()> {
    // There is intentionally no From<LeafError> for TaskError. '?' must first
    // use the original signature's SourceError, then the outer macro converts.
    Err::<(), _>(LeafError)?;
    Ok(())
}

#[wrap_map_err(TaskError)]
async fn async_two_stage_conversion() -> Result<()> {
    async { Err::<(), _>(LeafError) }.await?;
    Ok(())
}

#[test]
fn question_marks_convert_through_the_original_error_type() {
    assert_eq!(two_stage_conversion(), Err(TaskError("leaf")));
    assert_eq!(ready(async_two_stage_conversion()), Err(TaskError("leaf")));
}

#[wrap_map_err(TaskError)]
async fn suspend_once(polls: Arc<std::sync::atomic::AtomicUsize>) -> Result<u32> {
    std::future::poll_fn(|context| {
        if polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            context.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(Ok(43))
        }
    })
    .await
}

#[test]
fn async_wrapping_keeps_the_future_lazy_sendable_and_resumable() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct NoWake;
    impl Wake for NoWake {
        fn wake(self: Arc<Self>) {}
    }
    fn require_send(_: &impl Send) {}

    let polls = Arc::new(AtomicUsize::new(0));
    let mut future = Box::pin(suspend_once(polls.clone()));
    require_send(&future);
    assert_eq!(
        polls.load(Ordering::SeqCst),
        0,
        "construction cannot execute the body"
    );
    let waker = Waker::from(Arc::new(NoWake));
    let mut context = Context::from_waker(&waker);
    assert!(future.as_mut().poll(&mut context).is_pending());
    assert_eq!(future.as_mut().poll(&mut context), Poll::Ready(Ok(43)));
    assert_eq!(polls.load(Ordering::SeqCst), 2);
}

// Existing Celery consumers rename their Result import to keep it distinct
// from anyhow::Result. The annotation must convert those aliases too.
type TaskResult<T> = Result<T>;
type WrapResult<T> = Result<T>;

#[wrap_map_err(TaskError)]
async fn task_alias(fail: bool) -> TaskResult<u32> {
    if fail {
        return Err(SourceError("task alias"));
    }
    Ok(47)
}

#[wrap_map_err(TaskError)]
fn wrapper_alias() -> WrapResult<()> {
    Err(SourceError("wrapper alias"))
}

#[test]
fn renamed_result_aliases_used_by_celery_tasks_still_convert_errors() {
    assert_eq!(ready(task_alias(false)), Ok(47));
    assert_eq!(ready(task_alias(true)), Err(TaskError("task alias")));
    assert_eq!(wrapper_alias(), Err(TaskError("wrapper alias")));
}

#[wrap_map_err(TaskError)]
async fn owned_non_sync_argument(cell: std::cell::Cell<u32>) -> Result<u32> {
    let value = cell.get();
    async {}.await;
    Ok(value)
}

#[test]
fn an_owned_send_argument_does_not_gain_a_sync_requirement() {
    fn require_send(_: &impl Send) {}
    let future = owned_non_sync_argument(std::cell::Cell::new(53));
    require_send(&future);
    assert_eq!(ready(future), Ok(53));
}

#[wrap_map_err(TaskError)]
fn opaque_success(fail: bool) -> Result<impl Iterator<Item = u8>> {
    if fail {
        return Err(SourceError("opaque"));
    }
    Ok([2, 3, 5].into_iter())
}

#[wrap_map_err(TaskError)]
async fn nested_opaque_success() -> Result<Option<impl Iterator<Item = u8>>> {
    Ok(Some([7, 11].into_iter()))
}

#[test]
fn opaque_success_types_stay_in_the_public_return_position() {
    assert_eq!(
        opaque_success(false).unwrap().collect::<Vec<_>>(),
        [2, 3, 5]
    );
    assert!(matches!(opaque_success(true), Err(TaskError("opaque"))));
    assert_eq!(
        ready(nested_opaque_success())
            .unwrap()
            .unwrap()
            .collect::<Vec<_>>(),
        [7, 11]
    );
}

/// # Safety
/// `pointer` must reference a live initialized u32 for the duration of the call.
#[wrap_map_err(TaskError)]
unsafe fn unsafe_result(pointer: *const u32) -> Result<u32> {
    Ok(*pointer)
}

/// # Safety
/// `pointer` must stay live and initialized until the returned future completes.
#[wrap_map_err(TaskError)]
async unsafe fn async_unsafe_result(pointer: *const u32) -> Result<u32> {
    async {}.await;
    Ok(*pointer)
}

#[test]
fn wrapping_preserves_the_lexical_unsafe_context_of_the_original_function() {
    let value = 59;
    // The local value stays alive through both completed calls; no dangling
    // pointer or concurrent mutation is involved in this compile-time contract.
    unsafe {
        assert_eq!(unsafe_result(&value), Ok(59));
        assert_eq!(ready(async_unsafe_result(&value)), Ok(59));
    }
}
