extern crate image;

use std::thread;
use std::sync::Arc;
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let home = env::home_dir().expect("Can't get home directory"); // home
    let mut transitions = 3; // read from args

    for arg in env::args() { // argparse
        println!("{}", arg);
    }

    let wp_path_file_path = home.as_path().join(Path::new(".cache/wal/wal"));

    let mut wp_path_file = File::open(wp_path_file_path).expect("Wallpaper path file not found");
    let mut wallpaper_path_string = String::new();
    wp_path_file.read_to_string(&mut wallpaper_path_string);
    println!("Current wallpaper: {}", wallpaper_path_string);
    let wallpaper_path = Path::new(&wallpaper_path_string);
    let wallpaper = Arc::new(image::open(&wallpaper_path).unwrap());

    // blured_wallpaper.save(Path::new("/tmp/image1.jpg"));

    let mut threads = vec![];
    for i in 0..transitions {
        let wallpaper = wallpaper.clone();
        let mut blured_path = home.clone();
        let wallpaper_extension = wallpaper_path.extension().unwrap().to_os_string();

        threads.push(thread::spawn(move || {
            let blured = wallpaper.blur(12.0 / transitions as f32 * (i + 1) as f32);
            blured_path = blured_path.join(".cache/wpblur/filename"); // Filename gets stripped with set_file_name()
            blured_path.set_file_name(i.to_string());
            blured_path.set_extension(wallpaper_extension);
            println!("Blur for {}; Path: {:?}", 12.0 / transitions as f32 * (i + 1) as f32, blured_path);
            blured.save(Path::new(blured_path.as_path()));
            println!("{}", i);
        }));
    }
    for thread in threads {
        thread.join().unwrap();
    }
}
