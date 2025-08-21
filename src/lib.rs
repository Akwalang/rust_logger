use std::fmt;
use std::collections::HashMap;
use std::sync::{Mutex, LazyLock};

static ALIASES: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub mod internal {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn register_alias(alias: &str, tokens: &str) {
        let mut aliases = ALIASES.lock().unwrap();
        aliases.insert(alias.to_string(), tokens.to_string());
    }

    pub fn get_alias(alias: &str) -> Option<String> {
        let aliases = ALIASES.lock().unwrap();
        aliases.get(alias).cloned()
    }

    pub fn clear_aliases() {
        let mut aliases = ALIASES.lock().unwrap();
        aliases.clear();
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

                        // Check if this is an alias first
                        let tokens_to_process: Vec<String> = if let Some(alias_tokens) = get_alias(tag_inner) {
                            alias_tokens.split(',').map(|s| s.trim().to_string()).collect()
                        } else {
                            // parse tag: flexible tokens: comma-separated any of [color, italic, bold, underline]
                            tag_inner.split(',').map(|s| s.trim().to_string()).collect()
                        };

                        let mut italic_on = false;
                        let mut bold_on = false;
                        let mut underline_on = false;
                        let mut color_fg: Option<&str> = None;
                        let mut color_bright_bold = false;

                        for token in tokens_to_process.into_iter().filter(|s| !s.is_empty()) {
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
        let (bg, label, date, font) = level_styles(level);

        let ts = format_timestamp_utc();
        let prefix_label = format!("\x1b[0;{bg};38;2;0;0;0m {label} \x1b[0m ");
        let default_date_seq = if date != "0" { format!("\x1b[{date}m") } else { String::new() };
        let default_font_seq = if font != "0" { format!("\x1b[{font}m") } else { String::new() };
        let message_raw = format!("{}", args);
        let message_colored = apply_markup(&message_raw, &default_font_seq);

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
        println!("{}", final_line);
    }

    pub fn print_new_line() {
        println!("");
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

#[macro_export]
macro_rules! alias {
    ($alias:expr, $tokens:expr) => {{
        $crate::internal::register_alias($alias, $tokens);
    }};
}
