use std::{
    fs::File,
    io::{IsTerminal, Read, Seek, SeekFrom},
    path::Path,
    process,
    str::from_utf8,
};

use clap::{arg, command, Parser};
use colored::Colorize;
use notify::{RecursiveMode, Watcher};

use static_str::to_str;

enum Input {
    File(&'static Path),
    Stdin,
}

struct FileSpec {
    size: u64,
    input: Input,
}

impl FileSpec {
    fn new(input: Input) -> Self {
        let size: u64 = 0;
        let mut ret = Self { input, size };
        ret.update_size();
        ret
    }

    // size_on_disk() wasn't returning actual file size for linux.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn update_size(&mut self) {
        self.size = match self.input {
            Input::File(fpath) => fpath.metadata().unwrap().len(),
            _ => 0,
        };
    }

    #[cfg(target_os = "windows")]
    fn update_size(&mut self) {
        self.size = match self.input {
            Input::File(fpath) => fpath.unwrap().size_on_disk().unwrap(),
            _ => 0,
        }
    }

    fn read_last_n_lines(&mut self, num: i32) {
        let mut b_ns = num;
        let mut start: u64 = 0;

        match self.input {
            Input::File(path) => {
                let mut f = File::options().read(true).write(false).open(path).unwrap();

                // Stop when number of \n are met, or the file is completely read.
                while b_ns >= 0 && start < self.size {
                    // Read one byte at a time until we reach the specified number of \n's (b_ns)
                    start = start + 1;

                    f.seek(SeekFrom::Start(self.size - start)).unwrap();

                    let mut buf = vec![0; 1];
                    f.read_exact(&mut buf).unwrap();

                    if from_utf8(&buf).unwrap() == "\n".to_string() {
                        b_ns -= 1;
                    }
                }
                f.seek(SeekFrom::Start(self.size - start + 1)).unwrap();

                let mut buf_print = Vec::new();
                f.read_to_end(&mut buf_print).unwrap();

                print!("{}", String::from_utf8(buf_print).unwrap());
            }
            Input::Stdin => {
                if !std::io::stdin().is_terminal() {
                    let mut buf_print = String::new();
                    let stdin_lines = std::io::stdin();

                    for lines in stdin_lines
                        .lines()
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap()
                        .iter()
                        .rev()
                    {
                        buf_print.insert_str(0, &format!("{}\n", lines)[..]);
                        b_ns -= 1;
                        if b_ns < 0 {
                            break;
                        }

                        print!("{}", buf_print);
                    }
                }
            }
        };
    }

    fn follow_filter(&mut self, filter: &str) {
        match self.input {
            Input::File(fpath) => {
                if fpath.metadata().unwrap().len() >= self.size {
                    // Regular tail -f behaviour so far.
                    let mut f = File::options().read(true).write(false).open(fpath).unwrap();

                    // delay updating self.size so that we know exact seek of where the last output ended
                    f.seek(SeekFrom::Start(self.size)).unwrap();

                    let mut buf = Vec::new();
                    f.read_to_end(&mut buf).unwrap();
                    let new_line = String::from_utf8(buf).unwrap();

                    // Start filtering things out here...
                    let lines = new_line.split("\n");

                    for line in lines {
                        if line.contains(&filter) {
                            let phrases: Vec<&str> = line.split(filter).collect();
                            for i in phrases[..phrases.len() - 1].iter() {
                                print!("{}", *i);
                                print!("{}", filter.red());
                            }
                            println!("{}", phrases[phrases.len() - 1]);
                        }
                    }
                } else {
                    // Display message informing that the file size reduced since the last output. continue following new EOF
                    println!("***FILE TRUNCATED: READING FROM NEW EOF***");
                }

                // update self.size after we are done printing/filtering
                self.update_size();
            }
            Input::Stdin => {}
        }
    }
}

fn main() {
    let mut args = Args::parse();

    let input = match args.file {
        Some(file) => {
            let filepath = to_str(file);

            let p = Path::new(filepath);

            match p.try_exists() {
                Ok(true) => Input::File(p),
                Ok(false) => {
                    eprintln!("error: file doesn't exist");
                    process::exit(-1);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(-1);
                }
            }
        }
        None => Input::Stdin,
    };

    let mut fspec = FileSpec::new(input);
    let sieve = to_str(args.sieve);

    // Automatically follow if sieve is specified
    if sieve != "" {
        args.follow = true;
    }

    let num = match args.num_lines.parse::<i32>() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("illegal offset: {}", args.num_lines);
            std::process::exit(-1);
        }
    };

    fspec.read_last_n_lines(num);

    if let Input::File(path) = fspec.input {
        if args.follow {
            let mut watcher = notify::recommended_watcher(move |res| match res {
                Ok(_event) => fspec.follow_filter(sieve),
                Err(e) => println!("watch error: {:?}", e),
            })
            .unwrap();

            watcher.watch(path, RecursiveMode::Recursive).unwrap();
            loop {}
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    /// Follow a file for live changes.
    #[arg(short, long, action)]
    follow: bool,

    /// Phrase to filter new lines with. Will automatically enable [-f --follow]
    #[arg(short, long, default_value = "")]
    sieve: String,

    /// Number of lines from the end to tail.
    #[arg(short, long, default_value = "5")]
    num_lines: String,

    /// Path of the file to tail/follow.
    #[arg(default_value = None)]
    file: Option<String>,
}
