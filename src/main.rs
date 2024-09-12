use chrono::Local;
use crossterm::terminal;
use std::{
    collections::HashMap,
    env,
    io::{stdin, Read},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};
use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    encoder::{AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
};

#[derive(Debug)]
enum Value {
    PathBuf(PathBuf),
    Monitor(Monitor),
    Flag(Arc<Mutex<bool>>),
}

struct Capture {
    encoder: Option<VideoEncoder>,
    flag: Arc<Mutex<bool>>,
}

impl GraphicsCaptureApiHandler for Capture {
    type Flags = HashMap<String, Value>;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(user_settings: Self::Flags) -> Result<Self, Self::Error> {
        let output_path = match user_settings.get("output_path") {
            Some(Value::PathBuf(path)) => Some(path),
            _ => {
                println!("Key 'output_path' not found or invalid");
                None
            }
        };

        let monitor = match user_settings.get("monitor") {
            Some(Value::Monitor(monitor)) => Some(monitor),
            _ => {
                println!("Key 'monitor' not found or invalid");
                None
            }
        };

        let flag = match user_settings.get("flag") {
            Some(Value::Flag(flag)) => Some(flag),
            _ => {
                println!("Key 'flag' not found or invalid");
                None
            }
        };

        if let (Some(output_path), Some(monitor), Some(flag)) = (output_path, monitor, flag) {
            let encoder = VideoEncoder::new(
                VideoSettingsBuilder::new(monitor.width().unwrap(), monitor.height().unwrap()),
                AudioSettingsBuilder::default().disabled(true),
                ContainerSettingsBuilder::default(),
                output_path.clone(),
            )?;

            Ok(Self {
                encoder: Some(encoder),
                flag: Arc::clone(flag),
            })
        } else {
            Err("Failed to initialize Capture".into())
        }
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        self.encoder.as_mut().unwrap().send_frame(frame)?;

        if *self.flag.lock().unwrap() {
            self.encoder.take().unwrap().finish()?;
            capture_control.stop();
            println!("Stop recording...");
        }

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture Session Closed");
        Ok(())
    }
}

fn main() {
    let primary_monitor = Monitor::primary().expect("There is no primary monitor");

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let datetime = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let record_name = format!("screen_recording_{}.mp4", datetime);
    let captures_path = project_root.join("Captures").join(record_name);

    let shared_flag = Arc::new(Mutex::new(false));
    let shared_flag_clone = Arc::clone(&shared_flag);

    println!("Start recording...");
    println!("Press 'q' to stop recorder");
    let handle = thread::spawn(move || {
        terminal::enable_raw_mode().expect("Could not turn on Raw mode");
        loop {
            let mut buf = [0; 1];
            if stdin().read(&mut buf).expect("Failed to read line") == 1 {
                let character = buf[0];
                if character == b'q' {
                    let mut flag = shared_flag_clone.lock().unwrap();
                    *flag = true;
                    terminal::disable_raw_mode().expect("Could not disable raw mode");
                    break;
                }
            }
        }
    });

    let mut user_settings: HashMap<String, Value> = HashMap::new();
    user_settings.insert("output_path".to_string(), Value::PathBuf(captures_path));
    user_settings.insert("monitor".to_string(), Value::Monitor(primary_monitor));
    user_settings.insert("flag".to_string(), Value::Flag(shared_flag));

    let settings = Settings::new(
        primary_monitor,
        CursorCaptureSettings::WithoutCursor,
        DrawBorderSettings::Default,
        ColorFormat::Rgba8,
        user_settings,
    );

    Capture::start(settings).expect("Screen Capture Failed");

    handle.join().unwrap();

    println!("Finished recorder")
}
