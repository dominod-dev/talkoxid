use cursive::event::{Callback, Event, EventResult, Key};
use cursive::theme::{Color, ColorStyle, ColorType};
use cursive::traits::*;
use cursive::view::{ScrollStrategy, SizeConstraint, ViewWrapper};
use cursive::views::{
    Layer, LinearLayout, ResizedView, ScrollView, SelectView, TextArea, TextView,
};
use cursive::wrap_impl;
use cursive::{CbSink, Cursive};
use std::rc::Rc;
use std::sync::mpsc;

use oxychat::{Channel, Chat, DummyChat, Message};

struct MessageBoxView {
    view: TextArea,
    tx: mpsc::Sender<String>,
}

impl MessageBoxView {
    // Creates a new view with the given buffer size
    fn new(tx: mpsc::Sender<String>) -> Self {
        let view = TextArea::new();
        MessageBoxView { tx, view }
    }
}

fn update_chat(siv: &mut Cursive) {
    siv.call_on_name("chat", |view: &mut BufferView| view._update());
}

impl ViewWrapper for MessageBoxView {
    wrap_impl!(self.view: TextArea);
    fn wrap_on_event<'r>(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Enter) => {
                self.tx.send(String::from(self.view.get_content())).unwrap();
                self.view.set_content("");
                EventResult::Consumed(Some(Callback::from_fn(update_chat)))
            }
            ev => self.view.on_event(ev),
        }
    }
}

struct BufferView {
    _rx: mpsc::Receiver<String>,
    view: TextView,
    _cb_sink: CbSink,
    chat_system: Rc<dyn Chat>,
}

impl BufferView {
    fn new(rx: mpsc::Receiver<String>, cb_sink: CbSink, chat_system: Rc<dyn Chat>) -> Self {
        let view = TextView::new("");
        BufferView {
            _rx: rx,
            view,
            _cb_sink: cb_sink,
            chat_system,
        }
    }

    fn _update(&mut self) {
        // while let Ok(line) = self._rx.try_recv() {
        //     self.view.append(format!("{}\n", &line));
        //     self._cb_sink.send(Box::new(Cursive::noop)).unwrap();
        // }
        self.view.set_content(
            self.chat_system
                .last_10_messages(Channel::Group(String::from("general")))
                .iter()
                .fold(String::from(""), |x, y| {
                    format!("{}\n[{}]: {}", x, y.author, y.content)
                }),
        )
    }
}

impl<'a> ViewWrapper for BufferView {
    wrap_impl!(self.view: TextView);
}

fn update_channel<'r>(_tx: mpsc::Sender<String>) -> impl Fn(&mut Cursive, &str) -> () {
    let closure = move |siv: &mut Cursive, item: &str| {
        siv.call_on_name("chat", |view: &mut BufferView| view._update());
        siv.call_on_name("channel_name", |view: &mut TextView| {
            view.set_content(format!("#{}", item))
        });
    };
    return closure;
}

fn main() {
    let mut siv = cursive::default();
    let chat_system = Rc::new(DummyChat::new());

    let cb_sink = siv.cb_sink().clone();
    siv.add_global_callback('q', |s| s.quit());
    let (tx, rx) = mpsc::channel();
    let tx1 = mpsc::Sender::clone(&tx);
    let tx2 = mpsc::Sender::clone(&tx);

    let white = ColorType::Color(Color::Rgb(255, 255, 255));
    let black = ColorType::Color(Color::Rgb(0, 0, 0));
    let white_on_black = ColorStyle::new(white, black);

    // You can load a theme from a file at runtime for fast development.
    siv.load_theme_file("assets/style.toml").unwrap();

    // Or you can directly load it from a string for easy deployment.
    // siv.load_toml(include_str!("../../assets/style.toml"))
    //     .unwrap();
    let chat_cloned = Rc::clone(&chat_system);
    let mut buffer = ScrollView::new(
        BufferView::new(rx, cb_sink.clone(), chat_cloned)
            .with_name("chat")
            .full_screen(),
    );
    buffer.set_scroll_strategy(ScrollStrategy::StickToBottom);
    let chat = Layer::with_color(
        ResizedView::new(SizeConstraint::Full, SizeConstraint::Full, buffer),
        white_on_black,
    );
    let message_input_box = MessageBoxView::new(tx1);
    let message_input = ResizedView::new(
        SizeConstraint::Full,
        SizeConstraint::AtLeast(10),
        message_input_box,
    );
    let chat_layout = ResizedView::new(
        SizeConstraint::Full,
        SizeConstraint::Full,
        LinearLayout::vertical()
            .child(TextView::new("#general").with_name("channel_name"))
            .child(chat)
            .child(TextView::new("Message:"))
            .child(message_input),
    );
    let chat_cloned = Rc::clone(&chat_system);
    for mess in chat_cloned.last_10_messages(Channel::Group(String::from("general"))) {
        tx.send(format!("[{}]: {}", mess.author, mess.content))
            .unwrap();
    }
    let mut channel_list = SelectView::new().on_submit(update_channel(tx2));
    let chat_cloned = Rc::clone(&chat_system);
    for ch in chat_cloned.channels() {
        channel_list.add_item_str(ch)
    }
    let mut users_list = SelectView::new().on_submit(|s, item: &str| {
        s.call_on_name("channel_name", |view: &mut TextView| view.set_content(item));
    });
    for ch in chat_system.friends() {
        users_list.add_item_str(ch)
    }
    let channel_users = ScrollView::new(
        LinearLayout::vertical()
            .child(TextView::new("CHANNELS:"))
            .child(channel_list)
            .child(TextView::new("USERS:"))
            .child(users_list),
    );
    let channel_layout = ResizedView::new(
        SizeConstraint::AtLeast(20),
        SizeConstraint::Full,
        channel_users,
    );
    let global_layout = LinearLayout::horizontal()
        .child(channel_layout)
        .child(chat_layout);

    siv.add_fullscreen_layer(global_layout);

    siv.run();
}
