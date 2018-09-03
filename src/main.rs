extern crate i3ipc;
extern crate image;

use std::env;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread;

use i3ipc::{
    event::{inner::*, Event},
    reply::NodeType,
    I3Connection, I3EventListener, Subscription,
};

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
            listen(send);
        });

        let worker = thread::spawn(move || {
            work(recv, PathBuf::from(bg_path), transitions);
        });

        println!("Main: Listener joined: {:?}", listener.join());
        println!("Main: Worker joined: {:?}", worker.join());
    }
}

fn current_workspace_is_empty(connection: &mut I3Connection) -> bool {
    let outputs = connection.get_tree().unwrap().nodes;
    for output in outputs {
        for section in &output.nodes {
            if let NodeType::Con = section.nodetype {
                for workspace in &section.nodes {
                    if workspace.focused && workspace.nodes.is_empty() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn work(receiver: Receiver<bool>, bg_path: PathBuf, transitions: u8) {
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
            if let Err(e) = std::process::Command::new("feh")
                .arg("--bg-fill")
                .arg(&bg_current)
                .spawn()
            {
                // Reset i in case of feh fail
                i -= 1;
                println!("Error setting background image: {:?}", e);
            }
        } else if !blur && i != 0 {
            i -= 1;
            let result;
            if i == 0 {
                println!("Setting {:?}", bg_path);
                result = std::process::Command::new("feh")
                    .arg("--bg-fill")
                    .arg(&bg_path)
                    .spawn();
            } else {
                bg_current.set_file_name((i - 1).to_string());
                bg_current.set_extension(bg_path.extension().unwrap().to_os_string());
                println!("Setting {:?}", bg_current);
                result = std::process::Command::new("feh")
                    .arg("--bg-fill")
                    .arg(&bg_current)
                    .spawn();
            }

            if let Err(e) = result {
                // Reset i in case of feh fail
                i += 1;
                println!("Error setting background image: {:?}", e);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn listen(sender: Sender<bool>) {
    println!("Listener created");

    let mut i3_connection = I3Connection::connect().expect("Unable to create connection");
    let mut i3_listener = I3EventListener::connect().expect("Unable to create listener");

    let mut send_result;
    if current_workspace_is_empty(&mut i3_connection) {
        send_result = sender.send(false);
    } else {
        send_result = sender.send(true);
    }

    let subscriptions = [Subscription::Workspace, Subscription::Window];
    i3_listener
        .subscribe(&subscriptions)
        .expect("Unable to subscribe");

    for event in i3_listener.listen() {
        if let Err(e) = send_result {
            println!("Send error in listener: {}\nExiting thread", e);
        }

        match event {
            Ok(Event::WindowEvent(event)) => match event.change {
                WindowChange::Close => {
                    if current_workspace_is_empty(&mut i3_connection) {
                        send_result = sender.send(false);
                    }
                }
                WindowChange::Focus => {
                    send_result = sender.send(true);
                }
                _ => continue,
            },
            Ok(Event::WorkspaceEvent(event)) => match event.change {
                WorkspaceChange::Focus => {
                    if event
                        .current
                        .expect("Failed getting current workspace")
                        .nodes
                        .is_empty()
                    {
                        send_result = sender.send(false);
                    } else {
                        send_result = sender.send(true);
                    }
                }
                _ => continue,
            },
            Err(e) => {
                println!("Listener error: {}", e);
                return;
            }
            _ => unreachable!(),
        }
    }
}
