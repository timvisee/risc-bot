use std::process::{ExitStatus, Stdio};

use futures::{future, prelude::*};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::wrappers::LinesStream;

use super::Error;

/// Execute the given command.
///
/// `stdout` and `stderr` is streamed line by line to the `output` closure,
/// which is called for each line that received.
pub async fn execute<O>(cmd: &mut Command, output: O) -> Result<ExitStatus, Error>
where
    O: Fn(String) -> Result<(), Error> + Clone + 'static,
{
    // Spawn a child process to run the given command in
    // TODO: configurable timeout
    let process = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();

    // Return errors
    let mut process = match process {
        Ok(process) => process,
        Err(e) => return Err(Error::Spawn(e)),
    };

    // Build process output streams, process each line with the output closure
    let stdout_reader = BufReader::new(process.stdout.take().unwrap());
    let stderr_reader = BufReader::new(process.stderr.take().unwrap());
    let stdout_handler = output.clone();
    let stderr_handler = output.clone();
    let stdout_stream = LinesStream::new(stdout_reader.lines())
        .map_err(Error::CollectOutput)
        .try_for_each(|output| {
            stdout_handler(output).expect("failed to handle stdout of process");
            future::ok(())
        });
    let stderr_stream = LinesStream::new(stderr_reader.lines())
        .map_err(Error::CollectOutput)
        .try_for_each(|output| {
            stderr_handler(output).expect("failed to handle stderr of process");
            future::ok(())
        });

    // Wait for the child process to exit, catch the status code
    let process_exit = process
        .wait_with_output()
        .map_ok(|output| output.status)
        .map_err(Error::Complete);

    // Wait on the output streams and on a status code
    future::try_join3(process_exit, stdout_stream, stderr_stream)
        .await
        .map(|(status, _, _)| status)
}
