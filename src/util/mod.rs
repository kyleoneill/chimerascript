use std::time::Duration;
use std::time::Instant;

pub struct Timer {
    start: Instant
}

impl Timer {
    pub fn new() -> Self {
        Self { start: Instant::now() }
    }
    pub fn finish(&self) -> String {
        let duration = self.start.elapsed();
        duration_to_readable_time(duration)
    }
}

fn duration_to_readable_time(duration: Duration) -> String {
    let as_secs = duration.as_secs();
    match as_secs {
        0 => {
            let as_millis = duration.as_millis();
            match as_millis {
                0 => {
                    format!("{}μ", duration.as_micros())
                },
                _ => format!("{}ms", as_millis)
            }
        },
        _ => format!("{}s", as_secs)
    }
}
