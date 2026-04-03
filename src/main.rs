mod model;

use iced::{Element, Task, Theme};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Markdown Editor")
        .theme(App::theme)
        .run()
}

#[derive(Debug, Clone)]
enum Message {}

struct App;

impl App {
    fn new() -> (Self, Task<Message>) {
        (Self, Task::none())
    }

    fn theme(&self) -> Theme {
        Theme::TokyoNight
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        "md-editor".into()
    }
}
