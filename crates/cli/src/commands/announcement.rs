//! `trama announcement new`. The job registry of the app runs it through
//! [`crate::exec::Authority::run`].

use super::{app_only, in_app};
use crate::spec::*;

pub fn commands() -> Vec<Command> {
    vec![Command::new("announcement", "new", "Announces a card the agent will file, so that the Board shows a ghost of it.", app_only)
        .notes(&[
            "Prints the `id` to pass as `announcement` when creating the card. An unknown or foreign identity still creates the card, with a note.",
        ])
        .access(Access::Job)
        .input(vec![
            req("kind", one_of(&["proposal", "task", "drift"]), "The kind of card."),
            req("title", line_max(60), "The title, on one line of at most 60 characters."),
        ])
        .output(vec![req("id", line(), "The announcement identity.")])
        .place(in_app)
        .example(r#"trama announcement new --input '{"kind":"task","title":"Cancel an order"}'"#)]
}
