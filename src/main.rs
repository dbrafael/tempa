mod directories;
mod template;

use std::sync::Arc;

use directories::DirectoryFiles;
use tokio::task::JoinSet;

const DEBUG: bool = false;

#[derive(Debug)]
enum Error {
    DirectoryCreateError,
    FileReadError,
    FileWriteError,
    FileCopyError,
    FileCreateError,
    ArgumentsNoInputError,
    ArgumentsNoOutputError,
    ArgumentsNoReplacementsError,
    ReplacementsReadError,
    PrepareCleanOutputError,
}

#[derive(Clone, Debug)]
struct ProgramArgs {
    input: String,
    output: String,
    open: String,
    close: String,
    replacements: yaml_rust::Yaml,
}

impl TryFrom<getopts::Matches> for ProgramArgs {
    type Error = Error;
    fn try_from(value: getopts::Matches) -> Result<Self, Self::Error> {
        let input = value.opt_str("i").ok_or(Error::ArgumentsNoInputError)?;
        let output = value.opt_str("o").ok_or(Error::ArgumentsNoOutputError)?;
        let replacements = value
            .opt_str("r")
            .ok_or(Error::ArgumentsNoReplacementsError)?;

        let open = value.opt_str("s").unwrap_or("%".to_string());
        let close = value.opt_str("c").unwrap_or(open.clone());

        let replacements = yaml_rust::YamlLoader::load_from_str(
            &std::fs::read_to_string(replacements).expect("Error reading replacements"),
        )
        .map_err(|_| Error::ReplacementsReadError)?;

        Ok(ProgramArgs {
            input,
            output,
            open,
            close,
            replacements: replacements[0].to_owned(),
        })
    }
}

fn print_usage(program: &str, opts: getopts::Options) {
    let brief = format!("Usage: {program} -i IN_DIR -o OUT_DIR -r requirements.yaml");
    print!("{}", opts.usage(&brief));
}

fn setup_getopts(options: &mut getopts::Options) {
    options.reqopt("i", "input", "input directory", "DIR");
    options.reqopt("o", "output", "output directory", "DIR");
    options.optopt(
        "s",
        "separator-open",
        "opening separator (default %)",
        "SEP",
    );
    options.optopt(
        "c",
        "separator-close",
        "closing separator (same as opening separator if not specified)",
        "SEP",
    );
    options.reqopt("r", "replacements", "replacements file (yaml)", "FILE");
    options.optflag("h", "help", "print help menu");
}

fn prepare(args: &ProgramArgs) -> Result<(), Error> {
    println!("Cleaning output directory");
    std::fs::remove_dir_all(&args.output).map_err(|_| Error::PrepareCleanOutputError)
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let program = args[0].clone();

    let mut opts = getopts::Options::new();
    setup_getopts(&mut opts);

    let matches = match opts.parse(&args[1..]) {
        Ok(opts) => opts,
        Err(e) => {
            print_usage(&program, opts);
            panic!("{e}");
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let args: ProgramArgs = matches.try_into().expect("Error parsing arguments.");
    prepare(&args).expect("Error during prepare stage.");

    let files = DirectoryFiles::child_files_recursive(&args.input, &args.output)
        .expect("Error reading input directory");

    let args = Arc::new(args);
    let mut parsed_count = 0;
    let total_files = files.len();
    let mut handles: JoinSet<_> = files
        .into_iter()
        .map(|task| (task, args.clone()))
        .map(|(task, args)| async move {
            match task.execute(&args) {
                Ok((replacements, from, to)) => {
                    let Some(to) = to else {
                        if DEBUG {
                            println!("Skipped {from:?}");
                        }
                        return 1;
                    };
                    if replacements > 0 && DEBUG {
                        println!(
                            "Parsed {from:?} into {to:?} applying {replacements} replacements."
                        );
                    } else if replacements > 0 {
                        println!("Parsed {from:?}");
                    } else if DEBUG {
                        println!("Copied {from:?} into {to:?}.");
                    }
                    return 1;
                }
                Err((file, e)) => {
                    eprint!("Error parsing file {file:?}: {e:?}");
                    return 0;
                }
            }
        })
        .collect();

    while let Some(res) = handles.join_next().await {
        if let Ok(add) = res {
            parsed_count += add;
        }
    }

    println!("Finished cloning directory, {parsed_count}/{total_files} files parsed.");
    if parsed_count != total_files {
        eprintln!(
            "Operation finished but {} files were skipped",
            total_files - parsed_count
        );
    }
}
