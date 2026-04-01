use iced::widget::{markdown, row, scrollable, text_editor};
use iced::{Element, Fill, Task, Theme};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Markdown Editor")
        .theme(App::theme)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    LinkClicked(String),
}

struct App {
    content: text_editor::Content,
    items: Vec<markdown::Item>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let source = "# 欢迎使用 Markdown 编辑器

左侧编辑，右侧**实时预览**。

## 支持的语法

- **粗体** 和 *斜体*
- `行内代码`
- [链接](https://example.com)

## 代码块

```rust
fn main() {
    println!(\"Hello, world!\");
}
```

> 引用块也支持

---

1. 第一项
2. 第二项
3. 第三项
";
        let app = Self {
            items: markdown::parse(source).collect(),
            content: text_editor::Content::with_text(source),
        };
        (app, Task::none())
    }

    fn theme(&self) -> Theme {
        Theme::TokyoNight
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {
                self.content.perform(action);
                self.items = markdown::parse(&self.content.text()).collect();
                Task::none()
            }
            Message::LinkClicked(url) => {
                let _ = open::that(&url);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let editor = text_editor(&self.content)
            .on_action(Message::Edit)
            .height(Fill)
            .highlight("markdown", iced::highlighter::Theme::SolarizedDark);

        let preview = scrollable(
            markdown::view(
                &self.items,
                markdown::Settings::with_style(
                    markdown::Style::from_palette(Theme::TokyoNight.palette()),
                ),
            )
            .map(Message::LinkClicked),
        )
        .height(Fill);

        row![editor, preview]
            .spacing(10)
            .padding(10)
            .into()
    }
}
