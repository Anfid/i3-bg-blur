use dirs;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{Receiver, TryRecvError},
    thread,
    time::Duration,
};

pub fn work(receiver: Receiver<bool>, bg_path: &PathBuf, transitions: u8) {
    debug!("Worker created");

    let bg_cache = dirs::cache_dir()
        .expect("Can't get cache directory")
        .as_path()
        .join(Path::new("i3-bg-blur/filename"));

    let mut do_blur = false;
    let mut i = 0;

    loop {
        match receiver.try_recv() {
            Ok(state) => {
                do_blur = state;
                trace!("Worker received Ok({})", state);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                debug!("Worker received Err(Disconnected)");
                return;
            }
        }

        let old_i = i;
        let bg = if do_blur && i != transitions {
            i += 1;
            Some(
                bg_cache
                    .with_file_name(i.to_string())
                    .with_extension(bg_path.extension().unwrap().to_os_string()),
            )
        } else if !do_blur && i != 0 {
            i -= 1;
            if i == 0 {
                // Set unblured
                Some(PathBuf::from(&bg_path))
            } else {
                Some(
                    bg_cache
                        .with_file_name(i.to_string())
                        .with_extension(bg_path.extension().unwrap().to_os_string()),
                )
            }
        } else {
            None
        };

        if let Some(bg) = bg {
            if let Err(e) = set_bg(&bg) {
                // Reset i in case of feh fail
                warn!("Unable to set background image. Exit code: {}", e);
                i = old_i;
            }
        } else {
            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn set_bg(bg_path: &PathBuf) -> Result<(), i32> {
    debug!("Setting background image: {:?}", bg_path);

    let result = Command::new("feh")
        .arg("--bg-fill")
        .arg(&bg_path)
        .spawn()
        .expect("Failed to start feh")
        .wait()
        .expect("Failed to start feh");

    if result.success() {
        Ok(())
    } else {
        Err(result.code().unwrap_or(125))
    }
}
