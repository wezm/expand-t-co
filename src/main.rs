#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, stdin, BufRead, BufReader};

use regex::{Captures, Regex};

lazy_static! {
    static ref TCO_RE: Regex = Regex::new(r"https://t\.co/[a-zA-Z0-9]+").unwrap();
}

fn main() -> io::Result<()> {
    let mut seen = HashMap::new();

    for path in env::args_os().skip(1) {
        if path == "-" {
            process_path(stdin(), &mut seen)?;
        } else {
            process_path(File::open(path)?, &mut seen)?;
        };
    }

    Ok(())
}

fn process_path<R: io::Read>(read: R, seen: &mut HashMap<String, String>) -> io::Result<()> {
    let reader = BufReader::new(read);
    let agent = ureq::agent();
    agent.set_max_pool_connections_per_host(10);

    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                eprintln!("error reading line: {}", err);
                continue;
            }
        };

        let result = TCO_RE.replace_all(&line, |caps: &Captures| match seen.get(&caps[0]) {
            Some(location) => location.to_string(),
            None => {
                let resp = agent
                    .head(&caps[0])
                    .redirects(0)
                    .timeout(std::time::Duration::from_secs(10))
                    .call();
                let location = if resp.redirect() {
                    match resp.header("location") {
                        Some(location) => location.to_string(),
                        None => {
                            eprintln!("{} missing Location header", &caps[0]);
                            caps[0].to_string()
                        }
                    }
                } else {
                    dbg!(&resp);
                    let error = resp
                        .synthetic_error()
                        .as_ref()
                        .map(|err| err.status_text())
                        .unwrap_or("unknown");
                    eprintln!("unable to fetch {}: {}", &caps[0], error);
                    caps[0].to_string()
                };
                seen.insert(caps[0].to_string(), location);
                seen[&caps[0]].to_string()
            }
        });
        println!("{}", result);
    }

    Ok(())
}
