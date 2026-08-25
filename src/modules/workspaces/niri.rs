use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

use serde_json::Value;

use crate::logger::{error, info};

use super::{CompositorEvent, IpcBackend, Workspace};

const NIRI_SOCKET_VAR: &str = "NIRI_SOCKET";

const STREAM_INIT_MSG: &str = "\"EventStream\"\n";
const REQUEST_HANDLED: &str = r#"{"Ok":"Handled"}"#;

pub struct NiriIpcBackend {
    buffer: String,
    receiver: BufReader<UnixStream>,
}

impl IpcBackend for NiriIpcBackend {
    fn next_event(&mut self) -> Option<CompositorEvent> {
        self.buffer.clear();
        self.receiver.read_line(&mut self.buffer).ok()?;

        parse_compositor_event(&self.buffer)
    }

    fn try_create() -> Option<Self>
    where
        Self: Sized,
    {
        let socket_path = env::var(NIRI_SOCKET_VAR).ok()?;
        let unix_socket = UnixStream::connect(socket_path).ok()?;
        let mut receiver = BufReader::new(unix_socket);

        receiver
            .get_mut()
            .write_all(STREAM_INIT_MSG.as_bytes())
            .ok()?;

        let mut buffer = String::new();
        receiver.read_line(&mut buffer).ok()?;

        if buffer.trim() != REQUEST_HANDLED {
            return None;
        }

        if let Err(reason) = receiver.get_mut().shutdown(Shutdown::Write) {
            error(format!(
                "Could not establish a clean connection to Niri due to {reason}"
            ))
        };

        Some(Self {
            buffer: String::new(),
            receiver,
        })
    }
}

fn parse_compositor_event(content: &str) -> Option<CompositorEvent> {
    let parsed = content.parse::<Value>().ok()?;

    println!("{parsed:#?}");
    if let Some(workspaces) = parsed.get("WorkspacesChanged") {
        return workspaces_changed(workspaces);
    }

    if let Some(workspace) = parsed.get("WorkspaceActivated") {
        return workspace_activated(workspace);
    }

    // info(format!("[Niri] Unhandled event {parsed}"));
    None
}

fn workspaces_changed(workspaces: &Value) -> Option<CompositorEvent> {
    let workspaces = workspaces.get("workspaces")?.as_array()?;
    let mut workspace_list = Vec::new();

    // {
    //   "id": 1,
    //   "idx": 1,
    //   "name": null,
    //   "output": "HDMI-A-1",
    //   "is_urgent": false,
    //   "is_active": true,
    //   "is_focused": true,
    //   "active_window_id": 9
    // }
    // Since we're communicating with a remote process and not reading from user input, we can be
    // a bit more relaxed with the error handling. That being said, if any of these were to silently
    // fail, it would be hell to debug
    for workspace in workspaces {
        let id = workspace.get("id")?.as_i64()?.cast_unsigned();
        let idx = workspace.get("idx")?.as_i64()? as u8;
        let name = workspace.get("name")?.as_str().map(|str| str.to_string());
        let is_focused = workspace.get("is_focused")?.as_bool()?;

        let workspace = Workspace {
            id,
            idx,
            name,
            is_focused,
        };
        workspace_list.push(workspace);
    }

    workspace_list.sort_by(|workspace_1, workspace_2| workspace_1.idx.cmp(&workspace_2.idx));

    Some(CompositorEvent::WorkspacesChanged(workspace_list))
}

fn workspace_activated(workspace: &Value) -> Option<CompositorEvent> {
    let is_focused = workspace.get("focused")?.as_bool()?;
    let id = workspace.get("id")?.as_i64()?.cast_unsigned();

    Some(CompositorEvent::WorkspaceActivated(Workspace {
        id,
        idx: 0,
        is_focused,
        name: None,
    }))
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::modules::workspaces::{
        IpcBackend,
        niri::{NIRI_SOCKET_VAR, NiriIpcBackend},
    };

    #[test]
    fn test_create_niri_backend() {
        if env::var(NIRI_SOCKET_VAR).is_err() {
            return;
        }
        let mut niri = <NiriIpcBackend as IpcBackend>::try_create().unwrap();
        assert!(niri.next_event().is_some())
    }
}
