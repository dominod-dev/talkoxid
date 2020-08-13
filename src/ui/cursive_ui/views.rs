use async_channel::Sender;

use cursive::event::{Event, EventResult, Key};
use cursive::theme::{Color, PaletteColor, Theme};
use cursive::traits::*;
use cursive::view::{ScrollStrategy, ViewWrapper};
use cursive::views::{NamedView, ScrollView, SelectView, TextArea, TextView};
use cursive::wrap_impl;
use cursive::{CbSink, Cursive, Printer};

use std::error::Error;

use super::super::super::core::{Channel, ChatEvent};

pub struct MessageBoxView {
    view: TextArea,
    pub channel: Option<Channel>,
    multiline: bool,
    tx: Sender<ChatEvent>,
}

impl MessageBoxView {
    pub fn new(channel: Option<Channel>, tx: Sender<ChatEvent>) -> Self {
        let view = TextArea::new();
        MessageBoxView {
            channel,
            tx,
            view,
            multiline: false,
        }
    }
}

impl ViewWrapper for MessageBoxView {
    wrap_impl!(self.view: TextArea);
    fn wrap_on_event<'r>(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Enter) if !self.multiline => {
                self.tx
                    .try_send(ChatEvent::SendMessage(
                        String::from(self.view.get_content()),
                        self.channel.clone().unwrap(),
                    ))
                    .unwrap();
                self.view.set_content("");
                EventResult::Consumed(None)
            }
            Event::CtrlChar('l') => {
                self.multiline = !self.multiline;
                EventResult::Consumed(None)
            }
            ev => self.view.on_event(ev),
        }
    }
    fn wrap_draw(&self, printer: &Printer) {
        let black = Color::Rgb(0, 0, 0);
        let mut theme = Theme::default();
        theme.palette[PaletteColor::Secondary] = black;
        let new_printer = printer.theme(&theme);
        self.view.draw(&new_printer);
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
        self.cb_sink.send(Box::new(|siv: &mut Cursive| {
            siv.call_on_name(
                "scroll",
                move |view: &mut NamedView<ScrollView<NamedView<BufferView>>>| {
                    view.get_mut().scroll_to_bottom();
                    view.get_mut()
                        .set_scroll_strategy(ScrollStrategy::StickToBottom);
                },
            );
        }))?;
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
