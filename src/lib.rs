use chrono::{DateTime, Duration, Local, Timelike};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use log::Log;

/// 日志配置项
#[derive(Debug)]
pub struct Logger {
    /// 日志最高输出等级
    level: Option<Level>,
    /// 日志输出到文件配置项
    log_file_config: Option<LogFile>,
    /// 是否输出到控制台
    print: Option<bool>,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            level: Default::default(),
            log_file_config: Default::default(),
            print: Some(true),
        }
    }
}

///日志输出到文件配置项：
#[derive(Debug, Clone)]
pub struct LogFile {
    /// 是否归档
    archive: bool,
    ///日志输出日志文件，支持多个
    paths: Vec<PathBuf>,
    ///多久归档一次,以时间为单位
    how_long: Option<ArchiveDurantion>,
}

#[derive(Default)]
pub struct LogFileBuild {
    /// 是否归档
    pub archive: bool,
    ///日志输出日志文件，支持多个
    pub paths: Vec<PathBuf>,
    ///多久归档一次,以时间为单位
    pub how_long: Option<ArchiveDurantion>,
}

impl LogFileBuild {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn archive(mut self, archive: bool) -> Self {
        self.archive = archive;
        self
    }
    pub fn paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.paths = paths;
        self
    }
    pub fn how_long(mut self, how_long: ArchiveDurantion) -> Self {
        self.how_long = Some(how_long);
        self
    }
    pub fn build(self) -> LogFile {
        LogFile {
            archive: self.archive,
            paths: self.paths,
            how_long: if self.how_long.is_none() {
                Some(ArchiveDurantion::ThreeHour)
            } else {
                self.how_long
            },
        }
    }
}

impl LogFile {
    fn archive_log(log_file: LogFile) {
        if !log_file.archive {
            return;
        }
        if let Some(duration) = &log_file.how_long {
            let duration = duration.get_durantion();
            let file_arc = Arc::new(Mutex::new(log_file));
            let mut current_time = Local::now();
            let mut next_hour = next_time(current_time, duration);
            loop {
                let current_hourt = Local::now().format("%Y%m%d%H");
                let interval = next_hour - current_time;
                dbg!("interval:{}", interval);
                //sleep截止到下次归档时间
                thread::sleep(interval.to_std().unwrap());
                let file_arc_clone = file_arc.clone();
                //归档
                //NOTE:  需要获取所有权
                thread::Builder::new()
                    .name("archive thread in loop".to_string())
                    .spawn(move || {
                        let file_arc_lock = file_arc_clone.lock().unwrap();
                        let file_log = &*file_arc_lock;
                        file_log.paths.iter().for_each(|old_path| {
                            let new_path = match old_path.to_str() {
                                Some(path) => {
                                    let truncated = &path[..path.len() - 4];
                                    PathBuf::from(format!(
                                        "{}{}.log",
                                        truncated,
                                        current_hourt.to_string()
                                    ))
                                }
                                None => panic!("日志文件地址错误"),
                            };
                            fs::rename(old_path, new_path).unwrap();
                        })
                    })
                    .unwrap();

                //重置时间
                current_time = next_hour;
                next_hour = next_time(current_time, duration);
            }
        }
    }
}
fn next_time(current_time: DateTime<Local>, long: Duration) -> DateTime<Local> {
    current_time
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
        + long
}

///日志文件归档间断,默认三小时
#[derive(Debug, Clone)]
pub enum ArchiveDurantion {
    ///一小时
    OneHour,
    ///三小时
    ThreeHour,
    ///六小时
    SixHour,
    ///十二小时
    TwelveHour,
    ///一天
    OneDay,
}

impl ArchiveDurantion {
    fn get_durantion(&self) -> Duration {
        match self {
            ArchiveDurantion::OneHour => Duration::hours(1),
            ArchiveDurantion::ThreeHour => Duration::hours(3),
            ArchiveDurantion::SixHour => Duration::hours(6),
            ArchiveDurantion::TwelveHour => Duration::hours(12),
            ArchiveDurantion::OneDay => Duration::days(1),
        }
    }
}
// impl Default for ArchiveDurantion {
//     fn default() -> Self {
//         Self::ThreeHour
//     }
// }

#[derive(Debug, PartialEq, Eq)]
pub enum Level {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    // This way these line up with the discriminants for LevelFilter below
    // This works because Rust treats field-less enums the same way as C does:
    // https://doc.rust-lang.org/reference/items/enumerations.html#custom-discriminant-values-for-field-less-enumerations
    Error = 1,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn,
    /// The "info" level.
    ///
    /// Designates useful information.
    Info,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace,
}
struct Aid;

lazy_static! {
    static ref LOG: Mutex<Logger> = Mutex::new(Logger::new());
}
impl Log for Aid {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let color = match record.level() {
            log::Level::Error => 31, //red
            log::Level::Warn => 93,  //bright yellow
            log::Level::Info => 34,  //blue
            log::Level::Debug => 32, //green
            log::Level::Trace => 90, //bright black
        };

        let lock_log = LOG.lock().unwrap();
        let mut format = String::new();

        if let Some(print) = lock_log.print {
            // 获取当前时间并格式化为所需格式
            let current_time = Local::now().format("%Y-%m-%d %H:%M:%S");
            //输出到控制台
            if print {
                format = format!(
                    "\u{1B}[{}m[{:>5}][{}] {}: {}\u{1B}[0m\n",
                    color,
                    record.level(),
                    current_time,
                    record.target(),
                    record.args(),
                );
                println!("{}", format);
            }
        }
        //输出到文件
        if let Some(log_file) = &lock_log.log_file_config {
            log_file.paths.iter().for_each(|path| {
                //创建目录
                let dir_path = Path::new(path).parent().expect("Invalid file path");
                fs::create_dir_all(dir_path).unwrap();

                //创建文件
                let mut file = OpenOptions::new()
                    .create(true)
                    .read(true)
                    .append(true)
                    .write(true)
                    .open(path)
                    .unwrap();

                //将文件写入日志
                file.write_all(remove_ansi_escape_sequences(&format).as_bytes())
                    .unwrap();
            });
        }
    }

    fn flush(&self) {}
}

impl Logger {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn level(mut self, level: Level) -> Self {
        self.level = Some(level);
        self
    }
    pub fn log_file_config(mut self, log_file_config: LogFile) -> Self {
        self.log_file_config = Some(log_file_config);
        self
    }
    pub fn print(mut self, print: bool) -> Self {
        self.print = Some(print);
        self
    }

    /// .
    /// 日志初始化，
    /// # Panics
    ///
    /// Panics if .
    pub fn init(self) {
        let mut lock_log = LOG.lock().unwrap();
        lock_log.log_file_config = self.log_file_config.clone();
        lock_log.print = self.print;
        drop(lock_log);

        static AID: Aid = Aid;
        log::set_logger(&AID).unwrap();
        let max_level = match &self.level {
            Some(level) => match level {
                Level::Error => log::LevelFilter::Error,
                Level::Warn => log::LevelFilter::Warn,
                Level::Info => log::LevelFilter::Info,
                Level::Debug => log::LevelFilter::Debug,
                Level::Trace => log::LevelFilter::Trace,
            },
            None => log::LevelFilter::Info,
        };
        log::set_max_level(max_level);
        // 如果日志持久化配置不为None，则异步执行备份
        if let Some(log_file) = self.log_file_config {
            thread::Builder::new()
                .name("archivelog  thread".to_string())
                .spawn(|| LogFile::archive_log(log_file))
                .unwrap();
        }
    }
}
// 移除 ANSI 转义序列的辅助函数
fn remove_ansi_escape_sequences(input: &str) -> String {
    let re = Regex::new("\u{1B}\\[[0-9;]*[mK]").unwrap();
    re.replace_all(input, "").to_string()
}

#[cfg(test)]
mod log_test {
    use std::env;
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;

    use crate::Level;
    use crate::LogFileBuild;
    use crate::Logger;

    #[test]
    fn new_test() {
        let current_dir = env::current_dir().unwrap();
        let pkg_name = env!("CARGO_PKG_NAME");
        let log_path = format!("{}/logger/{}.log", current_dir.to_str().unwrap(), pkg_name);

        let new = Logger::new()
            .level(Level::Trace)
            .log_file_config(
                LogFileBuild::new()
                    .paths(vec![PathBuf::from(log_path)])
                    .archive(true)
                    .build(),
            )
            .print(true);
        new.init();
        (1..=10).for_each(|i| {
            log::info!("info:{}", i);
            log::warn!("warn:{}", i);
            log::error!("error:{}", i);
            log::debug!("debug:{}", i);
            log::trace!("trace:{}", i);
            thread::sleep(Duration::from_secs(3))
        });

        assert_eq!(1, 2)
    }
}
