use std::borrow::Cow;
use std::sync::Arc;

use skim::prelude::*;
use skim::tui::options::PreviewLayout;

use crate::tmux::Tmux;

struct Item {
    name: String,
    tmux: Tmux,
}

impl SkimItem for Item {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }

    fn preview(&self, context: PreviewContext) -> ItemPreview {
        self.tmux.focus_session(Some(self.name.clone()));
        ItemPreview::AnsiText(self.tmux.preview(&self.name, context.width, context.height))
    }
}

pub fn run(tmux: Tmux, sessions: Vec<String>) {
    let options = SkimOptionsBuilder::default()
        .height("90%".to_owned())
        .reverse(true)
        .preview(Some("".to_owned()))
        .preview_window(PreviewLayout::from("down:60%"))
        .prompt("Session: ".to_owned())
        .build()
        .unwrap();

    let (tx, rx) = unbounded();
    for name in sessions {
        let item = Item {
            name,
            tmux: tmux.clone(),
        };

        tx.send(Arc::new(item) as Arc<dyn SkimItem>).ok();
    }

    drop(tx);
    let _ = Skim::run_with(options, Some(rx));
}
