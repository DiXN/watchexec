use std::path::MAIN_SEPARATOR;
use std::process::Command;

use clap::{App, Arg};

#[derive(Debug)]
pub struct Args {
    pub paths: Vec<String>,
    pub filters: Vec<String>,
    pub ignores: Vec<String>,
    pub clear_screen: bool,
    pub debounce: u64,
    pub debug: bool,
    pub run_initially: bool,
    pub no_shell: bool,
    pub no_vcs_ignore: bool,
    pub once: bool,
    pub poll: bool,
    pub poll_interval: u32,
}

#[cfg(target_family = "windows")]
pub fn clear_screen() {
    let _ = Command::new("cmd").arg("/c").arg("cls").status();
}

#[cfg(target_family = "unix")]
pub fn clear_screen() {
    let _ = Command::new("tput").arg("reset").status();
}

#[allow(unknown_lints)]
#[allow(or_fun_call)]
pub fn get_args() -> Args {
    let args = App::new("watchexec")
        .version(crate_version!())
        .about("Execute commands when watched files change")
        .arg(Arg::with_name("extensions")
                 .help("Comma-separated list of file extensions to watch (js,css,html)")
                 .short("e")
                 .long("exts")
                 .takes_value(true))
        .arg(Arg::with_name("path")
                 .help("Watch a specific directory")
                 .short("w")
                 .long("watch")
                 .number_of_values(1)
                 .multiple(true)
                 .takes_value(true))
        .arg(Arg::with_name("clear")
                 .help("Clear screen before executing command")
                 .short("c")
                 .long("clear"))
        .arg(Arg::with_name("restart")
                 .help("Restart the process if it's still running")
                 .short("r")
                 .long("restart"))
        .arg(Arg::with_name("debounce")
                 .help("Set the timeout between detected change and command execution, defaults to 500ms")
                 .takes_value(true)
                 .value_name("milliseconds")
                 .short("d")
                 .long("debounce"))
        .arg(Arg::with_name("verbose")
                 .help("Print debugging messages to stderr")
                 .short("v")
                 .long("verbose"))
        .arg(Arg::with_name("filter")
                 .help("Ignore all modifications except those matching the pattern")
                 .short("f")
                 .long("filter")
                 .number_of_values(1)
                 .multiple(true)
                 .takes_value(true)
                 .value_name("pattern"))
        .arg(Arg::with_name("ignore")
                 .help("Ignore modifications to paths matching the pattern")
                 .short("i")
                 .long("ignore")
                 .number_of_values(1)
                 .multiple(true)
                 .takes_value(true)
                 .value_name("pattern"))
        .arg(Arg::with_name("no-vcs-ignore")
                 .help("Skip auto-loading of .gitignore files for filtering")
                 .long("no-vcs-ignore"))
        .arg(Arg::with_name("no-default-ignore")
                 .help("Skip auto-ignoring of commonly ignored globs")
                 .long("no-default-ignore"))
        .arg(Arg::with_name("poll")
                 .help("Forces polling mode")
                 .long("force-poll")
                 .value_name("interval"))
        .arg(Arg::with_name("once").short("1").hidden(true))
        .get_matches();

    let paths = values_t!(args.values_of("path"), String).unwrap_or(vec![String::from(".")]);

    let mut filters = values_t!(args.values_of("filter"), String).unwrap_or(vec![]);

    if let Some(extensions) = args.values_of("extensions") {
        for exts in extensions {
            filters.extend(
                exts.split(',')
                    .filter(|ext| !ext.is_empty())
                    .map(|ext| format!("*.{}", ext.replace(".", ""))),
            );
        }
    }

    let mut ignores = vec![];
    let default_ignores = vec![
        format!("**{}.DS_Store", MAIN_SEPARATOR),
        String::from("*.py[co]"),
        String::from("#*#"),
        String::from(".#*"),
        String::from(".*.sw?"),
        String::from(".*.sw?x"),
        format!("**{}.git{}**", MAIN_SEPARATOR, MAIN_SEPARATOR),
        format!("**{}.hg{}**", MAIN_SEPARATOR, MAIN_SEPARATOR),
        format!("**{}.svn{}**", MAIN_SEPARATOR, MAIN_SEPARATOR),
    ];

    if args.occurrences_of("no-default-ignore") == 0 {
        ignores.extend(default_ignores)
    };
    ignores.extend(values_t!(args.values_of("ignore"), String).unwrap_or(vec![]));

    let poll_interval = if args.occurrences_of("poll") > 0 {
        value_t!(args.value_of("poll"), u32).unwrap_or_else(|e| e.exit())
    } else {
        1000
    };

    let debounce = if args.occurrences_of("debounce") > 0 {
        value_t!(args.value_of("debounce"), u64).unwrap_or_else(|e| e.exit())
    } else {
        500
    };

    Args {
        paths: paths,
        filters: filters,
        ignores: ignores,
        clear_screen: args.is_present("clear"),
        debounce: debounce,
        debug: args.is_present("verbose"),
        run_initially: !args.is_present("postpone"),
        no_shell: args.is_present("no-shell"),
        no_vcs_ignore: args.is_present("no-vcs-ignore"),
        once: args.is_present("once"),
        poll: args.occurrences_of("poll") > 0,
        poll_interval: poll_interval,
    }
}
