#[macro_use(load_yaml)]
extern crate clap;
extern crate dirs;
extern crate image;
#[macro_use]
extern crate log;
extern crate stderrlog;

use clap::App;
use std::{
    path::{Path, PathBuf},
    sync::mpsc::channel,
    thread,
};

mod i3_listener;
mod worker;

fn main() {
    let yaml = load_yaml!("args.yaml");
    let matches = App::from_yaml(yaml).get_matches();
    let verbosity = match matches.occurrences_of("verbose") {
        0 => 1,
        1 => {
            println!("Log level: info");
            2
        }
        2 => {
            println!("Log level: debug");
            3
        }
        3 | _ => {
            println!("Log level: trace");
            4
        }
    };
    let quiet = matches.is_present("quiet");
    stderrlog::new()
        .color(stderrlog::ColorChoice::Auto)
        .timestamp(stderrlog::Timestamp::Second)
        .verbosity(verbosity)
        .quiet(quiet)
        .init()
        .unwrap(); // Err only if stderrlog is already initialized

    let transitions = match matches.value_of("transitions").unwrap_or("3").parse::<u8>() {
        Ok(value) => value,
        Err(e) => {
            error!("Unable to set transitions: {}", e);
            std::process::exit(22); // Invalid argument err code
        }
    };
    let sigma = match matches.value_of("sigma").unwrap_or("12.0").parse::<f32>() {
        Ok(value) => value,
        Err(e) => {
            error!("Unable to set sigma: {}", e);
            std::process::exit(22); // Invalid argument err code
        }
    };

    let bg_path_file_path = dirs::cache_dir()
        .expect("Can't get cache directory")
        .join(Path::new("wal/wal"));

    loop {
        // Give time for i3 to load and wal to set wallpaper
        std::thread::sleep(std::time::Duration::new(1, 0));

        let bg_path = match std::fs::read_to_string(&bg_path_file_path) {
            Ok(r) => PathBuf::from(r),
            Err(e) => {
                warn!("Unable to read {:?}: {}", &bg_path_file_path, e);
                continue;
            }
        };

        info!("Current background image: {:?}", bg_path);

        blur_images(&bg_path, transitions, sigma);

        let (send, recv) = channel();
        let listener = thread::spawn(move || {
            i3_listener::listen(send);
        });

        let worker = thread::spawn(move || {
            worker::work(recv, &bg_path, transitions);
        });

        let listener_result = listener.join();
        let worker_result = worker.join();
        debug!("Listener thread joined: {:?}", listener_result);
        debug!("Worker thread joined: {:?}", worker_result);
    }
}

fn blur_images(bg_path: &PathBuf, transitions: u8, sigma: f32) {
    let blur_cache_dir = dirs::cache_dir()
        .expect("Can't get cache directory")
        .join("i3-bg-blur");
    if let Err(e) = std::fs::remove_dir_all(&blur_cache_dir) {
        info!("Can not clear cache directory: {}", e);
    } else {
        trace!("Removed old cache directory");
    }
    if let Err(e) = std::fs::create_dir_all(&blur_cache_dir) {
        error!("Can not create directory {:?}: {}", blur_cache_dir, e);
        panic!();
    }

    let bg = image::open(&bg_path).unwrap();

    let threads: Vec<_> = (0..transitions)
        .map(|i| {
            let bg = bg.clone();
            let bg_ext = bg_path.extension().unwrap().to_os_string();

            thread::spawn(move || {
                let blur_strength = sigma / f32::from(transitions) * f32::from(i + 1);
                trace!("Started blur with sigma {}", blur_strength);
                let blured = bg.blur(blur_strength);
                let mut blured_path = dirs::cache_dir()
                    .expect("Can't get cache directory")
                    .join("i3-bg-blur/filename"); // Filename gets stripped with set_file_name()
                blured_path.set_file_name((i + 1).to_string());
                blured_path.set_extension(bg_ext);
                trace!("Finished blur with sigma {}", blur_strength);
                blured
                    .save(Path::new(blured_path.as_path()))
                    .expect("Can't save blured image");
            })
        }).collect();

    for thread in threads {
        thread.join().unwrap();
    }
    info!("Finished bluring images");
}
