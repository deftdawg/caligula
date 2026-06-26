use std::io::{Write, stdout};
use is_terminal::IsTerminal;

/// Sends a notification to the terminal using OSC 9, 99, and 777 sequences.
///
/// Supported by iTerm2 (OSC 9), Ghostty/cmux (OSC 99), and urxvt (OSC 777).
pub fn send_terminal_notification(title: &str, body: &str) {
    if !stdout().is_terminal() {
        return;
    }

    // OSC 9: iTerm2
    print!("\x1b]9;{}\x1b\\", body);

    // OSC 777: urxvt
    print!("\x1b]777;notify;{};{}\x1b\\", title, body);

    // OSC 99: Ghostty / cmux
    // Format: \x1b]99;notify;t=<title>;b=<body>\x1b\\
    print!("\x1b]99;notify;t={};b={}\x1b\\", title, body);

    let _ = stdout().flush();
}
