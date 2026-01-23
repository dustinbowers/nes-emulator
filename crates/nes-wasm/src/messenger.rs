use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::MessageEvent;

pub struct Messenger<T: Serialize + DeserializeOwned> {
    queue: Rc<RefCell<VecDeque<T>>>,
}

impl<T: Serialize + DeserializeOwned + 'static> Messenger<T> {
    pub fn new() -> Self {
        Self {
            queue: Rc::new(RefCell::new(VecDeque::new())),
        }
    }

    pub fn init_message_listener(&self) {
        let window = web_sys::window().unwrap();
        let queue = self.queue.clone();

        let closure = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let log = format!("[WASM Received Message Event]: {:?}", event);
            web_sys::console::log_1(&log.into());

            let data = event.data();
            if let Ok(message) = from_value(data) {
                queue.borrow_mut().push_back(message);
            }
        });

        window
            .add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())
            .unwrap();

        closure.forget();
    }

    pub fn send(&self, message: &T)
    where
        T: Serialize + DeserializeOwned,
    {
        let window = web_sys::window().unwrap();
        let parent = window.parent().unwrap().expect("Can't find parent");
        let value = to_value(message).unwrap();
        parent.post_message(&value, "*").unwrap();

        let log = format!("[WASM posted message]: {:?}", value);
        web_sys::console::log_1(&log.into());
    }

    pub fn receive(&mut self) -> Option<T> {
        self.queue.borrow_mut().pop_front()
    }
}
