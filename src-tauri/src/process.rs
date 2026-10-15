use std::io::{self, Read};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Runs `command` and collects its output, killing it when it takes longer than `limit`: a
/// stuck tool (a Docker engine that never answers, say) must not freeze the app.
pub fn output_within(command: &mut Command, limit: Duration) -> io::Result<Output> {
    let mut child = command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    // Both pipes are drained while the process runs; a full pipe would block it forever.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let status = wait_until(&mut child, Instant::now() + limit)?;
    Ok(Output { status, stdout: join(stdout), stderr: join(stderr) })
}

fn wait_until(child: &mut Child, deadline: Instant) -> io::Result<std::process::ExitStatus> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            // Already exiting or gone is fine: either way it is not running any more.
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(io::ErrorKind::TimedOut, "o comando passou do tempo limite e foi interrompido"));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            // A read error only truncates what is shown in the report.
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    })
}

fn join(reader: thread::JoinHandle<Vec<u8>>) -> Vec<u8> {
    reader.join().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_the_output_of_a_quick_command() {
        let output = output_within(Command::new("/bin/echo").arg("pronto"), Duration::from_secs(5)).expect("run");
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "pronto");
    }

    #[test]
    fn stops_a_command_that_takes_too_long() {
        let started = Instant::now();
        let result = output_within(Command::new("/bin/sleep").arg("10"), Duration::from_millis(200));

        assert_eq!(result.map_err(|error| error.kind()).unwrap_err(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn does_not_block_on_large_output() {
        let script = "i=0; while [ $i -lt 20000 ]; do echo 'linha de saída bem comprida para encher o pipe'; i=$((i+1)); done";
        let output = output_within(Command::new("/bin/sh").args(["-c", script]), Duration::from_secs(20)).expect("run");
        assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 20_000);
    }
}
