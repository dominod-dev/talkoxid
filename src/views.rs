use cursive::event::{Event, EventResult, Key};
use cursive::traits::*;
use cursive::view::ViewWrapper;
use cursive::views::{TextArea, TextView};
use cursive::wrap_impl;
use cursive::{CbSink, Cursive};

use super::ChatEvent;
use std::sync::mpsc;

pub struct MessageBoxView {
    view: TextArea,
    tx: mpsc::Sender<ChatEvent>,
}

impl MessageBoxView {
    // Creates a new view with the given buffer size
    pub fn new(tx: mpsc::Sender<ChatEvent>) -> Self {
        let view = TextArea::new();
        MessageBoxView { tx, view }
    }
}

impl ViewWrapper for MessageBoxView {
    wrap_impl!(self.view: TextArea);
    fn wrap_on_event<'r>(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Enter) => {
                self.tx
                    .send(ChatEvent::SendMessage(String::from(
                        self.view.get_content(),
                    )))
                    .unwrap();
                self.view.set_content("");
                // EventResult::Consumed(Some(Callback::from_fn(update_chat)))
                EventResult::Consumed(None)
            }
            ev => self.view.on_event(ev),
        }
    }
}

pub struct BufferView {
    view: TextView,
    cb_sink: CbSink,
}

impl BufferView {
    pub fn new(cb_sink: CbSink) -> Self {
        let view = TextView::new("");
        BufferView { view, cb_sink }
    }

    pub fn init(&mut self, content: String) {
        self.view.set_content(content);
        self.cb_sink.send(Box::new(Cursive::noop)).unwrap();
    }

    pub fn add_message(&mut self, message: String) {
        self.view.append(message);
        self.cb_sink.send(Box::new(Cursive::noop)).unwrap();
    }
}

impl<'a> ViewWrapper for BufferView {
    wrap_impl!(self.view: TextView);
}
