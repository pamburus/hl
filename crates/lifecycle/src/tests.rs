use super::*;
use std::sync::{Arc, Mutex};

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
    let started = Arc::new(Mutex::new(false));
    let started_clone = Arc::clone(&started);
    let completed = Arc::new(Mutex::new(false));
    let completed_clone = Arc::clone(&completed);

    struct SlowDrop {
        started: Arc<Mutex<bool>>,
        completed: Arc<Mutex<bool>>,
    }

    impl Drop for SlowDrop {
        fn drop(&mut self) {
            *self.started.lock().unwrap() = true;
            std::thread::sleep(std::time::Duration::from_millis(50));
            *self.completed.lock().unwrap() = true;
        }
    }

    {
        let slow_drop = SlowDrop {
            started: started_clone,
            completed: completed_clone,
        };
        let _async_drop = AsyncDrop::new(slow_drop);
    }

    assert!(*started.lock().unwrap());
    assert!(*completed.lock().unwrap());
}
