use std::{fmt, fs, io};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::fmt::Formatter;
use std::io::{Write};
use std::sync::{Mutex, OnceLock, RwLock};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive (Debug,Clone,Copy, Serialize, Deserialize)]
pub enum LogLevel {
    SuccessEvent,
    Info,
    Warning,
    Error,
    UserError,
    UserSuccess,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let text = match self {
            LogLevel::SuccessEvent =>   "Success",
            LogLevel::Info =>           "INFO",
            LogLevel::Warning =>        "WARN",
            LogLevel::Error =>          "ERROR",
            LogLevel::UserError =>      "USER ERROR",
            LogLevel::UserSuccess =>    "USER success"
        };

        write!(f, "{text}")
    }
}


#[derive (Debug,Clone)]
pub struct LogMessage {
    pub level: LogLevel,
    pub text: String,
}

pub trait LogSink: Send + Sync {
    fn receive(&self, msg: &LogMessage);
}

pub struct Logger {
    sinks: RwLock<Vec<Box<dyn LogSink>>>
}

impl Logger {
    pub fn global() -> &'static Logger {
        static LOGGER: OnceLock<Logger> = OnceLock::new();
        LOGGER.get_or_init(|| Logger {
            sinks: RwLock::new(Vec::new()),
        })
    }

    pub fn add_sink(&self, sink: Box<dyn LogSink>) {
        self.sinks.write().unwrap().push(sink);
    }

    pub fn dispatch(&self, level: LogLevel, text: String) {
        let msg = LogMessage { level, text };

        if let Ok(sinks) = self.sinks.read() {
            for sink in sinks.iter() {
                sink.receive(&msg);
            }
        }
    }
}

#[macro_export]
macro_rules! r_log {
    ($level:expr, $($arg:tt)*) => {
        $crate::logging::Logger::global().dispatch($level, format!($($arg)*));
    }
}

pub struct TerminalSink {
    pub cli_prompt: Option<String>,
}

impl LogSink for TerminalSink {
    fn receive(&self, message: &LogMessage) {
        let color = match message.level {
            LogLevel::SuccessEvent =>   "\x1B[42m\x1B[30m",
            LogLevel::Info =>           "\x1B[44m\x1B[30m",
            LogLevel::Warning =>        "\x1B[43m\x1B[30m",
            LogLevel::Error =>          "\x1B[41m\x1B[30m",
            LogLevel::UserError =>      "\x1B[35m",
            LogLevel::UserSuccess =>    "\x1B[32m",
        };

        let width = crossterm::terminal::size()
            .map(|(width, _height)| (width as usize).saturating_sub(2).min(500) )
            .unwrap_or(80);
        let time = Local::now().format("%H:%M:%S");
        let raw_line = format!("({}) [{:-^12}] {}", time, message.level.to_string(), message.text);

        let is_system_message = color.contains("\x1B[4");
        let log_line = match is_system_message {
            true =>     format!("{} {:<width$} \x1B[0m", color, raw_line),
            false =>    format!("{} {} \x1B[0m", color, raw_line)
        };


        let stdout = io::stdout();
        let mut handle = stdout.lock();

        if let Some(prompt) = &self.cli_prompt {
            write!(handle,"\r\x1b[2K").unwrap();
            writeln!(handle, "{}", log_line).unwrap();
            write!(handle, "{}", prompt).unwrap();
            handle.flush().unwrap();
        } else {
            writeln!(handle, "{}", log_line).unwrap();
            handle.flush().unwrap();
        }

    }
}

pub struct FileSink {
    file: Mutex<File>,
}

impl FileSink {
    pub fn new(path: &str) -> FileSink {
        let path_object = Path::new(path);

        if path_object.exists() {
            if let Ok(metadata) = path_object.metadata() {
                let (time_str, prefix) = match fs::metadata(&path).and_then(|m| m.created().or_else(|_| metadata.modified())) {
                    Ok(file_time) => {
                        let datetime: DateTime<Local> = file_time.into();
                        (datetime.format("%Y-%m-%d_%H-%M-%S").to_string(), "")
                    }
                    Err(_) => {
                        let now = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
                        (now, "old_file_backuped_")
                    }
                };

                let file_stem = path_object.file_stem().and_then(|s| s.to_str()).unwrap_or("backup");
                let ext = path_object.extension().and_then(|s| s.to_str()).unwrap_or("log");

                let archive_name = format!("{}{}_{}.{}", prefix, time_str, file_stem, ext);
                let archive_path = path_object.with_file_name(archive_name);

                if let Err(e) = fs::rename(&path_object, &archive_path) {
                    println!("Warning: Couldnt archive old Log-File: {}", e);
                }
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .expect("Failed to open log file");

        Self {
            file: Mutex::new(file)
        }
    }
}

impl LogSink for FileSink {
    fn receive(&self, message: &LogMessage) {
        let time = Local::now().format("%H:%M:%S");

        let log_line = format!("({}) [{:-^12}] {}\n", time, message.level.to_string(), message.text);

        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(log_line.as_bytes());

            let _ = file.flush();
        }
    }
}