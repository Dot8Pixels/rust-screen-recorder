use chrono::Local;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::{
    path::{Path, PathBuf},
    time::Instant,
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
}

struct Capture {
    encoder: Option<VideoEncoder>,
    start: Instant,
}

impl GraphicsCaptureApiHandler for Capture {
    type Flags = HashMap<String, Value>;

    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(user_settings: Self::Flags) -> Result<Self, Self::Error> {
        let output_path = if let Some(output_path) = user_settings.get("output_path") {
            match output_path {
                Value::PathBuf(path) => Some(path),
                _ => {
                    println!("Key exists, but it's not a PathBuf");
                    None
                }
            }
        } else {
            println!("Key 'output_path' not found");
            None
        };

        if let Some(output_path) = output_path {
            println!("The path is: {:?}", output_path);
        }

        let monitor = if let Some(monitor) = user_settings.get("monitor") {
            match monitor {
                Value::Monitor(monitor) => Some(monitor),
                _ => {
                    println!("Key exists, but it's not a Monitor");
                    None
                }
            }
        } else {
            println!("Key 'output_path' not found");
            None
        };

        if let Some(monitor) = monitor {
            println!("The monitor is: {:?}", monitor);
        }

        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(
                monitor.unwrap().width().unwrap(),
                monitor.unwrap().height().unwrap(),
            ),
            AudioSettingsBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            output_path.unwrap(),
        )?;

        Ok(Self {
            encoder: Some(encoder),
            start: Instant::now(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        print!(
            "\rRecording for: {} seconds",
            self.start.elapsed().as_secs()
        );
        io::stdout().flush()?;

        self.encoder.as_mut().unwrap().send_frame(frame)?;

        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let trigger_path = project_root.join("trigger.txt");
        let trigger =
            fs::read_to_string(trigger_path).expect("Should have been able to read the file");

        if trigger == *"1" {
            self.encoder.take().unwrap().finish()?;
            capture_control.stop();
            println!();
        }

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture Session Closed");

        Ok(())
    }
}

fn open_text_file(file_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    open::that(file_path)?; // This opens the file in the system's default application
    Ok(())
}
fn main() {
    let primary_monitor = Monitor::primary().expect("There is no primary monitor");

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let datetime = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let record_name = format!("screen_recording_{}.mp4", datetime);
    let captures_path = project_root.join("Captures").join(record_name);

    let trigger_path = project_root.join("trigger.txt");
    fs::write(trigger_path.clone(), "0").unwrap();
    match open_text_file(trigger_path) {
        Ok(_) => println!("Text file opened successfully."),
        Err(e) => eprintln!("Failed to open text file: {}", e),
    }

    let mut user_settings: HashMap<String, Value> = HashMap::new();
    user_settings.insert(String::from("output_path"), Value::PathBuf(captures_path));
    user_settings.insert(String::from("monitor"), Value::Monitor(primary_monitor));

    let settings = Settings::new(
        primary_monitor,
        CursorCaptureSettings::WithoutCursor,
        DrawBorderSettings::Default,
        ColorFormat::Rgba8,
        user_settings,
    );

    Capture::start(settings).expect("Screen Capture Failed");
}
