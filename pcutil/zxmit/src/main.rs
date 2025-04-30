#![windows_subsystem = "windows"]

use iced::widget::{button, center, column, text, text_input};
use iced::{Center, Element, Subscription, Event, Task, window::Event as WindowEvent, window};
use std::path::PathBuf;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;

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
        size: iced::Size::new(450.0, 350.0),
        resizable: (false),
        ..Default::default()
    };
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .window(settings)
        .title("ZXmit")
        .run()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    address: String,
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

#[derive(Debug, Clone)]
enum UploadError {
    File,
    Connection,
}

#[derive(Debug)]
struct FileUploader {
    address: String,
    filepath: PathBuf,
}

impl FileUploader {
    fn split_at_last_dot(filename: &str) -> (String, String) {
        let parts: Vec<&str> = filename.split('.').collect();

        if parts.len() <= 1 {
            return (filename.to_string(), "".to_string());
        }

        let last = parts.last().unwrap().to_string();
        let first = parts[..parts.len() - 1].join(" ");

        (first, last)
    }

    fn filename_to_short(filename: &str) -> String {
        let (mut name, mut extension) = Self::split_at_last_dot(filename);

        if name.len() > 8 {
            name = name.chars().take(8).collect();
        }
        if extension.len() > 3 {
            extension = extension.chars().take(3).collect();
        }

        let re = Regex::new(r"[ \t\.\\/]").unwrap();

        name = re.replace_all(&name, "_").to_string();
        extension = re.replace_all(&extension, "_").to_string();

        let res = std::format!("{}.{}", name, extension);
        println!("Short filename is {}", &res);

        res
    }

    async fn upload(self) -> Result<(), UploadError> {
        let mut file = tokio::fs::read(self.filepath.clone())
            .await
            .map_err(|_| UploadError::File)?;
        let basename = self.filepath.as_path().file_name().unwrap().to_str().unwrap();
        let mut namebuf: Vec<u8> = Self::filename_to_short(basename).into();
        assert!(namebuf.len() <= 32);
        namebuf.resize(32, 0);
        namebuf.append(&mut file);

        let addr = format!("{}:6144", self.address);

        let mut stream = TcpStream::connect(addr)
            .await
            .map_err(|_| UploadError::Connection)?;

        {
            stream.write_all(&namebuf)
                .await
                .map_err(|_| UploadError::Connection)?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
struct App {
    filepath: Option<PathBuf>,
    address: Option<String>,
    status: String,
    sending: bool,
}

#[derive(Debug, Clone)]
enum Message {
    ConfigLoaded(Result<Config, ConfigError>),
    ConfigSaved(Result<(), ConfigError>),
    UploadDone(Result<(), UploadError>),
    AddressChanged(String),
    ButtonPressed,
    EventOccurred(Event),
}

impl App {
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
                self.status = "Upload complete.".to_string();
                Task::none()
            }
            Message::UploadDone(Err(err)) => {
                self.sending = false;
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
            Message::ButtonPressed => {
                self.sending = true;
                Task::batch(vec![
                    Task::perform(Config {
                        address: if let Some(addr) = self.address.clone() { addr } else { "".to_string() },
                    }
                    .save_config(),
                    Message::ConfigSaved),
                    Task::perform(FileUploader {
                        address: if let Some(addr) = self.address.clone() { addr } else { "".to_string() },
                        filepath: if let Some(path) = self.filepath.clone() { path } else { PathBuf::new() },
                    }
                    .upload(),
                    Message::UploadDone),
                ])
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

        let status_text = text(&self.status);

        let content = column![
            text_input,
            filename,
            button,
            status_text,
            text("© 2025 Vasily Khoruzhick"),
        ]
        .align_x(Center)
        .spacing(20)
        .padding(20)
        .max_width(400);

        center(content)
            .into()
    }
}
