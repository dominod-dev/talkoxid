use async_channel::Sender;

use cursive::event::{Event, EventResult, Key};
use cursive::theme::{Color, ColorStyle, ColorType};
use cursive::traits::*;
use cursive::view::ViewWrapper;
use cursive::views::{EditView, SelectView, TextView};
use cursive::wrap_impl;
use cursive::{CbSink, Cursive};

use std::error::Error;

use super::{Channel, ChatEvent};

pub struct MessageBoxView {
    view: EditView,
    pub channel: Option<Channel>,
    tx: Sender<ChatEvent>,
    rt: tokio::runtime::Handle,
}

impl MessageBoxView {
    // Creates a new view with the given buffer size
    pub fn new(
        channel: Option<Channel>,
        tx: Sender<ChatEvent>,
        rt: tokio::runtime::Handle,
    ) -> Self {
        let white = ColorType::Color(Color::Rgb(255, 255, 255));
        let black = ColorType::Color(Color::Rgb(0, 0, 0));
        let white_on_black = ColorStyle::new(black, white);
        let view = EditView::new().style(white_on_black);
        MessageBoxView {
            channel,
            tx,
            view,
            rt,
        }
    }
}

impl ViewWrapper for MessageBoxView {
    wrap_impl!(self.view: EditView);
    fn wrap_on_event<'r>(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Enter) => {
                self.rt.block_on(async {
                    self.tx
                        .send(ChatEvent::SendMessage(
                            String::from(self.view.get_content().as_ref()),
                            self.channel.clone().unwrap(),
                        ))
                        .await
                        .unwrap()
                });
                self.view.set_content("");
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

    pub fn init(&mut self, content: String) -> Result<(), Box<dyn Error>> {
        self.view.set_content(content);
        self.cb_sink.send(Box::new(Cursive::noop))?;
        Ok(())
    }

    pub fn add_message(&mut self, message: String) -> Result<(), Box<dyn Error>> {
        self.view.append(message);
        self.cb_sink.send(Box::new(Cursive::noop))?;
        Ok(())
    }
}

impl<'a> ViewWrapper for BufferView {
    wrap_impl!(self.view: TextView);

    fn wrap_required_size(&mut self, size: cursive::Vec2) -> cursive::Vec2 {
        let mut required_size = self.view.required_size(size);
        required_size.x = size.x;
        required_size
    }
}

#[derive(Default)]
pub struct ChannelView {
    pub view: SelectView<Channel>,
}

impl ChannelView {
    pub fn new() -> Self {
        let view = SelectView::new();
        ChannelView { view }
    }
    pub fn on_submit(mut self, func: impl Fn(&mut Cursive, &Channel) + 'static) -> Self {
        self.view.set_on_submit(func);
        self
    }
}

impl<'a> ViewWrapper for ChannelView {
    wrap_impl!(self.view: SelectView<Channel>);
}
