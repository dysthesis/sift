use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value_t = 3000)]
    pub port: i16,

    // Logging controls
    /// Base log level for `siftd` and `libsift` (others remain warn)
    #[arg(long, value_enum)]
    pub log_level: Option<LogLevel>,

    /// Full filter directive string, e.g. "libsift=debug,sqlx=warn,tower_http=info"
    #[arg(long)]
    pub log_filter: Option<String>,

    /// Increase verbosity (-v=info, -vv=debug, -vvv=trace)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Reduce verbosity to warnings only (overrides -v)
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Log output format: auto (tty->pretty, non-tty->json), pretty, compact, or json
    #[arg(long, value_enum, default_value_t = LogFormat::Auto)]
    pub log_format: LogFormat,

    /// Color output: auto (respect NO_COLOR), always, or never
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,

    /// Optional log file path (non-blocking writer); defaults to stderr if not set
    #[arg(long)]
    pub log_file: Option<PathBuf>,

    /// Include query strings in logs (still only at debug/trace)
    #[arg(long, default_value_t = false)]
    pub log_queries: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Copy, Clone, Debug, Default, ValueEnum)]
pub enum LogFormat {
    #[default]
    Auto,
    Pretty,
    Compact,
    Json,
}

#[derive(Copy, Clone, Debug, Default, ValueEnum)]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}
