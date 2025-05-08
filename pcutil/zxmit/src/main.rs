#![windows_subsystem = "windows"]

mod upload;

use upload::{UploadError, FileUploader, UploadProgress};
use iced::widget::{button, center, checkbox, column, row, text, text_input, progress_bar};
use iced::{Center, Element, Subscription, Event, Task, window::Event as WindowEvent, window};
use std::path::PathBuf;
use std::time;
use serde::{Deserialize, Serialize};

const CARGO_PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[cfg(target_os = "linux")]
fn workarounds() {
    // Wayland backend doesn't implement drag and drop. *sigh*
    unsafe { std::env::set_var("WAYLAND_DISPLAY", ""); }
}

#[cfg(not(target_os = "linux"))]
fn workarounds() {
}

pub fn main() -> iced::Result {
    workarounds();
    let settings: window::settings::Settings = iced::window::settings::Settings {
        size: iced::Size::new(450.0, 400.0),
        resizable: (false),
        ..Default::default()
    };
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .window(settings)
        .title(App::title)
        .run()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    address: String,
    use_compression: bool,
    dummy: bool,
}

#[derive(Debug, Clone)]
enum ConfigError {
    File,
    Dir,
    Format,
}

impl Config {
    fn path() -> PathBuf {
        let dir = match dirs_next::config_dir() {
            Some(dir) => { dir },
            None => { PathBuf::new() },
        };

        dir.join(std::format!(".{}rc", env!("CARGO_CRATE_NAME")))
    }

    async fn load_config() -> Result<Config, ConfigError> {
        let contents = tokio::fs::read_to_string(Self::path())
            .await
            .map_err(|_| ConfigError::File)?;

        serde_json::from_str(&contents).map_err(|_| ConfigError::Format)
    }

    async fn save_config(self) -> Result<(), ConfigError> {
        let json = serde_json::to_string_pretty(&self)
            .map_err(|_| ConfigError::Format)?;

        let path = Self::path();

        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir)
                .await
                .map_err(|_| ConfigError::Dir)?;
        }
        {
            tokio::fs::write(path, json.as_bytes())
                .await
                .map_err(|_| ConfigError::File)?;
        }

        Ok(())
    }

}

#[derive(Debug)]
struct App {
    filepath: Option<PathBuf>,
    address: Option<String>,
    status: String,
    sending: bool,
    dummy: bool,
    use_compression: bool,
    progress: f32,
    total_bytes: usize,
    compressed_bytes: usize,
    now: Option<time::Instant>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            filepath: None,
            address: None,
            status: String::new(),
            sending: false,
            dummy: false,
            use_compression: true,
            progress: 0f32,
            total_bytes: 0,
            compressed_bytes: 0,
            now: None,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    ConfigLoaded(Result<Config, ConfigError>),
    ConfigSaved(Result<(), ConfigError>),
    Uploading(UploadProgress),
    UploadDone(Result<(), UploadError>),
    AddressChanged(String),
    ButtonPressed,
    UseCompressionChanged(bool),
    DummyChanged(bool),
    EventOccurred(Event),
}

impl App {
    fn title(&self) -> String {
        std::format!("ZXmit v{}  © 2025 Vasily Khoruzhick", CARGO_PKG_VERSION.unwrap())
    }

    fn new() -> (Self, Task<Message>) {
        (
            Self { ..Default::default() },
            Task::perform(Config::load_config(), Message::ConfigLoaded),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConfigLoaded(Ok(config)) => {
                *self = Self {
                    address : Some(config.address),
                    use_compression: config.use_compression,
                    dummy: config.dummy,
                    ..Default::default()
                };
                Task::none()
            }
            Message::ConfigLoaded(Err(_)) => {
                println!("Failed to load the config!");
                Task::none()
            }
            Message::ConfigSaved(Ok(())) => {
                if self.sending {
                    self.status = "Config saved.".to_string();
                }
                Task::none()
            }
            Message::ConfigSaved(Err(_)) => {
                self.status = "Failed to save the config!".to_string();
                Task::none()
            }
            Message::AddressChanged(address) => {
                self.address = Some(address);
                Task::none()
            }
            Message::UploadDone(Ok(())) => {
                self.sending = false;
                self.status = std::format!("Upload complete\nCompressed {} into {} bytes\nRatio: {}, time: {:.2?}",
                        self.total_bytes, self.compressed_bytes, self.compressed_bytes as f32 / self.total_bytes as f32,
                        self.now.unwrap().elapsed());
                self.now = None;
                Task::none()
            }
            Message::UploadDone(Err(err)) => {
                self.sending = false;
                self.now = None;
                match err {
                    UploadError::File => {
                        self.status = "Failed to read the file!".to_string();
                    },
                    UploadError::Connection => {
                        self.status = "Connection error, please check the address!".to_string();
                    }
                };
                Task::none()
            }
            Message::Uploading(progress) => {
                self.progress = progress.current_block as f32 / progress.blocks_num as f32;
                self.compressed_bytes = progress.compressed_bytes;
                self.total_bytes = progress.total_bytes;
                Task::none()
            }
            Message::ButtonPressed => {
                self.sending = true;
                self.now = Some(time::Instant::now());
                let task = Task::sip(FileUploader {
                        address: if let Some(addr) = self.address.clone() { addr } else { "".to_string() },
                        filepath: if let Some(path) = self.filepath.clone() { path } else { PathBuf::new() },
                        use_compression: self.use_compression,
                        dummy: self.dummy,
                        }.upload(),
                    Message::Uploading,
                    Message::UploadDone);
                Task::batch(vec![
                    Task::perform(Config {
                        address: if let Some(addr) = self.address.clone() { addr } else { "".to_string() },
                        use_compression: self.use_compression,
                        dummy: self.dummy,
                    }
                    .save_config(),
                    Message::ConfigSaved),
                    task,
                ])
            }
            Message::UseCompressionChanged(value) => {
                self.use_compression = value;
                Task::none()
            }
            Message::DummyChanged(value) => {
                self.dummy = value;
                Task::none()
            }
            Message::EventOccurred(event) => {
                if let Event::Window(WindowEvent::FileDropped(path)) = event {
                    self.filepath = Some(path);
                }
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![ 
            iced::event::listen().map(Message::EventOccurred),
        ])
    }

    fn view(&self) -> Element<Message> {
        let mut button_enabled = !self.sending;

        let address = match &self.address {
            Some(address) => {
                address
            },
            None => {
                button_enabled = false;
                &("".to_string())
            }
        };

        let text_input = text_input("Enter ZX Spectrum IP Address here", address)
            .on_input(Message::AddressChanged)
            .padding(10)
            .size(20);

        let filename = match &self.filepath {
            Some(filepath) => text(filepath.clone().into_os_string().into_string().unwrap()),
            None => {
                button_enabled = false;
                text("Drop the file here!")
            },
        };


        let button_text = if self.sending {
            "Working..."
        } else {
            "Send!"
        };
        let button = button(button_text)
            .padding(10)
            .on_press_maybe(if button_enabled {
                    Some(Message::ButtonPressed)
                } else {
                    None
                });

        let use_compression = checkbox("Use compression", self.use_compression)
            .on_toggle_maybe(if !self.sending {
                Some (Message::UseCompressionChanged)
            } else {
            None
            });

        let dummy = checkbox("Dummy run", self.dummy)
            .on_toggle_maybe(if !self.sending {
                Some (Message::DummyChanged)
            } else {
            None
            });

        let checkboxes = row![
            use_compression,
            dummy,
        ]
        .spacing(20)
        .padding(20);

        let status: Element<Message> = if self.sending {
            progress_bar(0.0..=1.0, self.progress).into()
        } else {
            text(&self.status).align_x(Center).into()
        };

        let content = column![
            text_input,
            filename,
            button,
            checkboxes,
            status,
        ]
        .align_x(Center)
        .spacing(20)
        .padding(20)
        .max_width(400);

        center(content)
            .into()
    }
}
