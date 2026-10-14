//! Single-instance mode: a second `ettin-edit file.txt` hands the file to the
//! window that is already open instead of starting another one.
//!
//! The first instance listens on a loopback TCP port and writes the port and
//! a random token to `instance.json` in the user's config folder. Later
//! instances read that file, send their file list with the token, and exit.
//! Only processes that can read the user's config folder know the token.

use std::collections::hash_map::RandomState;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use serde::{Deserialize, Serialize};

const TIMEOUT: Duration = Duration::from_millis(800);
/// Longest message accepted from another instance.
const MAX_MESSAGE: u64 = 1 << 20;

#[derive(Serialize, Deserialize)]
struct InstanceFile {
    port: u16,
    token: String,
    pid: u32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    token: String,
    files: Vec<PathBuf>,
}

/// The running instance's side: files sent by later instances arrive here.
pub struct Listener {
    pub files: Receiver<Vec<PathBuf>>,
    file: PathBuf,
}

fn instance_path(config_dir: &Path) -> PathBuf {
    config_dir.join("instance.json")
}

fn random_token() -> String {
    // RandomState is seeded from the OS's random source.
    (0..2)
        .map(|i| {
            let mut h = RandomState::new().build_hasher();
            h.write_u32(std::process::id());
            h.write_u8(i);
            format!("{:016x}", h.finish())
        })
        .collect()
}

fn running_addr(config_dir: &Path) -> Option<(SocketAddr, InstanceFile)> {
    let text = fs::read_to_string(instance_path(config_dir)).ok()?;
    let info = serde_json::from_str::<InstanceFile>(&text).ok()?;
    Some((SocketAddr::from((Ipv4Addr::LOCALHOST, info.port)), info))
}

/// True if another Ettin Edit is running and listening.
pub fn is_running(config_dir: &Path) -> bool {
    running_addr(config_dir)
        .is_some_and(|(addr, _)| TcpStream::connect_timeout(&addr, TIMEOUT).is_ok())
}

/// Sends `files` to a running instance. Returns true if one took them.
pub fn send_to_running(config_dir: &Path, files: &[PathBuf]) -> bool {
    let Some((addr, info)) = running_addr(config_dir) else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, TIMEOUT) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(TIMEOUT));
    let _ = stream.set_write_timeout(Some(TIMEOUT));
    let message = Message {
        token: info.token,
        files: files
            .iter()
            .map(|p| std::path::absolute(p).unwrap_or_else(|_| p.clone()))
            .collect(),
    };
    let Ok(mut line) = serde_json::to_string(&message) else {
        return false;
    };
    line.push('\n');
    if stream.write_all(line.as_bytes()).is_err() {
        return false;
    }
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).is_ok() && reply.trim() == "ok"
}

/// Starts listening for later instances. `wake` is called after files arrive.
pub fn listen(config_dir: &Path, wake: impl Fn() + Send + 'static) -> Option<Listener> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).ok()?;
    let port = listener.local_addr().ok()?.port();
    let token = random_token();
    let info = InstanceFile {
        port,
        token: token.clone(),
        pid: std::process::id(),
    };
    let file = instance_path(config_dir);
    fs::create_dir_all(config_dir).ok()?;
    write_private(&file, serde_json::to_string(&info).ok()?.as_bytes()).ok()?;

    let (tx, rx) = channel();
    std::thread::Builder::new()
        .name("ettin-edit-instance".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                handle(stream, &token, &tx);
                wake();
            }
        })
        .ok()?;
    Some(Listener { files: rx, file })
}

fn handle(stream: TcpStream, token: &str, tx: &Sender<Vec<PathBuf>>) {
    let _ = stream.set_read_timeout(Some(TIMEOUT));
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let mut line = String::new();
    if BufReader::new(stream.take(MAX_MESSAGE))
        .read_line(&mut line)
        .is_err()
    {
        return;
    }
    let Ok(message) = serde_json::from_str::<Message>(&line) else {
        return;
    };
    if message.token != token {
        return;
    }
    let _ = tx.send(message.files);
    let _ = writer.write_all(b"ok\n");
}

/// Readable by the current user only, where the platform supports it.
fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(data)
}

impl Drop for Listener {
    fn drop(&mut self) {
        // Remove the file only if it still points at this instance.
        let ours = fs::read_to_string(&self.file)
            .ok()
            .and_then(|t| serde_json::from_str::<InstanceFile>(&t).ok())
            .is_some_and(|i| i.pid == std::process::id());
        if ours {
            let _ = fs::remove_file(&self.file);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_instance_hands_over_files() {
        let dir = std::env::temp_dir().join(format!("ettin-edit-instance-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert!(!send_to_running(&dir, &[]));

        let listener = listen(&dir, || {}).unwrap();
        // An absolute path on every platform, so it arrives unchanged.
        let file = dir.join("a.txt");
        assert!(send_to_running(&dir, std::slice::from_ref(&file)));
        assert_eq!(listener.files.recv_timeout(TIMEOUT).unwrap(), vec![file]);

        // A wrong token is ignored.
        let text = fs::read_to_string(instance_path(&dir)).unwrap();
        let mut info: InstanceFile = serde_json::from_str(&text).unwrap();
        info.token = "nope".into();
        fs::write(instance_path(&dir), serde_json::to_string(&info).unwrap()).unwrap();
        assert!(!send_to_running(&dir, &[]));
        assert!(listener.files.try_recv().is_err());

        drop(listener);
        fs::remove_dir_all(&dir).ok();
    }
}
