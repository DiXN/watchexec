use std::collections::HashMap;
use std::fs::canonicalize;
use std::io::Write;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use cli;
use env_logger;
use error::{Error, Result};
use gitignore;
use log;
use notification_filter::NotificationFilter;
use pathop::PathOp;
use watcher::{Event, Watcher};

fn init_logger(debug: bool) {
    let mut log_builder = env_logger::Builder::new();
    let level = if debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Warn
    };

    log_builder
        .format(|buf, r| writeln!(buf, "*** {}", r.args()))
        .filter(None, level)
        .init();
}

pub fn run<F>(args: cli::Args, cb: F) -> Result<()> where F: Fn(Vec<PathOp>) {
    init_logger(args.debug);

    let mut paths = vec![];
    for path in args.paths {
        paths.push(canonicalize(&path).map_err(|e| Error::Canonicalization(path, e))?);
    }

    let gitignore = gitignore::load(if args.no_vcs_ignore { &[] } else { &paths });
    let filter = NotificationFilter::new(args.filters, args.ignores, gitignore)?;

    let (tx, rx) = channel();
    let (poll, poll_interval) = (args.poll, args.poll_interval).clone();

    let watcher = Watcher::new(tx.clone(), &paths, args.poll, args.poll_interval).or_else(|err| {
        if poll {
            return Err(err);
        }

        #[cfg(target_os = "linux")]
        {
            use nix::libc;
            let mut fallback = false;
            if let notify::Error::Io(ref e) = err {
                if e.raw_os_error() == Some(libc::ENOSPC) {
                    warn!("System notification limit is too small, falling back to polling mode. For better performance increase system limit:\n\tsysctl fs.inotify.max_user_watches=524288");
                    fallback = true;
                }
            }

            if fallback {
                return Watcher::new(tx, &paths, true, poll_interval);
            }
        }

        Err(err)
    })?;

    if watcher.is_polling() {
        warn!("Polling for changes every {} ms", args.poll_interval);
    }

    loop {
        debug!("Waiting for filesystem activity");
        cb(wait_fs(&rx, &filter, args.debounce));

        // Handle once option for integration testing
        if args.once {
            break;
        }
    }

    Ok(())
}

fn wait_fs(rx: &Receiver<Event>, filter: &NotificationFilter, debounce: u64) -> Vec<PathOp> {
    let mut paths = vec![];
    let mut cache = HashMap::new();

    loop {
        let e = rx.recv().expect("error when reading event");

        if let Some(ref path) = e.path {
            let pathop = PathOp::new(path, e.op.ok(), e.cookie);
            // Ignore cache for the initial file. Otherwise, in
            // debug mode it's hard to track what's going on
            let excluded = filter.is_excluded(path);
            if !cache.contains_key(&pathop) {
                cache.insert(pathop.clone(), excluded);
            }

            if !excluded {
                paths.push(pathop);
                break;
            }
        }
    }

    // Wait for filesystem activity to cool off
    let timeout = Duration::from_millis(debounce);
    while let Ok(e) = rx.recv_timeout(timeout) {
        if let Some(ref path) = e.path {
            let pathop = PathOp::new(path, e.op.ok(), e.cookie);
            if cache.contains_key(&pathop) {
                continue;
            }

            let excluded = filter.is_excluded(path);

            cache.insert(pathop.clone(), excluded);

            if !excluded {
                paths.push(pathop);
            }
        }
    }

    paths
}