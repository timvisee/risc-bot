extern crate htmlescape;
extern crate tokio_timer;

use std::process::ExitStatus;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use state::State;

use self::htmlescape::encode_minimal;
use self::tokio_timer::{Error as TimerError, Interval};
use failure::{err_msg, Compat, Error as FailureError, SyncFailure};
use futures::{
    future::{err, ok},
    Future, Stream,
};
use humantime::format_duration;
use telegram_bot::{
    prelude::*,
    types::{Message, MessageKind, ParseMode},
    Error as TelegramError,
};

use super::Action;
use executor::{isolated, Error as ExecutorError};

/// The action command name.
const CMD: &'static str = "exec";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &'static str = "Execute a shell command";

/// The number of characters to truncate the output log at.
const OUTPUT_TRUNCATE: usize = 4096 - 150;

/// The timeout duration for commands being executed.
const EXEC_TIMEOUT: Duration = Duration::from_secs(300);

/// The worst precision of the timeout duration for the commands being executed.
const EXEC_TIMEOUT_PRECISION: Duration = Duration::from_secs(1);

pub struct Exec;

impl Exec {
    pub fn new() -> Self {
        Exec
    }

    /// Execute the given command in isolated environment.
    ///
    /// Send a reply to the given user command `msg`
    /// and timely update it to show the status of the command that was executed.
    pub fn exec_cmd<'a>(
        state: &State,
        cmd: String,
        msg: &Message,
    ) -> impl Future<Item = (), Error = Error> {
        // Create the status message, and build the executable status object
        let exec_status = ExecStatus::create_status_msg(state.clone(), msg);

        // Build a future for the command execution, and status updating
        exec_status.and_then(move |status| {
            // Create an mutexed arc for the status
            let status = Arc::new(Mutex::new(status));

            // Execute the command in an isolated environment, process output and the exit code
            let status_output = status.clone();
            let status_exit = status.clone();
            let cmd = isolated::execute(cmd, move |line| {
                // Append the line to the captured output
                status_output.lock().unwrap().append_line(&line);
                Ok(())
            })
            .and_then(move |status| {
                // Set the exit status
                status_exit.lock().unwrap().set_status(status);
                ok(())
            })
            .map_err(|err| Error::Execute(err));

            // Set up an interval for constantly updating the status
            let status_update = status.clone();
            Interval::new(
                Instant::now() + Duration::from_millis(1000),
                Duration::from_millis(1000),
            )
            .map_err(|err| Error::Throttle(err))
            .for_each(move |_| {
                // Update the status on Telegram, throttled
                status_update.lock().unwrap().update_throttled();
                Ok(())
            })
            .select(cmd)
            .map_err(|(err, _)| err)
            .and_then(move |_| {
                // Update one final time, to ensure all status is sent to Telegram
                status.lock().unwrap().update();
                ok(())
            })
        })
    }
}

impl Action for Exec {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn hidden(&self) -> bool {
        HIDDEN
    }

    fn help(&self) -> &'static str {
        HELP
    }

    // TODO: proper error handling everywhere, pass errors along
    fn invoke(&self, state: &State, msg: &Message) -> Box<Future<Item = (), Error = FailureError>> {
        if let MessageKind::Text { ref data, .. } = &msg.kind {
            // The command to run in the shell
            // TODO: actually properly fetch the command to execute from the full message
            let cmd = data
                .splitn(2, ' ')
                .skip(1)
                .next()
                .map(|cmd| cmd.trim_left())
                .unwrap_or("")
                .to_owned();

            // Provide the user with feedback if no command is entered
            if cmd.trim().is_empty() {
                // Build a future for sending the help message
                let future = state
                    .telegram_send(
                        msg.text_reply(
                            "\
                             Please provide a command to run.\n\
                             \n\
                             For example:\n\
                             `/exec echo Hello!`\
                             ",
                        )
                        .parse_mode(ParseMode::Markdown),
                    )
                    .map(|_| ())
                    .map_err(|err| Error::Help(SyncFailure::new(err)))
                    .from_err();

                return Box::new(future);
            }

            // Print the command to run
            println!("CMD: {}", cmd);

            // Execute the command, report back to the user
            return Box::new(Self::exec_cmd(state, cmd, msg).from_err());
        } else {
            Box::new(ok(()))
        }
    }
}

/// An object that tracks the status of an executed command.
/// This object also holds the status message present in a Telegram group to update when the status
/// changes, along with the global state.
pub struct ExecStatus {
    /// The actual output.
    output: String,

    /// The exit status of the process.
    /// If set, the execution has completed although it might have failed.
    /// The status code itself defines whether the execution was successful.
    status: Option<ExitStatus>,

    /// The time the command started running at.
    started_at: SystemTime,

    /// The duration it took to complete executing the command.
    completion_duration: Option<Duration>,

    /// True if the output or status has changed since the last status message update.
    /// If true, this means that the status message doesn't represent the current status corretly,
    /// and thus it should be updated.
    changed: bool,

    /// The time the Telegram status message was last changed at.
    /// When the status instance is created, this is set to the current time.
    /// This is used to manage throttling.
    changed_at: SystemTime,

    /// The number of times the status message in Telegram was updated.
    updated_count: usize,

    /// The global state to communicate through Telegram.
    state: State,

    /// The status message in a Telegram chat that should be updated to report the executing
    /// status.
    status_msg: Message,
}

impl ExecStatus {
    /// Create a status output message as reply on the given `msg`,
    /// and return an `ExecStatus` for it.
    pub fn create_status_msg(
        state: State,
        msg: &Message,
    ) -> impl Future<Item = Self, Error = Error> {
        // TODO: handle the Telegram errors properly
        state
            .telegram_send(
                msg.text_reply("<i>Executing command...</i>")
                    .parse_mode(ParseMode::Html),
            )
            .map_err(|err| -> FailureError { SyncFailure::new(err).into() })
            .map_err(|err| Error::StatusMessage(err.compat()))
            .and_then(|msg| {
                if let Some(msg) = msg {
                    ok(ExecStatus::new(state, msg))
                } else {
                    err(Error::StatusMessage(err_msg(
                    "failed to send command status message, got empty response from Telegram API",
                ).compat()))
                }
            })
    }

    /// Build a new exec status object with the given status message and the global state.
    pub fn new(state: State, status_msg: Message) -> Self {
        ExecStatus {
            output: String::new(),
            status: None,
            started_at: SystemTime::now(),
            completion_duration: None,
            changed: false,
            changed_at: SystemTime::now(),
            updated_count: 0,
            state,
            status_msg,
        }
    }

    /// Append the given `output` to the cummulative output.
    /// Note that if the output is getting too large, parts will be truncated.
    /// To append a line, use `append_line()` instead.
    pub fn append(&mut self, output: &str) {
        // Append the output
        self.output += output;

        // Truncate the beginning of the output if it became too big
        if self.truncating() {
            let truncate_at = self.output.len() - OUTPUT_TRUNCATE;
            self.output = self.output.split_off(truncate_at);
        }

        // If anything is appended, we've changed
        if !output.is_empty() {
            self.changed = true;
        }
    }

    /// Check whether the output is being truncated because it became too large.
    fn truncating(&self) -> bool {
        self.output.len() >= OUTPUT_TRUNCATE
    }

    /// Append the given `line` to the output.
    /// The given line should not include a newline character.
    /// Note that if the output is getting too large, parts will be truncated.
    pub fn append_line(&mut self, line: &str) {
        if !self.output.is_empty() {
            self.append("\n");
        }
        self.append(line);
    }

    /// Set the exit status of the executed command.
    pub fn set_status(&mut self, status: ExitStatus) {
        // Mark that the status has changed if the exit status is different
        if self.status != Some(status) {
            self.changed = true;
        }

        // Update the status, and set the completion time
        self.status = Some(status);
        self.completion_duration = self.started_at.elapsed().ok();
    }

    /// Check whether this executable has completed.
    /// It may have successfully completed or it may have failed.
    pub fn completed(&self) -> bool {
        self.status.is_some()
    }

    /// Build the status message contents, based on the current executing status.
    /// The returned status message is in HTML format.
    fn build_status_msg(&self) -> String {
        // If not completed, and there is no output yet
        if !self.completed() && self.output.is_empty() {
            return "<i>Executing command...</i>".into();
        }

        // Determine what status emoji to use
        let emoji = if !self.completed() {
            "⏳"
        } else if self.status.unwrap().success() {
            "✅"
        } else {
            "❌"
        };

        // Deterime whether to print a special notice
        let mut notice = match self.status {
            Some(status) if !status.success() => format!(
                " Exit code <code>{}</code>",
                status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or("?".into()),
            ),
            _ => String::new(),
        };

        // Add some additional status labels to the notice if relevant
        let mut status_labels = Vec::new();
        if !self.completed() && self.throttling(1) {
            status_labels.push(format!("throttling {}s", self.throttle_secs(1)));
        }
        if self.timed_out() {
            status_labels.push("timed out".into());
        }
        if self.completed() && self.completion_duration.is_some() {
            status_labels.push(format!(
                "took {}",
                self.format_duration().unwrap_or("?".into())
            ));
        }
        if self.truncating() {
            if self.completed() {
                status_labels.push("truncated".into());
            } else {
                status_labels.push("truncating".into());
            }
        }
        if !status_labels.is_empty() {
            notice += &format!(" ({})", status_labels.join(", "));
        }

        // Format the output
        let output = if self.output.is_empty() {
            "<i>No output</i>".to_owned()
        } else {
            format!(
                "\
                 <b>Output:</b>\n\
                 <code>{}{}</code>\
                 ",
                if self.truncating() {
                    "[truncated] "
                } else {
                    ""
                },
                encode_minimal(&self.output),
            )
        };

        // Format the message
        format!(
            "\
             {}\n\
             \n\
             {}  {}\
             ",
            output, emoji, notice,
        )
    }

    /// Update the status message in Telegram with the newest status data in this object.
    /// This method spawns a future that completes asynchronously.
    // TODO: should we return a future for updating, to allow catching errors?
    pub fn update_status_msg(&mut self) {
        // Spawn a future to edit the status message with the newest build status text
        self.state.telegram_spawn(
            self.status_msg
                .edit_text(self.build_status_msg())
                .parse_mode(ParseMode::Html),
        );

        // Reset the changed status
        self.changed = false;
        self.updated_count += 1;
        self.changed_at = SystemTime::now();
    }

    /// Update the status message in Telegram with the newest status data in this object,
    /// if it has been changed after the last update.
    pub fn update(&mut self) {
        // Only if something changed
        if !self.changed {
            return;
        }

        // Actually update
        self.update_status_msg()
    }

    /// Update the status message in Telegram with the newest status data in this object,
    /// if it has been changed after the last update.
    ///
    /// This method won't update if it was invoked too quickly before the last change.
    pub fn update_throttled(&mut self) {
        // Throttle
        match self.changed_at.elapsed() {
            Ok(elapsed) if elapsed < self.throttle_duration() => return,
            Err(..) => return,
            _ => {}
        }

        // Update
        self.update()
    }

    /// Check whehter we're throttling output.
    ///
    /// An update count offset may be given.
    fn throttling(&self, offset: i64) -> bool {
        self.throttle_secs(offset) > 1
    }

    /// The time to wait in seconds while throttling before sending the next update to Telegram.
    /// The throttle time gradually increases the more messages updates are sent, to prevent
    /// hitting the rate limit enforced by Telegram for sending message updates.
    ///
    /// An update count offset may be given.
    fn throttle_secs(&self, offset: i64) -> u64 {
        // Get the update count
        let count = self.updated_count as i64 + offset;

        // TODO: make the throttle time configurable
        if count < 2 {
            1
        } else if count < 5 {
            3
        } else if count < 8 {
            5
        } else {
            10
        }
    }

    /// The time to wait while throttling before sending the next update to Telegram.
    /// The throttle time gradually increases the more messages updates are sent, to prevent
    /// hitting the rate limit enforced by Telegram for sending message updates.
    fn throttle_duration(&self) -> Duration {
        Duration::from_secs(self.throttle_secs(0)) - Duration::from_millis(50)
    }

    /// Check if the user command timed out.
    /// If the command hasn't completed yet, `false` is returned.
    fn timed_out(&self) -> bool {
        // We must have a status code of 124
        match self.status {
            Some(status) if status.code() == Some(124) => {}
            _ => return false,
        }

        // If a duration is known, it must reach the timeout time
        match self.completion_duration {
            Some(duration) if duration >= EXEC_TIMEOUT - EXEC_TIMEOUT_PRECISION => {}
            Some(_) => return false,
            _ => {}
        }

        // The conditions for being timed out have been met
        true
    }

    /// Format the completion duration, if known, into a human readable format.
    /// If the completion time is not known, `None` is returned.
    fn format_duration(&self) -> Option<String> {
        match self.completion_duration {
            Some(duration) if duration.as_secs() >= 1 => {
                Some(format_duration(Duration::from_secs(duration.as_secs())).to_string())
            }
            Some(duration) => Some(
                format_duration(duration)
                    .to_string()
                    .splitn(2, ' ')
                    .next()
                    .unwrap()
                    .into(),
            ),
            None => None,
        }
    }
}

/// An exec action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending the help message which is sent when no command input is
    /// given.
    #[fail(display = "failed to send help response message")]
    Help(#[cause] SyncFailure<TelegramError>),

    /// Failed to send the initial status message to update later on as the process continues.
    #[fail(display = "failed to send command status message")]
    StatusMessage(#[cause] Compat<FailureError>),

    /// An error occurred while executing the user command.
    #[fail(display = "failed to execute user shell command")]
    Execute(#[cause] ExecutorError),

    /// An error occurred while throttling status update messages.
    #[fail(display = "failed to throttle status update messages")]
    Throttle(#[cause] TimerError),
}
