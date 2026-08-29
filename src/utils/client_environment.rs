use std::fmt;
use std::fs;
use std::io;
use std::os::unix::net::UnixStream;
use std::str;

use niri_config::utils::RegexEq;
use smithay::reexports::rustix;

/// Environment captured from the process that opened a Wayland client connection.
///
/// The entries can contain credentials, so this type deliberately exposes only matching and a
/// redacted `Debug` implementation.
pub struct ClientEnvironment {
    entries: Vec<String>,
}

impl ClientEnvironment {
    pub fn capture(socket: &UnixStream) -> io::Result<Self> {
        let credentials = rustix::net::sockopt::socket_peercred(socket)?;
        let pid = rustix::process::Pid::as_raw(Some(credentials.pid));
        let bytes = fs::read(format!("/proc/{pid}/environ"))?;
        Ok(Self::from_bytes(&bytes))
    }

    pub fn matches(&self, pattern: &RegexEq) -> bool {
        self.entries
            .iter()
            .any(|entry| pattern.0.is_match(entry.as_str()))
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let entries = bytes
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty())
            .filter_map(|entry| str::from_utf8(entry).ok())
            .map(str::to_owned)
            .collect();

        Self { entries }
    }
}

impl fmt::Debug for ClientEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientEnvironment")
            .field("entry_count", &self.entries.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::io::{ErrorKind, Read as _};
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;

    const HELPER_SOCKET_ENV: &str = "NIRI_CLIENT_ENV_HELPER_SOCKET";
    const TEST_MARKER: &str = "NIRI_CLIENT_ENV_TEST_MARKER=peer-process";

    struct SocketFile(PathBuf);

    impl Drop for SocketFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn captures_peer_process_environment() {
        let socket_path = std::env::temp_dir().join(format!(
            "niri-client-environment-test-{}.sock",
            std::process::id()
        ));
        let socket_file = SocketFile(socket_path.clone());
        let listener = UnixListener::bind(&socket_path).unwrap();
        listener.set_nonblocking(true).unwrap();

        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "utils::client_environment::tests::capture_helper_process",
            ])
            .env_clear()
            .env(HELPER_SOCKET_ENV, &socket_path)
            .env("NIRI_CLIENT_ENV_TEST_MARKER", "peer-process")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(2);
        let socket = loop {
            match listener.accept() {
                Ok((socket, _)) => break socket,
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    if let Some(status) = child.try_wait().unwrap() {
                        panic!("client environment helper exited early: {status}");
                    }
                    if Instant::now() >= deadline {
                        child.kill().unwrap();
                        child.wait().unwrap();
                        panic!("timed out waiting for client environment helper");
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) => panic!("error accepting helper connection: {err}"),
            }
        };

        let environment = ClientEnvironment::capture(&socket).unwrap();
        drop(socket);
        drop(listener);
        drop(socket_file);
        assert!(child.wait().unwrap().success());

        assert!(environment.matches(&format!("^{TEST_MARKER}$").parse::<RegexEq>().unwrap()));
    }

    #[test]
    #[ignore]
    fn capture_helper_process() {
        let socket_path = std::env::var_os(HELPER_SOCKET_ENV).unwrap();
        let mut socket = UnixStream::connect(socket_path).unwrap();
        let mut closed = [0];
        let _ = socket.read(&mut closed);
    }

    #[test]
    fn matches_individual_environment_entries() {
        let environment = ClientEnvironment::from_bytes(b"CONTEXT=agent\0OTHER=value\0");

        assert!(environment.matches(&"^CONTEXT=agent$".parse::<RegexEq>().unwrap()));
        assert!(!environment.matches(&"agentOTHER".parse::<RegexEq>().unwrap()));
    }

    #[test]
    fn debug_output_does_not_expose_environment() {
        let environment = ClientEnvironment::from_bytes(b"SECRET=do-not-log\0");
        let debug = format!("{environment:?}");

        assert!(!debug.contains("SECRET"));
        assert!(!debug.contains("do-not-log"));
    }
}
