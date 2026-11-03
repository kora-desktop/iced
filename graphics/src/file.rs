//! Read the files a picture names.
use std::fs;
use std::io;
use std::path::Path;

/// Fails unless `path` names a regular file, following links.
///
/// Pictures are read on the thread that draws, so anything else must be turned away before it
/// is opened: opening a FIFO waits for a writer and a device such as `/dev/zero` never ends.
pub fn regular(path: &Path) -> io::Result<()> {
    if fs::metadata(path)?.is_file() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ))
    }
}

/// The contents of `path` as text, when it names a regular file.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    regular(path)?;
    fs::read_to_string(path)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn a_fifo_is_turned_away_without_waiting_for_a_writer() {
        let dir = std::env::temp_dir().join(format!("iced-file-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let fifo = dir.join("picture.svg");
        let _ = fs::remove_file(&fifo);
        assert!(
            Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );

        let (sender, receiver) = mpsc::channel();
        let path = fifo.clone();
        let _reader = std::thread::spawn(move || sender.send(read_to_string(&path).is_err()));

        let refused = receiver.recv_timeout(Duration::from_secs(5));
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(
            refused,
            Ok(true),
            "reading a FIFO must not wait for a writer"
        );
    }

    #[test]
    fn devices_and_folders_are_not_regular_files() {
        assert!(regular(Path::new("/dev/zero")).is_err());
        assert!(regular(Path::new("/")).is_err());
        assert!(regular(Path::new("/nonexistent/picture.svg")).is_err());
        assert!(read_to_string(Path::new("/dev/zero")).is_err());
    }

    #[test]
    fn a_regular_file_is_read() {
        let file = std::env::temp_dir().join(format!("iced-file-{}.svg", std::process::id()));
        fs::write(&file, "<svg/>").unwrap();
        let read = read_to_string(&file);
        let _ = fs::remove_file(&file);
        assert_eq!(read.unwrap(), "<svg/>");
    }
}
