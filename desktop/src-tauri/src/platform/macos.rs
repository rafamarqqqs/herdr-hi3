use fs2::FileExt;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    os::{
        fd::AsRawFd,
        unix::{fs::PermissionsExt, net::UnixStream, process::CommandExt},
    },
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

const LIMIT: u64 = 4 * 1024 * 1024;
#[derive(Serialize)]
pub struct Frame {
    pub bytes: String,
    pub closed: bool,
    pub reason: Option<String>,
}
#[derive(Serialize)]
pub struct SessionInfo {
    pub terminal_id: String,
    pub data_dir: String,
}
pub struct Runtime {
    root: PathBuf,
    binary: PathBuf,
    _lock: File,
}
pub struct Connection {
    child: Child,
    stdin: Option<ChildStdin>,
    frames: Receiver<Frame>,
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn paste_input(text: &str) -> Result<String, String> {
    if text.contains(['\x1b', '\0']) {
        return Err("Paste contains unsupported control characters".into());
    }
    // Herdr recognizes this complete envelope and applies the live shell's paste mode.
    Ok(format!("\x1b[200~{text}\x1b[201~"))
}
fn write_bounded(writer: &mut impl Write, mut bytes: &[u8]) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_millis(500);
    while !bytes.is_empty() {
        match writer.write(bytes) {
            Ok(0) => return Err("Controller input closed".into()),
            Ok(n) => bytes = &bytes[n..],
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(e) => return Err(err(e)),
        }
        if Instant::now() >= deadline {
            return Err("Controller input stalled. Reconnect before continuing.".into());
        }
    }
    Ok(())
}
pub fn validate_size(cols: u16, rows: u16) -> Result<(), String> {
    if !(2..=400).contains(&cols) || !(2..=200).contains(&rows) {
        return Err("Terminal dimensions must be 2–400 columns and 2–200 rows".into());
    }
    Ok(())
}
fn private_dir(path: &Path) -> std::io::Result<()> {
    if path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err(std::io::Error::other(
            "Refusing symlink for private runtime directory",
        ));
    }
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}
impl Runtime {
    pub fn new(root: PathBuf, binary: PathBuf) -> std::io::Result<Self> {
        if !binary.is_file() {
            return Err(std::io::Error::other("Bundled Herdr runtime is missing"));
        }
        private_dir(&root)?;
        if root
            .join("runtime/herdr-client.sock")
            .as_os_str()
            .as_encoded_bytes()
            .len()
            > 100
        {
            return Err(std::io::Error::other("Runtime socket path is too long"));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("gui.lock"))?;
        lock.try_lock_exclusive()
            .map_err(|_| std::io::Error::other("hi3 is already open"))?;
        for name in ["config", "state", "data", "runtime", "shell", "work"] {
            private_dir(&root.join(name))?;
        }
        let config = root.join("config/config.toml");
        if !config.exists() {
            OpenOptions::new().write(true).create_new(true).open(config)?.write_all(b"onboarding = false\n[theme]\nname = \"gruvbox\"\nauto_switch = false\n[terminal]\ndefault_shell = \"/bin/zsh\"\nshell_mode = \"non_login\"\n[session]\nresume_agents_on_restore = false\n[update]\nversion_check = false\nmanifest_check = false\n")?;
        }
        Ok(Self {
            root,
            binary,
            _lock: lock,
        })
    }
    fn socket(&self) -> PathBuf {
        self.root.join("runtime/herdr.sock")
    }
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.binary);
        command.env_clear();
        for key in [
            "HOME", "USER", "LOGNAME", "PATH", "LANG", "LC_CTYPE", "TMPDIR",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .env("SHELL", "/bin/zsh")
            .env("HERDR_SOCKET_PATH", self.socket())
            .env("HERDR_CONFIG_PATH", self.root.join("config/config.toml"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("XDG_RUNTIME_DIR", self.root.join("runtime"))
            .env("ZDOTDIR", self.root.join("shell"))
            .env("ENV", "/dev/null")
            .env("BASH_ENV", "/dev/null")
            .env("HERDR_DISABLE_SOUND", "1")
            .current_dir(self.root.join("work"));
        command
    }
    pub fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let mut stream = UnixStream::connect(self.socket()).map_err(err)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(err)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .map_err(err)?;
        writeln!(
            stream,
            "{}",
            json!({"id":"hi3","method":method,"params":params})
        )
        .map_err(err)?;
        let mut line = String::new();
        BufReader::new(stream)
            .take(LIMIT + 1)
            .read_line(&mut line)
            .map_err(err)?;
        if line.len() as u64 > LIMIT {
            return Err("API response exceeds size limit".into());
        }
        let response: Value = serde_json::from_str(&line).map_err(err)?;
        if let Some(error) = response.get("error") {
            return Err(format!("Herdr: {error}"));
        }
        response
            .get("result")
            .cloned()
            .ok_or("Missing API result".into())
    }
    fn ensure_server(&self) -> Result<(), String> {
        if self.request("ping", json!({})).is_ok() {
            return Ok(());
        }
        // Never silently replace an unresponsive server or fall back to a default socket.
        if self.socket().exists() {
            return Err("The isolated hi3 server is unavailable. Existing socket preserved; no other server was contacted.".into());
        }
        let mut command = self.command();
        command
            .arg("server")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // The server must outlive this GUI and its controlling terminal.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().map_err(err)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if self.request("ping", json!({})).is_ok() {
                thread::spawn(move || {
                    let _ = child.wait();
                });
                return Ok(());
            }
            if let Some(status) = child.try_wait().map_err(err)? {
                return Err(format!("Isolated server exited: {status}"));
            }
            thread::sleep(Duration::from_millis(80));
        }
        // Do not kill a server which may have started work despite a failed handshake.
        thread::spawn(move || {
            let _ = child.wait();
        });
        Err("Timed out starting the isolated server; no existing session was changed".into())
    }
    fn terminal(&self) -> Result<String, String> {
        let workspaces = self.request("workspace.list", json!({}))?;
        if let Some(spaces) = workspaces.get("workspaces").and_then(Value::as_array) {
            for space in spaces {
                if space.get("label").and_then(Value::as_str) != Some("hi3") {
                    continue;
                }
                let panes =
                    self.request("pane.list", json!({"workspace_id":space["workspace_id"]}))?;
                if let Some(id) = panes
                    .get("panes")
                    .and_then(Value::as_array)
                    .and_then(|p| p.first())
                    .and_then(|p| p.get("terminal_id"))
                    .and_then(Value::as_str)
                {
                    return Ok(id.into());
                }
            }
        }
        let created = self.request(
            "workspace.create",
            json!({"label":"hi3","cwd":self.root.join("work"),"focus":false}),
        )?;
        created["root_pane"]["terminal_id"]
            .as_str()
            .map(String::from)
            .ok_or("Created workspace has no terminal".into())
    }
    pub fn connect(&self, cols: u16, rows: u16) -> Result<(Connection, SessionInfo), String> {
        self.connect_to(cols, rows, None)
    }
    pub fn initialize(&self) -> Result<SessionInfo, String> {
        self.ensure_server()?;
        Ok(SessionInfo {
            terminal_id: self.terminal()?,
            data_dir: self.root.display().to_string(),
        })
    }
    pub fn navigation(&self) -> Result<Value, String> {
        self.request("session.snapshot", json!({}))?
            .get("snapshot")
            .cloned()
            .ok_or("Missing session snapshot".into())
    }
    pub fn connect_to(
        &self,
        cols: u16,
        rows: u16,
        target: Option<String>,
    ) -> Result<(Connection, SessionInfo), String> {
        validate_size(cols, rows)?;
        self.ensure_server()?;
        let terminal_id = match target {
            Some(id) => {
                let panes = self.request("pane.list", json!({}))?;
                if !panes["panes"]
                    .as_array()
                    .is_some_and(|panes| panes.iter().any(|pane| pane["terminal_id"] == id))
                {
                    return Err("Terminal no longer exists in this hi3 server".into());
                }
                id
            }
            None => self.terminal()?,
        };
        let mut child = self
            .command()
            .args([
                "terminal",
                "session",
                "control",
                &terminal_id,
                "--cols",
                &cols.to_string(),
                "--rows",
                &rows.to_string(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(err)?;
        let stdin = child.stdin.take().ok_or("Missing control input")?;
        // Bound pipe writes so a stalled controller cannot hold a command indefinitely.
        let fd = stdin.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            let _ = child.kill();
            let _ = child.wait();
            return Err(err(std::io::Error::last_os_error()));
        }
        let stdout = child.stdout.take().ok_or("Missing control output")?;
        let (tx, rx) = mpsc::sync_channel(4);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                let result = reader.by_ref().take(LIMIT + 1).read_line(&mut line);
                let frame = match result {
                    Ok(0) => Frame {
                        bytes: String::new(),
                        closed: true,
                        reason: Some("Terminal controller ended. Reconnect to resume.".into()),
                    },
                    Ok(_) if line.len() as u64 <= LIMIT => match parse_frame(&line) {
                        Ok(frame) => frame,
                        Err(e) => Frame {
                            bytes: String::new(),
                            closed: true,
                            reason: Some(e),
                        },
                    },
                    _ => Frame {
                        bytes: String::new(),
                        closed: true,
                        reason: Some("Invalid or oversized terminal frame".into()),
                    },
                };
                let closed = frame.closed;
                if tx.send(frame).is_err() || closed {
                    break;
                }
            }
        });
        Ok((
            Connection {
                child,
                stdin: Some(stdin),
                frames: rx,
            },
            SessionInfo {
                terminal_id,
                data_dir: self.root.display().to_string(),
            },
        ))
    }
}
fn parse_frame(line: &str) -> Result<Frame, String> {
    let value: Value = serde_json::from_str(line).map_err(err)?;
    match value["type"].as_str() {
        Some("terminal.frame") if value["encoding"] == "ansi" => Ok(Frame {
            bytes: value["bytes"].as_str().ok_or("Missing frame bytes")?.into(),
            closed: false,
            reason: None,
        }),
        Some("terminal.closed") => Ok(Frame {
            bytes: String::new(),
            closed: true,
            reason: value["reason"].as_str().map(String::from),
        }),
        _ => Err("Unsupported terminal stream message".into()),
    }
}
impl Connection {
    pub fn send(&mut self, value: Value) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("Controller is closed")?;
        write_bounded(stdin, format!("{value}\n").as_bytes())
    }
    pub fn read_frame(&mut self) -> Result<Option<Frame>, String> {
        match self.frames.try_recv() {
            Ok(frame) => Ok(Some(frame)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("Terminal controller disconnected".into()),
        }
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        // EOF releases terminal control. Killing this client never targets the server or shell.
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stalled_input_is_bounded() {
        let (mut writer, _reader) = UnixStream::pair().unwrap();
        writer.set_nonblocking(true).unwrap();
        let start = Instant::now();
        assert!(write_bounded(&mut writer, &vec![b'x'; 2 * 1024 * 1024]).is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn paste_envelope_rejects_escape_injection() {
        assert_eq!(
            paste_input("hello\nworld").unwrap(),
            "\x1b[200~hello\nworld\x1b[201~"
        );
        assert!(paste_input("\x1b[201~bad").is_err());
    }
    struct TestServer<'a>(&'a Runtime);
    impl Drop for TestServer<'_> {
        fn drop(&mut self) {
            // Only instantiated with a newly created test TempDir, never app data.
            let _ = self.0.request("server.stop", json!({}));
        }
    }
    fn wait_file(path: &Path) -> String {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if let Ok(text) = fs::read_to_string(path) {
                if !text.is_empty() {
                    return text;
                }
            }
            thread::sleep(Duration::from_millis(30));
        }
        panic!("test shell did not write {}", path.display());
    }
    #[test]
    #[ignore = "starts its own temporary Herdr server; requires HI3_TEST_RUNTIME_BIN"]
    fn isolated_runtime_lifecycle() {
        use base64::Engine;
        let binary =
            PathBuf::from(std::env::var("HI3_TEST_RUNTIME_BIN").expect("explicit test binary"));
        let temp = tempfile::Builder::new()
            .prefix("hi3-test-")
            .tempdir_in("/tmp")
            .unwrap();
        let runtime = Runtime::new(temp.path().join("app"), binary).unwrap();
        let _server = TestServer(&runtime);
        let (mut connection, first) = runtime.connect(80, 24).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut rendered = false;
        while Instant::now() < deadline {
            if let Some(frame) = connection.read_frame().unwrap() {
                assert!(!frame.closed, "{:?}", frame.reason);
                let ansi = base64::engine::general_purpose::STANDARD
                    .decode(frame.bytes)
                    .unwrap();
                if !ansi.is_empty() {
                    rendered = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(rendered, "controller must render a terminal frame");
        // Do not mistake a rendered prompt or successful file write for working output.
        // Split the marker in the command so echoed input cannot satisfy this assertion.
        connection
            .send(json!({"type":"terminal.input","text":"printf '\\110\\111\\063_OUTPUT_OK\\n'\r"}))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut output = String::new();
        while Instant::now() < deadline && !output.contains("HI3_OUTPUT_OK") {
            if let Some(frame) = connection.read_frame().unwrap() {
                assert!(!frame.closed, "{:?}", frame.reason);
                let ansi = base64::engine::general_purpose::STANDARD
                    .decode(frame.bytes)
                    .unwrap();
                output.push_str(&String::from_utf8_lossy(&ansi));
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(
            output.contains("HI3_OUTPUT_OK"),
            "command output must reach the GUI stream"
        );
        connection.send(json!({"type":"terminal.input","text":"printf 'ação 日本語' > unicode-proof; echo $$ > first-pid; sleep 1; printf finished > background-proof\r"})).unwrap();
        let work = runtime.root.join("work");
        let first_pid = wait_file(&work.join("first-pid"));
        drop(connection);
        assert_eq!(wait_file(&work.join("background-proof")), "finished");
        assert_eq!(wait_file(&work.join("unicode-proof")), "ação 日本語");
        assert!(runtime.request("ping", json!({})).is_ok());
        let (mut connection, second) = runtime.connect(100, 30).unwrap();
        assert_eq!(first.terminal_id, second.terminal_id);
        connection
            .send(json!({"type":"terminal.resize","cols":100,"rows":30}))
            .unwrap();
        connection.send(json!({"type":"terminal.input","text":"echo $$ > second-pid; stty size > size-proof\r"})).unwrap();
        assert_eq!(wait_file(&work.join("second-pid")), first_pid);
        assert_eq!(wait_file(&work.join("size-proof")).trim(), "30 100");
        connection.send(json!({"type":"terminal.input","text":paste_input("printf pasted > paste-proof\n").unwrap()})).unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(
            !work.join("paste-proof").exists(),
            "paste must not submit shell commands by itself"
        );
        connection
            .send(json!({"type":"terminal.input","text":"\r"}))
            .unwrap();
        assert_eq!(wait_file(&work.join("paste-proof")), "pasted");
        let snapshot = runtime.navigation().unwrap();
        let pane = snapshot["panes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pane| pane["terminal_id"] == first.terminal_id)
            .unwrap();
        let workspace_id = pane["workspace_id"].clone();
        assert!(
            snapshot["agents"].as_array().unwrap().is_empty(),
            "ordinary shell must not appear in Agents"
        );
        let created = runtime
            .request(
                "tab.create",
                json!({"workspace_id":workspace_id,"label":"second","focus":false}),
            )
            .unwrap();
        let target = created["root_pane"]["terminal_id"]
            .as_str()
            .unwrap()
            .to_owned();
        drop(connection);
        let (mut other, _) = runtime.connect_to(80, 24, Some(target)).unwrap();
        other
            .send(json!({"type":"terminal.input","text":"echo $$ > other-pid\r"}))
            .unwrap();
        assert_ne!(wait_file(&work.join("other-pid")), first_pid);
        drop(other);
        assert!(runtime
            .connect_to(80, 24, Some("missing-terminal".into()))
            .is_err());
        assert_eq!(
            runtime.navigation().unwrap()["workspaces"]
                .as_array()
                .unwrap()
                .len(),
            snapshot["workspaces"].as_array().unwrap().len(),
            "invalid selection must not create a workspace"
        );
        let (mut connection, _) = runtime.connect_to(80, 24, Some(first.terminal_id)).unwrap();
        connection
            .send(json!({"type":"terminal.input","text":"echo $$ > return-pid\r"}))
            .unwrap();
        assert_eq!(
            wait_file(&work.join("return-pid")),
            first_pid,
            "tab round trip must preserve the shell process"
        );
        let snapshot = runtime.navigation().unwrap();
        let source = snapshot["panes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pane| pane["terminal_id"] == second.terminal_id)
            .unwrap();
        let split=runtime.request("pane.split",json!({"target_pane_id":source["pane_id"],"direction":"right","ratio":0.5,"cwd":work,"focus":false})).unwrap();
        let target = split["pane"]["terminal_id"].as_str().unwrap().to_owned();
        let (mut sibling, _) = runtime.connect_to(40, 20, Some(target)).unwrap();
        connection
            .send(json!({"type":"terminal.resize","cols":60,"rows":25}))
            .unwrap();
        connection.send(json!({"type":"terminal.input","text":"printf first > lane-a; stty size > size-a\r"})).unwrap();
        sibling.send(json!({"type":"terminal.input","text":"printf second > lane-b; stty size > size-b\r"})).unwrap();
        assert_eq!(wait_file(&work.join("lane-a")), "first");
        assert_eq!(wait_file(&work.join("lane-b")), "second");
        assert_eq!(wait_file(&work.join("size-a")).trim(), "25 60");
        assert_eq!(wait_file(&work.join("size-b")).trim(), "20 40");
        sibling
            .send(json!({"type":"terminal.input","text":"echo $$ > sibling-before\r"}))
            .unwrap();
        let sibling_pid = wait_file(&work.join("sibling-before"));
        runtime
            .request(
                "pane.resize",
                json!({"pane_id":source["pane_id"],"direction":"right","amount":0.05}),
            )
            .unwrap();
        runtime.request("pane.reposition", json!({"pane_id":split["pane"]["pane_id"],"target_pane_id":source["pane_id"],"direction":"down"})).unwrap();
        let moved = runtime.navigation().unwrap();
        for original in [source, &split["pane"]] {
            assert!(moved["panes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|pane| pane["pane_id"] == original["pane_id"]
                    && pane["terminal_id"] == original["terminal_id"]));
        }
        sibling
            .send(json!({"type":"terminal.input","text":"echo $$ > sibling-after\r"}))
            .unwrap();
        assert_eq!(wait_file(&work.join("sibling-after")), sibling_pid);
        drop(sibling);
        connection
            .send(json!({"type":"terminal.input","text":"echo $$ > survivor-pid\r"}))
            .unwrap();
        assert_eq!(wait_file(&work.join("survivor-pid")), first_pid);
        connection
            .send(json!({"type":"terminal.scroll","direction":"up","lines":3}))
            .unwrap();
        drop(connection);
    }
    #[test]
    fn dimensions_are_bounded() {
        assert!(validate_size(80, 24).is_ok());
        assert!(validate_size(0, 24).is_err());
        assert!(validate_size(401, 24).is_err());
    }
    #[test]
    fn frames_validate_protocol() {
        assert!(
            !parse_frame(r#"{"type":"terminal.frame","encoding":"ansi","bytes":"YQ=="}"#)
                .unwrap()
                .closed
        );
        assert!(parse_frame(r#"{"type":"unknown"}"#).is_err());
    }
    #[test]
    fn environment_is_explicit_and_does_not_inherit_session_or_credentials() {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join("herdr");
        fs::write(&binary, "").unwrap();
        let runtime = Runtime::new(temp.path().join("hi3"), binary).unwrap();
        let command = runtime.command();
        let env: std::collections::HashMap<_, _> = command
            .get_envs()
            .filter_map(|(k, v)| {
                v.map(|v| {
                    (
                        k.to_string_lossy().into_owned(),
                        v.to_string_lossy().into_owned(),
                    )
                })
            })
            .collect();
        assert_eq!(
            env["HERDR_SOCKET_PATH"],
            runtime.socket().display().to_string()
        );
        for key in [
            "HERDR_ENV",
            "HERDR_SESSION",
            "HERDR_STARTUP_CWD",
            "HERDR_CLIENT_SOCKET_PATH",
            "GH_TOKEN",
            "GITHUB_TOKEN",
        ] {
            assert!(!env.contains_key(key));
        }
        assert!(Runtime::new(runtime.root.clone(), runtime.binary.clone()).is_err());
    }
}
