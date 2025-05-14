use std::fs;

use fern::Dispatch;
use log::LevelFilter;
use chrono::Local;

/// # Setup Logging
/// This function sets up the logging for the application. It creates two log files: `logs.log` and `errors.log`.
pub fn setup_logging() -> Result<(), fern::InitError> {
    // Create the logs folder if it does not exist
    fs::create_dir_all("logs")?;

    // Configure the logger for general logs
    let general_log = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Info)
        .chain(fern::log_file("logs/log.log")?);

    // Configure the logger for error logs
    let error_log = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Error)
        .chain(fern::log_file("logs/errors.log")?);

    // Combine the two loggers into the global logger
    Dispatch::new()
        .chain(general_log)
        .chain(error_log)
        .apply()?;

    Ok(())
}
