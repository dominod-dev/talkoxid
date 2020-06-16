use cursive::event::{Event, EventResult, Key};
use cursive::theme::{Color, ColorStyle, ColorType};
use cursive::traits::*;
use cursive::view::ViewWrapper;
use cursive::views::{EditView, TextView};
use cursive::wrap_impl;
use cursive::{CbSink, Cursive};

use super::ChatEvent;
use std::sync::mpsc;

pub struct MessageBoxView {
    view: EditView,
    tx: mpsc::Sender<ChatEvent>,
}

impl MessageBoxView {
    // Creates a new view with the given buffer size
    pub fn new(tx: mpsc::Sender<ChatEvent>) -> Self {
        let white = ColorType::Color(Color::Rgb(255, 255, 255));
        let black = ColorType::Color(Color::Rgb(0, 0, 0));
        let white_on_black = ColorStyle::new(black, white);
        let view = EditView::new().style(white_on_black);
        MessageBoxView { tx, view }
    }
}

impl ViewWrapper for MessageBoxView {
    wrap_impl!(self.view: EditView);
    fn wrap_on_event<'r>(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Enter) => {
                self.tx
                    .send(ChatEvent::SendMessage(String::from(
                        self.view.get_content().as_ref(),
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
