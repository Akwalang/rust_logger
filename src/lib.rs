use std::fmt;
use std::io::{self, Write};

pub mod internal {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::sync::Once;

    #[cfg(windows)]
    mod console_win {
        const STD_OUTPUT_HANDLE: i32 = -11;
        const STD_ERROR_HANDLE: i32 = -12;
        const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

        #[link(name = "kernel32")]
        extern "system" {
            fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
            fn SetConsoleCP(wCodePageID: u32) -> i32;
            fn GetStdHandle(nStdHandle: i32) -> isize;
            fn GetConsoleMode(hConsoleHandle: isize, lpMode: *mut u32) -> i32;
            fn SetConsoleMode(hConsoleHandle: isize, dwMode: u32) -> i32;
        }

        pub unsafe fn enable_utf8_and_ansi() {
            let _ = SetConsoleOutputCP(65001);
            let _ = SetConsoleCP(65001);

            let out = GetStdHandle(STD_OUTPUT_HANDLE);
            if out != 0 && out != -1 {
                let mut mode: u32 = 0;
                if GetConsoleMode(out, &mut mode as *mut u32) != 0 {
                    let _ = SetConsoleMode(out, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
                }
            }

            let err = GetStdHandle(STD_ERROR_HANDLE);
            if err != 0 && err != -1 {
                let mut mode: u32 = 0;
                if GetConsoleMode(err, &mut mode as *mut u32) != 0 {
                    let _ = SetConsoleMode(err, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
                }
            }
        }

        // no direct WriteConsoleW; standard UTF-8 writes are fine once code page is set
    }

    static INIT: Once = Once::new();
    fn init_console_once() {
        INIT.call_once(|| {
            #[cfg(windows)]
            unsafe {
                console_win::enable_utf8_and_ansi();
            }
        });
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
    pub enum Level {
        Debug,
        Info,
        Warn,
        Error,
        None,
    }

    fn parse_level(s: &str) -> Level {
        match s.to_ascii_lowercase().as_str() {
            "debug" => Level::Debug,
            "info" => Level::Info,
            "warn" => Level::Warn,
            "error" => Level::Error,
            "none" => Level::None,
            _ => Level::Debug,
        }
    }

    const BUILD_LOG_LEVEL: &str = env!("LOG_LEVEL");

    fn current_level() -> Level {
      parse_level(BUILD_LOG_LEVEL)
    }

    pub fn is_enabled(level: Level) -> bool {
        match (current_level(), level) {
            (Level::None, _) => false,
            (Level::Error, Level::Error) => true,
            (Level::Error, _) => false,
            (Level::Warn, Level::Error | Level::Warn) => true,
            (Level::Warn, _) => false,
            (Level::Info, Level::Error | Level::Warn | Level::Info) => true,
            (Level::Info, _) => false,
            (Level::Debug, _) => true,
        }
    }

    fn level_styles(level: Level) -> (&'static str, &'static str, &'static str, &'static str) {
        match level {
            Level::Debug => ("100", "DBG", "90", "30"),  // bg bright black (gray), fg gray
            Level::Info => ("44", "LOG", "34", "37"),    // bg blue, fg blue
            Level::Warn => ("43", "WRN", "33", "33"),    // bg yellow, fg yellow
            Level::Error => ("41", "ERR", "31", "31"),   // bg red, fg red
            Level::None => ("0", "", "0", "0"),
        }
    }

    /// Returns (fg_code, bright_bold)
    fn color_name_to_fg_code(name: &str) -> Option<(&'static str, bool)> {
        match name.to_ascii_lowercase().as_str() {
            "black" => Some(("30", false)),
            "red" => Some(("31", false)),
            "green" => Some(("32", false)),
            "orange" | "yellow" => Some(("33", false)),
            "blue" => Some(("34", false)),
            "purple" | "magenta" => Some(("35", false)),
            "cyan" => Some(("36", false)),
            "white" => Some(("37", false)),
            "gray" => Some(("90", false)),
            _ => None,
        }
    }

    fn apply_markup(input: &str, default_seq: &str) -> String {
        let mut out = String::with_capacity(input.len() + 16);
        let bytes = input.as_bytes();

        let mut i: usize = 0;

        while i < bytes.len() {
            if bytes[i] == b'<' {
                // find closing '>'
                if let Some(gt) = input[i..].find('>') {
                    let tag_inner = &input[i + 1..i + gt];

                    // ensure closing marker exists
                    if let Some(close_pos_rel) = input[i + gt + 1..].find("</>") {
                        let content_start = i + gt + 1;
                        let content_end = content_start + close_pos_rel;
                        let content = &input[content_start..content_end];

                        // parse tag: flexible tokens: comma-separated any of [color, italic, bold, underline]
                        let tokens: Vec<_> = tag_inner.split(',').map(|s| s.trim()).collect();

                        let mut italic_on = false;
                        let mut bold_on = false;
                        let mut underline_on = false;
                        let mut color_fg: Option<&str> = None;
                        let mut color_bright_bold = false;

                        for token in tokens.into_iter().filter(|s| !s.is_empty()) {
                            let lower = token.to_ascii_lowercase();
                            match lower.as_str() {
                                "italic" | "i" => { italic_on = true; }
                                "bold" | "b" => { bold_on = true; }
                                "underline" | "u" => { underline_on = true; }
                                _ => {
                                    if color_fg.is_none() {
                                        if let Some((fg, bright)) = color_name_to_fg_code(&lower) {
                                            color_fg = Some(fg);
                                            color_bright_bold = bright;
                                        }
                                    }
                                }
                            }
                        }

                        if color_bright_bold { bold_on = true; }

                        // Build opening sequence
                        let mut seq = String::new();

                        if bold_on { seq.push_str(";1"); }
                        if italic_on { seq.push_str(";3"); }
                        if underline_on { seq.push_str(";4"); }

                        if let Some(c) = color_fg {
                          seq.push_str(";"); seq.push_str(c);
                        }

                        if seq.is_empty() {
                            // no styling -> just append content and markers removed
                            out.push_str(content);
                        } else {
                            out.push_str("\x1b[");
                            out.push_str(&seq[1..]); // skip leading ';'
                            out.push('m');
                            out.push_str(content);
                            out.push_str("\x1b[0m");

                            if !default_seq.is_empty() {
                                out.push_str(default_seq);
                            }
                        }

                        i = content_end + 3; // skip "</>"

                        continue;
                    }
                }
            }

            let ch = input[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }

        out
    }

    fn days_to_ymd(days_since_epoch: i64) -> (i32, i32, i32) {
        let z = days_since_epoch + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097; // [0, 146096]
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
        let y = (yoe as i32) + (era as i32) * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100 + yoe / 400); // [0, 365]
        let mp = (5 * doy + 2) / 153; // [0, 11]
        let d = (doy - (153 * mp + 2) / 5 + 1) as i32; // [1, 31]
        let m = (mp + if mp < 10 { 3 } else { -9 }) as i32; // [1, 12]
        let y = y + if m <= 2 { 1 } else { 0 };

        (y, m, d)
    }

    fn format_timestamp_utc() -> String {
        let now = SystemTime::now();
        let dur = now.duration_since(UNIX_EPOCH).unwrap_or_default();
        let total_secs = dur.as_secs() as i64;
        let millis = dur.subsec_millis() as i32;
        
        let days = total_secs / 86_400;
        let sod = (total_secs % 86_400) as i64;
        let hour = (sod / 3_600) as i32;
        let min = ((sod % 3_600) / 60) as i32;
        let sec = (sod % 60) as i32;

        let (y, m, d) = days_to_ymd(days);

        format!("{:04}.{:02}.{:02} {:02}:{:02}:{:02}.{:03}", y, m, d, hour, min, sec, millis)
    }

    pub fn print_with_prefix(level: Level, args: fmt::Arguments) {
        init_console_once();

        let (bg, label, date, font) = level_styles(level);

        let ts = format_timestamp_utc();
        let prefix_label = format!("\x1b[0;{bg};38;2;0;0;0m {label} \x1b[0m ");
        let default_date_seq = if date != "0" { format!("\x1b[{date}m") } else { String::new() };
        let default_font_seq = if font != "0" { format!("\x1b[{font}m") } else { String::new() };
        let message_raw = format!("{}", args);
        let message_colored = apply_markup(&message_raw, &default_font_seq);

        let mut stdout = io::stdout();
        
        let ts_block = if default_date_seq.is_empty() {
            format!("[{ts}] ")
        } else {
            format!("{default_date_seq}[{ts}] ")
        };

        let msg_block = if default_font_seq.is_empty() {
            format!("{message_colored} ")
        } else {
            format!("{default_font_seq}{message_colored} ")
        };

        let final_line = format!("{prefix_label}{ts_block}{msg_block}\x1b[0m");
        let _ = writeln!(stdout, "{final_line}");
    }

    pub fn print_new_line() {
        init_console_once();

        let mut stdout = io::stdout();
        let _ = writeln!(stdout);
    }
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        if $crate::internal::is_enabled($crate::internal::Level::Debug) {
            $crate::internal::print_with_prefix($crate::internal::Level::Debug, format_args!($($arg)*));
        }
    }};
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        if $crate::internal::is_enabled($crate::internal::Level::Info) {
            $crate::internal::print_with_prefix($crate::internal::Level::Info, format_args!($($arg)*));
        }
    }};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        if $crate::internal::is_enabled($crate::internal::Level::Warn) {
            $crate::internal::print_with_prefix($crate::internal::Level::Warn, format_args!($($arg)*));
        }
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        if $crate::internal::is_enabled($crate::internal::Level::Error) {
            $crate::internal::print_with_prefix($crate::internal::Level::Error, format_args!($($arg)*));
        }
    }};
}

#[macro_export]
macro_rules! new_line {
    () => {{
        $crate::internal::print_new_line();
    }};
}
