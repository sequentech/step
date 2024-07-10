use indicatif::{ProgressBar, ProgressStyle};

// returns the progress bar to be called with pb.finish_with_message("msg")

pub fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}"));
    pb.enable_steady_tick(100);
    pb.set_message(String::from(msg));
    pb
}

pub fn create_bar(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_bar().template("{spinner} {msg}"));
    pb.enable_steady_tick(100);
    pb.set_message(String::from(msg));
    pb
}
