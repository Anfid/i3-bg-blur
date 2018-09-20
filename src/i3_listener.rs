extern crate i3ipc;

use self::i3ipc::{
    event::{inner::*, Event},
    reply::NodeType,
    I3Connection, I3EventListener, Subscription,
};
use std::sync::mpsc::Sender;

pub fn listen(sender: Sender<bool>) {
    debug!("i3 Listener created");

    let mut i3_connection = I3Connection::connect().expect("Unable to create connection");
    let mut i3_listener = I3EventListener::connect().expect("Unable to create listener");

    let mut send_result = if current_workspace_is_empty(&mut i3_connection) {
        sender.send(false)
    } else {
        sender.send(true)
    };

    let subscriptions = [Subscription::Workspace, Subscription::Window];
    i3_listener
        .subscribe(&subscriptions)
        .expect("Unable to subscribe");

    for event in i3_listener.listen() {
        if let Err(e) = send_result {
            warn!("Listener unable to send state: {}", e);
            warn!("Listener exiting");
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
                warn!("Listener unable to get event: {}", e);
                return;
            }
            _ => unreachable!(),
        }
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
