use std::{
    fs::File,
    io::{self, IsTerminal, Read, Seek, SeekFrom, Stdin},
    path::Path,
    str::from_utf8,
};

use clap::{arg, command, Parser};
use colored::Colorize;
use notify::{RecursiveMode, Watcher};

// use filesize::PathExt;

use static_str::to_str;

struct FileSpec {
    size: u64,
    fpath: Option<&'static Path>,
    stdin: Option<Vec<String>>,
}

impl FileSpec {
    fn new(fpath: Option<&'static Path>, stdin: Option<Vec<String>>) -> Self {
        let size: u64 = 0;
        let mut ret = Self { fpath, size, stdin };
        ret.update_size();
        ret
    }

    // size_on_disk() wasn't returning actual file size for linux.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn update_size(&mut self) {
        self.size = if self.fpath.is_some() {
            println!("{:#?}", self.fpath.unwrap());
            self.fpath.unwrap().metadata().unwrap().len()
        } else {
            0
        }
        // self.size = self.fpath.unwrap().metadata().unwrap().len();
    }

    #[cfg(target_os = "windows")]
    fn update_size(&mut self) {
        self.size = self.fpath.unwrap().size_on_disk().unwrap();
    }
}

fn main() {
    let mut args = Args::parse();

    let filepath = to_str(args.file);

    let mut _stdin: Vec<String> = [].to_vec();

    let input = std::io::stdin();

    let stdin_lines: Option<Vec<String>>;

    if !input.is_terminal() {
        stdin_lines = Some(input.lines().collect::<Result<Vec<_>, _>>().unwrap());
    } else {
        stdin_lines = None;
    }

    let sieve = to_str(args.sieve);

    // Automatically follow if sieve is specified
    if sieve != "" {
        args.follow = true;
    }

    let path = if filepath != "" {
        Some(Path::new(filepath))
    } else {
        None
    };

    let mut fspec = FileSpec::new(path, stdin_lines);

    let num = args.num_lines.parse::<i32>().unwrap();
    read_last_n_lines(&mut fspec, num);

    if args.follow && path.is_some() {
        let mut watcher = notify::recommended_watcher(move |res| match res {
            Ok(_event) => follow_filter(&mut fspec, sieve),
            Err(e) => println!("watch error: {:?}", e),
        })
        .unwrap();

        watcher
            .watch(path.unwrap(), RecursiveMode::Recursive)
            .unwrap();
        loop {}
    }
}

fn read_last_n_lines(file: &mut FileSpec, num: i32) {
    let mut b_ns = num;
    let mut start: u64 = 0;

    if file.fpath.is_some() {
        let mut f = File::options()
            .read(true)
            .write(false)
            .open(file.fpath.unwrap())
            .unwrap();

        // Stop when number of \n are met, or the file is completely read.
        while b_ns > 0 && start < file.size {
            // Read one byte at a time until we reach the specified number of \n's (b_ns)
            start = start + 1;

            f.seek(SeekFrom::Start(file.size - start)).unwrap();

            let mut buf = vec![0; 1];
            f.read_exact(&mut buf).unwrap();

            if from_utf8(&buf).unwrap() == "\n".to_string() {
                b_ns -= 1;
            }
        }

        // Seek to position of last \n and print rest of the file out.
        f.seek(SeekFrom::Start(file.size - start)).unwrap();

        let mut buf_print = Vec::new();
        f.read_to_end(&mut buf_print).unwrap();

        print!("{}", String::from_utf8(buf_print).unwrap());
    } else if file.stdin.is_some() {
        // Far easier to do when input is stdin string...

        let mut buf_print = String::new();

        for lines in file.stdin.clone().unwrap().iter().rev() {
            buf_print.insert_str(0, &format!("{}\n", lines)[..]);
            b_ns -= 1;
            if b_ns == 0 {
                break;
            }
        }

        print!("{}", buf_print);
    }
}

fn follow_filter(file: &mut FileSpec, filter: &str) {
    if file.fpath.unwrap().metadata().unwrap().len() >= file.size {
        // Regular tail -f behaviour so far.
        let mut f = File::options()
            .read(true)
            .write(false)
            .open(file.fpath.unwrap())
            .unwrap();

        // delay updating file.size so that we know exact seek of where the last output ended
        f.seek(SeekFrom::Start(file.size)).unwrap();

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
        println!("***FILE TRUN'ED***");
    }

    // update file.size after we are done printing/filtering
    file.update_size();
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
    #[arg(default_value = "")]
    file: String,
}
