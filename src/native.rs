//! The card Tern draws for a running cocaine, over the Tern Surface Protocol:
//! what it prevents, how long it has kept the machine awake and how long it
//! has left. Tern ticks the timers itself, so cocaine only redraws when the
//! progress bar moves a whole percent. Other terminals keep the plain lines.

use std::time::{Duration, Instant};

use tern_sdk::{
    Font, Input, Options, Session, SurfaceOptions, nodes,
    ui::{self, Status, View},
};

use crate::format_duration;
use crate::power::PowerAssertion;

/// How cocaine stopped.
enum Ended {
    /// The timeout ran out.
    Timeout,
    /// Ctrl+C, or the terminal went away.
    Stopped,
}

/// Keeps the machine awake in a native card until Ctrl+C or `timeout`.
/// False, having shown nothing, when the terminal isn't Tern (or `TERN_TSP=0`).
pub fn run(timeout: Option<Duration>, assertions: &[PowerAssertion]) -> bool {
    if !in_tern() {
        return false;
    }
    let options = Options::new()
        .app("cocaine")
        .version(env!("CARGO_PKG_VERSION"))
        .bracketed_paste(false)
        .kitty_keyboard(false);
    let Ok(Some(mut session)) = Session::<()>::connect(options) else {
        return false;
    };
    let Ok(sf) = session.open(SurfaceOptions::flow().font(Font::Sans).title("cocaine")) else {
        let _ = session.close();
        return false;
    };

    let prevents = assertions
        .iter()
        .map(|a| a.assertion_type().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let start = Instant::now();
    let mut shown = percent(start, timeout);
    let _ = session.render(sf, running(&prevents, start, timeout));

    let ended = loop {
        let wait = match timeout {
            Some(t) => match t.checked_sub(start.elapsed()) {
                Some(left) if !left.is_zero() => Some(left.min(Duration::from_secs(1))),
                _ => break Ended::Timeout,
            },
            None => None,
        };
        match session.next(wait) {
            Ok(Some(Input::Key(key))) if key.is("ctrl+c") => break Ended::Stopped,
            Ok(_) => {}
            Err(_) => break Ended::Stopped,
        }
        let now = percent(start, timeout);
        if now != shown {
            shown = now;
            let _ = session.render(sf, running(&prevents, start, timeout));
        }
    };

    let _ = session.render(sf, finished(&prevents, start.elapsed(), timeout, &ended));
    let _ = session.close();
    true
}

/// Whether to try TSP at all: in a Tern pane, or in tmux (which the SDK
/// checks for Tern itself). Other terminals never see the probe.
fn in_tern() -> bool {
    std::env::var_os("TERM_PROGRAM").is_some_and(|p| p == "tern")
        || std::env::var_os("TMUX").is_some_and(|v| !v.is_empty())
}

/// The whole percent of `timeout` gone since `start`.
fn percent(start: Instant, timeout: Option<Duration>) -> Option<u64> {
    let t = timeout?.as_millis().max(1);
    Some((start.elapsed().as_millis() * 100 / t).min(100) as u64)
}

fn muted(text: &str) -> ui::TextNode {
    ui::text(ui::span(text).style("muted"))
}

/// The card while cocaine runs.
fn running(prevents: &str, start: Instant, timeout: Option<Duration>) -> View {
    let mut clock = nodes![muted("Awake for"), ui::elapsed().since(start)];
    let mut card = ui::card()
        .key("awake")
        .icon("bolt")
        .head("Keeping awake")
        .meta(prevents)
        .status(Status::Running);
    if let Some(t) = timeout {
        let left = t.saturating_sub(start.elapsed()).as_millis();
        clock.push(muted("·").into());
        clock.push(
            ui::elapsed()
                .age(-i64::try_from(left).unwrap_or(i64::MAX))
                .into(),
        );
        clock.push(muted("left").into());
        let done = percent(start, timeout).unwrap_or(0);
        card = card
            .child(ui::row().key("clock").gap("sm").children(clock))
            .child(
                ui::progress()
                    .key("bar")
                    .value(done as f64 / 100.0)
                    .label(format!("{done}%")),
            );
    } else {
        card = card.child(ui::row().key("clock").gap("sm").children(clock));
    }
    let stop = ui::row()
        .key("stop")
        .gap("sm")
        .children(nodes![ui::kbd(["ctrl", "c"]), muted("to stop")]);
    View::new().main([card.child(stop)])
}

/// The card cocaine leaves behind.
fn finished(prevents: &str, took: Duration, timeout: Option<Duration>, ended: &Ended) -> View {
    let how = match (ended, timeout) {
        (Ended::Timeout, Some(t)) => format!("at its {} limit", format_duration(t)),
        _ => "with Ctrl+C".to_string(),
    };
    View::new().main([ui::card()
        .key("awake")
        .icon("bolt")
        .head(format!("Kept awake for {}", format_duration(took)))
        .meta(prevents)
        .status(Status::Done)
        .child(
            ui::row()
                .key("clock")
                .gap("sm")
                .children(nodes![muted("Ended"), ui::text(how)]),
        )])
}
