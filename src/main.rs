use clap::{Arg, Command as ClapCommand};
use env_logger::Builder;
use log::LevelFilter;
use std::fs::File;
use std::io::Error;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;
use std::process;
use tokio::{
    process::Child,
    process::Command,
    signal::unix::{signal, SignalKind},
    sync::broadcast,
};

// TODO: Use anyhow for error with the .context() method
const ARGS_EXTENSION: &str = "args";

fn run_cmd(binary_file_path: &PathBuf, binary_args_file_path: &PathBuf) -> Result<Child, Error> {
    // Check if the args file is present
    let _binary_args_path = match binary_args_file_path.metadata() {
        Ok(stat) => stat,
        Err(e) => {
            log::error!("{}: {}", binary_args_file_path.display(), e);
            process::exit(1);
        }
    };

    // Open the args file
    let reader = BufReader::new(File::open(binary_args_file_path.to_owned()).unwrap());

    // Build the args string
    let mut args = vec![] as Vec<String>;
    for line in reader.lines() {
        for word in line.unwrap().split_whitespace() {
            args.push(word.to_string());
        }
    }

    log::info!(
        "running command '{} {}'",
        binary_file_path.file_name().unwrap().to_str().unwrap(),
        args.as_slice().join(" ").as_str()
    );

    let tries = 0;
    // We try multiple times since the file might be busy
    // This happens if the file is being written to (e.g. go build)
    while tries < 1000 {
        let child = Command::new(binary_file_path.to_str().unwrap())
            .args(args)
            .spawn();

        match child {
            // if ok => return child so that we can kill it later
            Ok(child) => {
                log::debug!(
                    "program '{}' pid is '{}'",
                    binary_file_path.file_name().unwrap().to_str().unwrap(),
                    child.id().unwrap()
                );
                return Ok(child);
            }
            Err(e) => match e.kind() {
                // Requires feature gate and not using stable channel...
                // std::io::ErrorKind::ExecutableFileBusy => {
                //     log::error!("{} not found", self.binary_file_path.display());
                //     continue;
                // }
                other_error => {
                    log::error!("{:?}", other_error);
                    process::exit(1);
                }
            },
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to spawn child process",
    ))
}

// This async function checks if the binary file has changed every second
// If it has, it notifies the channel
// We don't use inotify here, since we are only watching over a single file
// We could potentially increase the interval value too
async fn file_changed(
    binary_file_path: PathBuf,
    tx_copy: broadcast::Sender<u32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        // Stat the file while entering the loop
        let initial_path_meta = std::fs::metadata(&binary_file_path);

        // Sleep for 1 second
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Stat the file again after 1 second
        let path_after_one_sec_meta = std::fs::metadata(&binary_file_path);

        // If the file doesn't exist, it's fine, it might be that it's being removed or created
        // Precisely check for the is not found error
        match (initial_path_meta, path_after_one_sec_meta) {
            (Ok(initial_meta), Ok(after_one_sec_meta)) => {
                // if the elapsed time is different, it means that the file has changed
                if initial_meta.created().unwrap().elapsed().unwrap().as_secs()
                    != after_one_sec_meta
                        .created()
                        .unwrap()
                        .elapsed()
                        .unwrap()
                        .as_secs()
                {
                    log::info!(
                        "file {} changed, notifying channel for reload",
                        binary_file_path.display()
                    );
                    // notifying the channel
                    match tx_copy.send(1) {
                        Ok(_) => {}
                        Err(e) => log::error!("Failed to send message to the channel: {}", e),
                    }
                }
            }
            (Err(e), _) | (_, Err(e)) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    return Err(Box::new(e));
                } else {
                    log::error!(
                        "file {} doesn't exist, but the program might be running, will reload it once the new program exists",
                        binary_file_path.display()
                    );
                    continue;
                }
            }
        }
    }
}

// This small handler is used to catch the interruptuption signals
async fn signal_handler() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut hangup = signal(SignalKind::hangup())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    tokio::select! {
        _ = sigint.recv() => {},
        _ = sigterm.recv() => {},
        _ = hangup.recv() => {},
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    // Setup logger
    Builder::new().filter(None, LevelFilter::Info).init();

    let (first, second) = tokio::join!(signal_handler(), run(),);
    first.unwrap();
    second.unwrap();
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cmd = ClapCommand::new("example")
                        .version("1.0")
                        .about("clap example")
                        .arg(Arg::new("binary-path")
                             .short('b')
                             .long("binary-path")
                             .value_name("binary path")
                             .help("Path of the binary to execute.")
                             .required(true)
                             .num_args(0..)

                        )
                        .arg(Arg::new("arguments-file-path")
                             .short('a')
                             .long("arguments-file-path")
                             .value_name("arguments file path")
                             .help("Arguments to pass to the binary execution")
                             .long_help("Arguments to pass to the binary while executing it (defaults to the binary path suffixed with .args")
                             .required(false)
                        )
                        .get_matches();

    let binary_file_path = Path::new(cmd.get_one::<String>("binary-path").unwrap());
    let _binary_file_stat = match binary_file_path.metadata() {
        Ok(_) => {}
        Err(e) => {
            log::error!("{}: {}", binary_file_path.display(), e);
            process::exit(1);
        }
    };
    let binary_args_file_path = if cmd.get_one::<String>("arguments-file-path").is_none() {
        binary_file_path.with_extension(ARGS_EXTENSION)
    } else {
        cmd.get_one::<String>("arguments-file-path").unwrap().into()
    };

    // Create a brodcast channel to send and receive the child PID
    let (tx, _rx): (broadcast::Sender<u32>, broadcast::Receiver<u32>) = broadcast::channel(10);
    let mut rx = tx.subscribe();

    // Spawn the file watcher
    tokio::spawn(file_changed(binary_file_path.to_path_buf(), tx));

    // Running a loop that acts as a watcher for the binary file
    loop {
        // Run the binary
        let mut child = run_cmd(&binary_file_path.to_path_buf(), &binary_args_file_path).unwrap();

        tokio::select! {
            // Main program handler for the interrupt signal
            _ = signal_handler() => {
            log::info!("received signal for program '{}', bye now!",binary_file_path.file_name().unwrap().to_str().unwrap());
            child.kill().await.expect("kill failed");
            process::exit(0);
            },

            // Child process handler once the program is done
            _ = child.wait() => {
                log::info!("program '{}' exited", binary_file_path.file_name().unwrap().to_str().unwrap());
            },

            // the binary was reloaded, so we kill the child process
            _ = rx.recv() => {
            log::info!("received termination request, killing pid {}", child.id().unwrap());
            child.kill().await.expect("kill failed");
            },
        }
    }
}
