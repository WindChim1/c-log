use std::{env, path::PathBuf, thread, time::Duration};

use c_log::{Level, LogFileBuild, Logger};

fn main() {
    let current_dir = env::current_dir().unwrap();
    let pkg_name = env!("CARGO_PKG_NAME");
    let log_path = format!("{}/logger/{}.log", current_dir.to_str().unwrap(), pkg_name);

    let new = Logger::new()
        .level(Level::Trace)
        .log_file_config(
            LogFileBuild::new()
                .paths(vec![PathBuf::from(log_path)])
                .archive(true)
                .how_long(c_log::ArchiveDurantion::OneHour)
                .build(),
        )
        .print(true);
    new.init();
    (1..=1000000000).for_each(|i| {
        log::info!("info:{}", i);
        log::warn!("warn:{}", i);
        log::error!("error:{}", i);
        log::debug!("debug:{}", i);
        log::trace!("trace:{}", i);
        thread::sleep(Duration::from_secs(60))
    });
}
