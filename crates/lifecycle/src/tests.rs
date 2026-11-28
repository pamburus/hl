use super::*;
use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

/// How long a test waits for something that should happen at once, so a bug makes it fail instead of hang.
const HANG_GUARD: Duration = Duration::from_secs(10);

/// A value whose drop reports that it has started, then waits for the gate to open before it completes.
///
/// If the gate is not opened in time, the drop returns without completing.
struct GatedDrop {
    started: mpsc::Sender<()>,
    gate: mpsc::Receiver<()>,
    completed: Arc<Mutex<bool>>,
}

impl Drop for GatedDrop {
    fn drop(&mut self) {
        self.started.send(()).ok();
        if self.gate.recv_timeout(HANG_GUARD).is_ok() {
            *self.completed.lock().unwrap() = true;
        }
    }
}

#[test]
fn drop_notifier_calls_notify_on_drop() {
    let called = Arc::new(Mutex::new(false));
    let called_clone = Arc::clone(&called);

    {
        let _notifier = DropNotifier::new(42, move || {
            *called_clone.lock().unwrap() = true;
        });
    }

    assert!(*called.lock().unwrap());
}

#[test]
fn drop_notifier_drops_inner_before_notify() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let order_clone = Arc::clone(&order);

    struct TestDrop {
        order: Arc<Mutex<Vec<u8>>>,
    }

    impl Drop for TestDrop {
        fn drop(&mut self) {
            self.order.lock().unwrap().push(1);
        }
    }

    {
        let test_drop = TestDrop {
            order: Arc::clone(&order),
        };
        let _notifier = DropNotifier::new(test_drop, move || {
            order_clone.lock().unwrap().push(2);
        });
    }

    let sequence = order.lock().unwrap();
    assert_eq!(*sequence, vec![1, 2]);
}

#[test]
fn async_drop_drops_value_in_background() {
    let dropped = Arc::new(Mutex::new(false));
    let dropped_clone = Arc::clone(&dropped);

    struct TestDrop {
        dropped: Arc<Mutex<bool>>,
    }

    impl Drop for TestDrop {
        fn drop(&mut self) {
            *self.dropped.lock().unwrap() = true;
        }
    }

    {
        let test_drop = TestDrop { dropped: dropped_clone };
        let _async_drop = AsyncDrop::new(test_drop);
    }

    assert!(*dropped.lock().unwrap());
}

#[test]
fn async_drop_waits_for_completion() {
    let (started_tx, started_rx) = mpsc::channel();
    let (gate_tx, gate_rx) = mpsc::channel();
    let completed = Arc::new(Mutex::new(false));

    {
        let _async_drop = AsyncDrop::new(GatedDrop {
            started: started_tx,
            gate: gate_rx,
            completed: Arc::clone(&completed),
        });
        started_rx.recv_timeout(HANG_GUARD).expect("the value was not dropped");
        // Not completed yet, so dropping `_async_drop` must wait for the rest of the drop.
        assert!(!*completed.lock().unwrap(), "the drop did not wait for the gate");
        gate_tx.send(()).expect("the drop stopped waiting for the gate");
    }

    assert!(*completed.lock().unwrap());
}
