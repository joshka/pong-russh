use std::io::{self, Write};

use delegate::delegate;
use ratatui::{
    backend::{Backend, CrosstermBackend, WindowSize},
    layout::Size,
};
use russh::{server::Handle, ChannelId};

/// A backend that writes to an SSH terminal.
///
/// This backend is a wrapper around the crossterm backend that writes to a terminal handle. It
/// delegates most of the methods to the inner crossterm backend, but overrides the methods related
/// to the terminal size and window size.
#[derive(Debug)]
pub struct SshBackend {
    inner: CrosstermBackend<TerminalHandle>,
    size: Size,
    window_size: WindowSize,
}

impl SshBackend {
    pub fn new(
        channel_id: ChannelId,
        session_handle: Handle,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> Self {
        let terminal_handle = TerminalHandle::new(channel_id, session_handle);
        let size = Size::new(col_width as u16, row_height as u16);
        let window_size = WindowSize {
            columns_rows: Size::new(col_width as u16, row_height as u16),
            pixels: Size::new(pix_width as u16, pix_height as u16),
        };
        Self {
            inner: CrosstermBackend::new(terminal_handle),
            size,
            window_size,
        }
    }
}

impl Backend for SshBackend {
    delegate! {
        to self.inner {
            #[allow(late_bound_lifetime_arguments)]
            fn draw<'a, I>(&mut self, content: I) -> std::io::Result<()>
            where
                I: Iterator<Item = (u16, u16, &'a ratatui::prelude::buffer::Cell)>;

            fn hide_cursor(&mut self) -> std::io::Result<()>;
            fn show_cursor(&mut self) -> std::io::Result<()>;
            #[allow(deprecated)]
            fn get_cursor(&mut self) -> std::io::Result<(u16, u16)>;
            #[allow(deprecated)]
            fn set_cursor(&mut self, x: u16, y: u16) -> std::io::Result<()>;
            fn get_cursor_position(&mut self) -> io::Result<ratatui::prelude::Position> ;
            fn set_cursor_position<P: Into<ratatui::prelude::Position>>(&mut self, position: P) -> io::Result<()> ;
            fn clear(&mut self) -> std::io::Result<()>;

        }
    }
    // can't delegate as there is a conflict with the `Write` trait
    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.inner)
    }
    fn size(&self) -> io::Result<Size> {
        Ok(self.size)
    }
    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(self.window_size)
    }
}

#[derive(Clone)]
pub struct TerminalHandle {
    handle: Handle,
    channel_id: ChannelId,
    // The sink collects the data which is finally flushed to the handle.
    sink: Vec<u8>,
}

impl TerminalHandle {
    pub fn new(channel_id: ChannelId, handle: Handle) -> Self {
        Self {
            handle,
            channel_id,
            sink: Vec::new(),
        }
    }
}

impl std::fmt::Debug for TerminalHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalHandle")
            .field("handle", &"...")
            .field("sink", &self.sink)
            .field("channel_id", &self.channel_id)
            .finish()
    }
}

// The crossterm backend writes to the terminal handle.
impl Write for TerminalHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.sink.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let handle = self.handle.clone();
        let channel_id = self.channel_id;
        let data = self.sink.clone().into();
        futures::executor::block_on(async move {
            let result = handle.data(channel_id, data).await;
            if result.is_err() {
                eprintln!("Failed to send data: {:?}", result);
            }
        });

        self.sink.clear();
        Ok(())
    }
}
