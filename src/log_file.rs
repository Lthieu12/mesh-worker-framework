//! Internal MWF log file management.

use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Error, Write};
use std::path::{Path, PathBuf};

use crate::framework_error::MWFError;
use crate::mwf::MWFConfiguration;
use crate::mwf_log::{LogEvent, LogList, LogOutputFormat, LoggingData};

/// Header written at the beginning of every MWF log file.
///
/// This header allows MWF to distinguish its own log files
/// from user-created files sharing the same name.
const LOG_FILE_HEADER_V1: &str = "############################################################
# Mesh Worker Framework Log File
# Format Version : 1
############################################################
";

/// Message written before file logging is disabled.
const LOG_FILE_FULL_MESSAGE: &str = "Maximum log file size reached.\n\
     File logging disabled.\n";

/// Initializes the MWF log file.
///
/// Existing MWF log files are truncated.
/// Existing non-MWF files are never modified.
pub(crate) fn init_log_file(configuration: &mut MWFConfiguration) -> Result<File, MWFError> {
    // Find a reusable MWF log file.
    configuration.log_file_path = find_log_file_path(&configuration.log_file_path)?;

    // Open and initialize the log file.
    let mut log_file = OpenOptions::new().create(true).write(true).truncate(true).open(&configuration.log_file_path).map_err(|error| log_file_access_failed(error, &configuration.log_file_path))?;

    let log_file_header = format!("{}\n{}\n", LOG_FILE_HEADER_V1, LogList::get_header(LogOutputFormat::CSV),);

    log_file.write_all(log_file_header.as_bytes()).map_err(|error| log_file_access_failed(error, &configuration.log_file_path))?;

    log_file.flush().map_err(|error| log_file_access_failed(error, &configuration.log_file_path))?;

    Ok(log_file)
}

/// Finds a reusable MWF log file.
///
/// If the requested file already exists but is not an MWF log
/// file, numbered filenames are tried until a suitable file
/// is found.
fn find_log_file_path(log_file_path: &Path) -> Result<PathBuf, MWFError> {
    let mut candidate_log_file_path = log_file_path.to_path_buf();
    let mut file_index = 1;

    loop {
        // Candidate log file does not exist.
        if !candidate_log_file_path.exists() {
            return Ok(candidate_log_file_path);
        }

        // Candidate log file belongs to MWF.
        if is_mwf_log_file(&candidate_log_file_path)? {
            return Ok(candidate_log_file_path);
        }

        // Build the next candidate log file path.
        let mut file_name = log_file_path.file_stem().ok_or(MWFError::Logging_LogFileInvalidPath)?.to_os_string();

        file_name.push(format!("_{file_index}"));

        if let Some(extension) = log_file_path.extension() {
            file_name.push(".");
            file_name.push(extension);
        }

        candidate_log_file_path.set_file_name(file_name);

        file_index += 1;
    }
}

/// Returns whether the specified file is an MWF log file.
fn is_mwf_log_file(log_file_path: &Path) -> Result<bool, MWFError> {
    let log_file = File::open(log_file_path).map_err(|error| log_file_access_failed(error, log_file_path))?;

    let mut reader = BufReader::new(log_file);

    let mut header = String::new();

    for _ in 0..4 {
        let mut line = String::new();

        if reader.read_line(&mut line).map_err(|error| log_file_access_failed(error, log_file_path))? == 0 {
            return Ok(false);
        }

        header.push_str(&line);
    }

    Ok(header == LOG_FILE_HEADER_V1)
}

/// Creates a FrameworkError for a log file access failure.
fn log_file_access_failed(error: Error, log_file_path: &Path) -> MWFError {
    MWFError::Logging_LogFileAccessFailed(anyhow::Error::new(error).context(format!("Log file: {}", log_file_path.display(),)))
}

/// Writes a log event to the MWF log file.
pub(crate) fn write_log_event(logging_data: &mut LoggingData, log_event: &LogEvent) -> Result<(), MWFError> {
    // File logging is disabled.
    if !logging_data.log_file_enabled {
        return Ok(());
    }

    // Log file is not available anymore.
    let log_file = match &mut logging_data.log_file {
        Some(log_file) => log_file,

        None => return Ok(()),
    };

    // Convert the log event to a CSV line.
    let log_line = format!("{}\n", log_event.to_csv_string());

    // Write the log event.
    log_file.write_all(log_line.as_bytes()).map_err(|error| log_file_access_failed(error, &logging_data.log_file_path))?;

    // Flush the log file.
    log_file.flush().map_err(|error| log_file_access_failed(error, &logging_data.log_file_path))?;

    // Update the current log file size.
    logging_data.log_file_current_size_bytes += log_line.len() as u64;

    // Disable file logging if the maximum
    // log file size has been reached.
    if logging_data.log_file_current_size_bytes >= logging_data.log_file_max_size_bytes {
        log_file.write_all(LOG_FILE_FULL_MESSAGE.as_bytes()).map_err(|error| log_file_access_failed(error, &logging_data.log_file_path))?;

        log_file.flush().map_err(|error| log_file_access_failed(error, &logging_data.log_file_path))?;

        // Close the log file.
        logging_data.log_file = None;
    }

    Ok(())
}
