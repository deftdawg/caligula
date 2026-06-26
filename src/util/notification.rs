use std::io::{Write, stdout};
use is_terminal::IsTerminal;
use base64::{Engine as _, engine::general_purpose::STANDARD};

/// Sends a notification to the terminal using OSC 9, 99, and 777 sequences.
///
/// Supported by iTerm2 (OSC 9), Kitty/cmux (OSC 99), and urxvt/Ghostty (OSC 777).
pub fn send_terminal_notification(title: &str, body: &str) {
    if !stdout().is_terminal() {
        return;
    }

    // OSC 9: iTerm2
    print!("\x1b]9;{}\x1b\\", body);

    // OSC 777: urxvt
    print!("\x1b]777;notify;{};{}\x1b\\", title, body);

    // OSC 99: Kitty / cmux (standard protocol)
    let title_b64 = STANDARD.encode(title);
    let body_b64 = STANDARD.encode(body);
    print!("\x1b]99;i=caligula:p=title:d=0:e=1;{}\x1b\\", title_b64);
    print!("\x1b]99;i=caligula:p=body:e=1;{}\x1b\\", body_b64);

    let _ = stdout().flush();
}
