use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{Receiver, TryRecvError},
    thread,
    time::Duration,
};

pub fn work(receiver: Receiver<bool>, bg_path: PathBuf, transitions: u8) {
    println!("Worker created");

    let home = env::home_dir().expect("Can't get home directory"); // home
    let mut bg_current = home.as_path().join(Path::new(".cache/i3-bg-blur/filename"));

    let mut blur = false;
    let mut i = 0;

    loop {
        match receiver.try_recv() {
            Ok(state) => {
                blur = state;
                println!("Worker Ok: {}", state);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return,
        }

        if blur && i != transitions {
            i += 1;
            bg_current.set_file_name((i - 1).to_string());
            bg_current.set_extension(bg_path.extension().unwrap().to_os_string());
            println!("Setting {:?}", bg_current);
            let mut success = false;
            if let Ok(mut handle) = Command::new("feh")
                .arg("--bg-fill")
                .arg(&bg_current)
                .spawn()
            {
                match handle.wait() {
                    Ok(_) => success = true,
                    Err(_) => success = false,
                }
            }
            if !success {
                // Reset i in case of feh fail
                i -= 1;
                println!("Error setting background image");
            }
        } else if !blur && i != 0 {
            i -= 1;
            let mut success = false;
            if i == 0 {
                println!("Setting {:?}", bg_path);
                if let Ok(mut handle) = Command::new("feh").arg("--bg-fill").arg(&bg_path).spawn() {
                    match handle.wait() {
                        Ok(_) => success = true,
                        Err(_) => success = false,
                    }
                }
            } else {
                bg_current.set_file_name((i - 1).to_string());
                bg_current.set_extension(bg_path.extension().unwrap().to_os_string());
                println!("Setting {:?}", bg_current);
                if let Ok(mut handle) = Command::new("feh")
                    .arg("--bg-fill")
                    .arg(&bg_current)
                    .spawn()
                {
                    match handle.wait() {
                        Ok(_) => success = true,
                        Err(_) => success = false,
                    }
                }
            }

            if !success {
                // Reset i in case of feh fail
                i += 1;
                println!("Error setting background image");
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}
