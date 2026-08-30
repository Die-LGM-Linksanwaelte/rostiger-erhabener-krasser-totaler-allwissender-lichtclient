use std::{fmt, fs, io, thread};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::fmt::Formatter;
use std::io::Write;
use std::sync::{mpsc, Arc, Mutex, OnceLock, RwLock};
use std::sync::mpsc::SyncSender;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Defines the severity or category of a log message.
#[derive (Debug,Clone,Copy, Serialize, Deserialize)]
pub enum LogLevel {
    /// A background success event or system confirmation.
    SuccessEvent,
    /// General informational message.
    Info,
    /// Warning about a non-fatal issue or unexpected behavior.
    Warning,
    /// Critical or fatal system error.
    Error,
    /// An error triggered by invalid user input or actions.
    UserError,
    /// A success message resulting directly from a user action.
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

/// Internal representation of a single log event.
#[derive (Debug,Clone)]
pub struct LogMessage {
    /// The severity level of the message.
    level: LogLevel,
    /// The actual log text.
    text: String,
    /// The local time when the message was dispatched.
    timestamp: DateTime<Local>,
    /// Flag indicating whether this message is only relevant in debug mode.
    is_debug: bool,
    
}

/// Trait for destinations that can process and output log messages.
///
/// Sinks must be thread-safe (`Send + Sync`) to be utilized by the background logger.
pub trait LogSink: Send + Sync {
    /// Processes a single incoming log message.
    ///
    /// # Arguments
    ///
    /// * `msg` - Reference to the structured [`LogMessage`]
    fn receive(&self, msg: &LogMessage);
}

/// The central logging dispatcher that routes messages to all registered sinks via a background thread.
pub struct Logger {
    /// Thread-safe collection of all registered log sinks.
    sinks: Arc<RwLock<Vec<Box<dyn LogSink>>>>,
    /// Sender channel used to dispatch messages to the background logging thread.
    log_tx: SyncSender<LogMessage>,
}

impl Logger {
    /// Retrieves or initializes the global singleton instance of the [`Logger`].
    pub fn global() -> &'static Logger {
        static LOGGER: OnceLock<Logger> = OnceLock::new();
        LOGGER.get_or_init(|| {
            let (tx, rx) = mpsc::sync_channel::<LogMessage>(16_384);
            let sinks = Arc::new(RwLock::new(Vec::<Box<dyn LogSink>>::new()));

            let thread_sinks = sinks.clone();

            thread::spawn(move || {
                while let Ok(msg) = rx.recv() {
                    if let Ok(locked_sinks) = thread_sinks.read() {
                        for sink in locked_sinks.iter() {
                            sink.receive(&msg);
                        }
                    }
                }
            });

            Logger {
                sinks,
                log_tx: tx,
            }
        })
    }

    /// Registers a new log sink to receive future messages.
    ///
    /// # Arguments
    ///
    /// * `sink` - A boxed instance implementing the [`LogSink`] trait
    pub fn add_sink(&self, sink: Box<dyn LogSink>) {
        self.sinks.write().unwrap().push(sink);
    }

    /// Creates and sends a new log message to the background processing thread.
    ///
    /// # Arguments
    ///
    /// * `level`    - The severity level of the message
    /// * `text`     - The formatted log text
    /// * `is_debug` - Whether this message should only be visible in debug builds
    pub fn dispatch(&self, level: LogLevel, text: String, is_debug: bool) {
        let msg = LogMessage {
            level,
            text,
            timestamp: Local::now(),
            is_debug,
        };
        // Drop-on-full: logging must never backpressure the caller or balloon memory.
        // try_send fails silently when the bounded queue is saturated or the receiver is gone.
        let _ = self.log_tx.try_send(msg);
    }
}

/// Dispatches a formatted log message to the global logger.
///
/// This macro is the primary interface for logging throughout the application. It behaves
/// exactly like standard Rust formatting macros (e.g., `println!` or `format!`), allowing
/// you to easily interpolate variables into your log messages. Messages sent via `r_log!`
/// are always processed and distributed to all registered sinks (like terminal and files),
/// regardless of whether the application is compiled in debug or release mode.
///
/// # Arguments
///
/// * `$level` - The severity of the log, provided as a [`LogLevel`] variant (e.g., `LogLevel::Info`, `LogLevel::Warning`).
/// * `$arg`   - A format string and an arbitrary number of formatting arguments, following standard `std::fmt` syntax.
///
/// # Examples
///
/// ```rust
/// // Basic usage with a simple string
/// r_log!(LogLevel::Info, "System initialized successfully");
///
/// // Formatting with variables
/// let port = 8080;
/// r_log!(LogLevel::SuccessEvent, "Server listening on port {}", port);
///
/// // Logging complex errors
/// r_log!(LogLevel::Error, "Failed to load configuration file at {}: {}", path, err);
/// ```
#[macro_export]
macro_rules! r_log {
    ($level:expr, $($arg:tt)*) => {
        $crate::logging::Logger::global().dispatch($level, format!($($arg)*), false)
    }
}

/// Dispatches a formatted debug log message to the global logger.
///
/// This macro functions identically to [`r_log!`], but messages logged with this macro
/// are strictly flagged as debug messages. The visibility of these messages depends
/// on the configured [`LogSink`]s.
///
/// Specifically, the default [`TerminalSink`] will ignore and hide these messages when
/// the application is compiled in release mode (i.e., `not(debug_assertions)`).
/// However, the default [`FileSink`] will always record them, regardless of the build
/// profile, ensuring debug trails are kept in the logs without cluttering the user's terminal.
///
/// # Arguments
///
/// * `$level` - The severity of the log, provided as a [`LogLevel`] variant.
/// * `$arg`   - A format string and its arguments, following standard `std::fmt` syntax.
#[macro_export]
macro_rules! r_debug_log {
    ($level:expr, $($arg:tt)*) => {
        $crate::logging::Logger::global().dispatch($level, format!($($arg)*), true)
    };
}

/// A log sink that formats and prints messages to standard output, supporting terminal colors and interactive prompts.
pub struct TerminalSink {
    /// An optional command-line prompt string to restore after printing a log line.
    pub cli_prompt: Option<String>,
}

impl LogSink for TerminalSink {
    fn receive(&self, message: &LogMessage) {
        if message.is_debug && !cfg!(all(debug_assertions, not(test))) {
            return;
        }
        
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
        let time = message.timestamp.format("%H:%M:%S");
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

/// A log sink that writes messages sequentially to a specified file.
pub struct FileSink {
    /// Thread-safe handle to the open log file.
    file: Mutex<File>,
}

impl FileSink {
    /// Initializes a new [`FileSink`]. If a file already exists at the given path, it is archived and renamed.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path where logs should be written
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
        let time = message.timestamp.format("%H:%M:%S");

        let log_line = format!("({}) [{:-^12}] {}\n", time, message.level.to_string(), message.text);

        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(log_line.as_bytes());

            let _ = file.flush();
        }
    }
}