use std::{pin::pin, time::Duration};

use futures::{Stream, StreamExt as _, stream};
use ratatui::{Terminal, prelude::Backend};
use tokio::sync::mpsc;

use self::state::UIEvent;
use crate::{
    facade::{WVState, WriteVerifyWorkflow, watch::Watch},
    logging::LogPaths,
    tui::fancy_ui::{display::draw, state::State},
    util::runtime::RemoteSpawn,
};

mod display;
mod state;
mod widgets;

/// How often we refresh the display
const REFRESH_PERIOD: Duration = Duration::from_millis(100);

pub struct Params<'a, B, T>
where
    B: Backend + 'a,
    T: Stream<Item = std::io::Result<crossterm::event::Event>> + Send + 'static,
{
    pub terminal: &'a mut Terminal<B>,
    pub begin: &'a WriteVerifyWorkflow,
    pub child_state: Watch<WVState>,
    pub terminal_events: T,
    pub log_paths: &'a LogPaths,
}

/// Run the fancy TUI.
#[tracing::instrument(skip_all)]
pub fn run<'a, B, T>(runtime: impl RemoteSpawn, params: Params<'a, B, T>)
where
    B: Backend,
    T: Stream<Item = std::io::Result<crossterm::event::Event>> + Send + 'static,
{
    let (tx, mut rx) = mpsc::channel(128);

    // Aggregate events together inside the async thread
    let terminal_events = params.terminal_events;
    runtime.spawn(move || async move {
        let mut events = pin!(create_event_stream(terminal_events));

        while let Some(ev) = events.next().await {
            let Ok(_) = tx.send(ev).await else {
                return;
            };
        }
    });

    // Start the draw loop, which reduces events it receives
    let state = State::initial(params.begin);
    let events_iter = std::iter::from_fn(move || rx.blocking_recv());
    draw_loop(
        params.terminal,
        state,
        events_iter,
        params.child_state,
        params.begin,
        params.log_paths,
    );
}

/// Creates the main [`UIEvent`] stream to be fed to the fancy UI.
fn create_event_stream<'a>(
    terminal_events: impl Stream<Item = std::io::Result<crossterm::event::Event>> + 'a,
) -> impl Stream<Item = UIEvent> + 'a {
    let terminal_events = terminal_events.map(|e: std::io::Result<crossterm::event::Event>| {
        UIEvent::RecvTermEvent(e.map_err(|e| (e.to_string(), e.kind())))
    });

    let timeout_events =
        stream::unfold(tokio::time::interval(REFRESH_PERIOD), |mut i| async move {
            i.tick().await;
            Some((UIEvent::SleepTimeout, i))
        });

    stream::select(terminal_events, timeout_events)
}

/// Synchronous get-UI-event-and-draw loop.
///
/// Panics if the terminal somehow can't be written to.
fn draw_loop(
    terminal: &mut Terminal<impl Backend>,
    mut state: State,
    events: impl IntoIterator<Item = UIEvent>,
    child: Watch<WVState>,
    begin: &WriteVerifyWorkflow,
    log_paths: &LogPaths,
) {
    let mut notification_sent = false;
    for ev in events {
        let child = child.borrow();

        if child.is_finished() && !notification_sent {
            use crate::facade::workflow::WorkflowState as _;
            let image_name = begin.input_file.file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_else(|| begin.input_file.to_string_lossy());
            let device_path = begin.target.devnode.to_string_lossy();

            match child.result() {
                Some(Ok(_)) => crate::util::notification::send_terminal_notification(
                    "Caligula",
                    &format!("Burn of {} to {} finished successfully!", image_name, device_path),
                ),
                Some(Err(e)) => crate::util::notification::send_terminal_notification(
                    "Caligula Error",
                    &format!("Error burning {} to {}: {}", image_name, device_path, e),
                ),
                None => {}
            }
            notification_sent = true;
        }

        let Some(new_state) = state.on_event(&child, ev) else {
            return;
        };
        state = new_state;

        draw(&mut state, &child, terminal, log_paths).expect("Failed to write to terminal");
    }
}
