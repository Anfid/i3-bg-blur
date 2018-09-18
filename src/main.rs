extern crate image;

use std::{
    env,
    path::{Path, PathBuf},
    sync::mpsc::channel,
    sync::Arc,
    thread,
};

mod i3_listener;
mod worker;

fn main() {
    let home = env::home_dir().expect("Can't get home directory"); // home
    let transitions: u8 = 3; // TODO: read from args

    for arg in env::args() {
        // TODO: argparse
        println!("{}", arg);
    }

    std::fs::create_dir_all(home.join(".cache/i3-bg-blur"))
        .expect("Error while creating cache dir");

    let bg_path_file_path = home.as_path().join(Path::new(".cache/wal/wal"));

    loop {
        // Give time for i3 to load and wal to set wallpaper
        std::thread::sleep(std::time::Duration::new(1, 0));

        let bg_path = match std::fs::read_to_string(&bg_path_file_path) {
            Ok(r) => PathBuf::from(r),
            Err(e) => {
                println!("Error reading {:?}: {}", &bg_path_file_path, e);
                continue;
            }
        };

        println!("Current background image: {:?}", bg_path);

        {
            let bg = Arc::new(image::open(&bg_path).unwrap());

            let threads: Vec<_> = (0..transitions)
                .map(|i| {
                    let bg = bg.clone();
                    let mut blured_path = home.clone();
                    let bg_ext = bg_path.extension().unwrap().to_os_string();

                    thread::spawn(move || {
                        let blured = bg.blur(12.0 / transitions as f32 * (i + 1) as f32);
                        blured_path = blured_path.join(".cache/i3-bg-blur/filename"); // Filename gets stripped with set_file_name()
                        blured_path.set_file_name(i.to_string());
                        blured_path.set_extension(bg_ext);
                        println!(
                            "Blur for {}; Path: {:?}",
                            12.0 / transitions as f32 * (i + 1) as f32,
                            blured_path
                        );
                        println!("{:?}", blured.save(Path::new(blured_path.as_path())));
                        println!("{}", i);
                    })
                })
                .collect();

            for thread in threads {
                thread.join().unwrap();
            }
        }

        let (send, recv) = channel();
        let listener = thread::spawn(move || {
            i3_listener::listen(send);
        });

        let worker = thread::spawn(move || {
            worker::work(recv, PathBuf::from(bg_path), transitions);
        });

        println!("Main: Listener joined: {:?}", listener.join());
        println!("Main: Worker joined: {:?}", worker.join());
    }
}
