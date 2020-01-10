use futures_intrusive::sync::LocalManualResetEvent;
use std::collections::HashMap;

pub struct MPMCManualResetEvent {
    counter: u128,
    handles: HashMap<u128, LocalManualResetEvent>,
}

impl MPMCManualResetEvent {
    pub fn new() -> MPMCManualResetEvent {
        MPMCManualResetEvent {
            handles: HashMap::<u128, LocalManualResetEvent>::new(),
            counter: 0,
        }
    }

    pub fn get_handle(&mut self) -> u128 {
        let handle = LocalManualResetEvent::new(false);
        let handle_id = self.counter;
        self.handles.insert(handle_id, handle);
        self.counter += 1;
        handle_id
    }

    pub async fn block(&self, handle: u128) {
        let handle = self.handles.get(&handle).unwrap();
        handle.wait().await;
        handle.reset();
    }

    pub fn drop_handle(&mut self, handle: u128) {
        self.handles.remove(&handle);
    }

    pub fn broadcast(&self) {
        for (_, handle) in self.handles.iter() {
            handle.set()
        }
    }

    pub fn broadcast_to_others(&self, handle: u128) {
        for (handle_id, current_handle) in self.handles.iter() {
            if *handle_id == handle {
                continue;
            }
            current_handle.set()
        }
    }
}
