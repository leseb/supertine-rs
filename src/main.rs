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

fn run_cmd(binary_file_path: PathBuf, binary_args_file_path: PathBuf) -> Result<Child, Error> {
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
    while tries < 1000 {
        let child = Command::new(binary_file_path.to_str().unwrap())
            .args(args)
            .spawn();

        match child {
            // if ok => return child so that we can kill it later
            Ok(child) => {
                log::info!(
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

async fn file_changed(
    binary_file_path: PathBuf,
    tx_copy: broadcast::Sender<u32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let initial_path = binary_file_path.to_path_buf().clone();
        let initial_path_meta = std::fs::metadata(initial_path);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let initial_path_after_one_sec_meta = binary_file_path.to_path_buf().clone();
        let path_after_one_sec_meta = std::fs::metadata(initial_path_after_one_sec_meta);

        // If files don't exist, it's fine, it might be that it's being removed or created
        if initial_path_meta.is_err() || path_after_one_sec_meta.is_err() {
            log::info!("file doesn't exist, try again");
            continue;
        }
        // Check if both files have the same inode
        if initial_path_meta
            .unwrap()
            .created()
            .unwrap()
            .elapsed()
            .unwrap()
            .as_secs()
            != path_after_one_sec_meta
                .unwrap()
                .created()
                .unwrap()
                .elapsed()
                .unwrap()
                .as_secs()
        {
            log::info!("file has changed, notifying channel");
            // notifying the channel
            let _ = tx_copy.send(1);
        }
    }
}

async fn signal_handler() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // An infinite stream of hangup signals.
    let mut sigint_stream = signal(SignalKind::interrupt())?;

    // Print whenever a interrupt signal is received
    loop {
        sigint_stream.recv().await;
        log::info!("bye now!");
        break;
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
        Ok(stat) => stat,
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

    // Create a brodcast channel to send an receive the child PID
    let (tx_copy, _rx_copy): (broadcast::Sender<u32>, broadcast::Receiver<u32>) =
        broadcast::channel(10);

    let mut _rx_copy = tx_copy.subscribe();

    tokio::spawn(file_changed(
        binary_file_path.to_path_buf().clone(),
        tx_copy,
    ));

    loop {
        let a = binary_file_path.to_path_buf().clone();
        let b = binary_args_file_path.to_path_buf().clone();

        // Run the binary
        let mut child = run_cmd(a, b).unwrap();

        tokio::select! {
            // Main program handler for the interrupt signal
            _ = signal_handler() => {
            log::info!("received interrupt signal for program '{}',",binary_file_path.file_name().unwrap().to_str().unwrap());
            child.kill().await.expect("kill failed");
            process::exit(0);
            },

            // Child process handler once the program is done
            _ = child.wait() => {
                log::info!("child exited");
            },

            // the binary was reloaded, so we kill the child process
            _ = _rx_copy.recv() => {
            log::info!("received termination request, killing pid {}", child.id().unwrap());
            child.kill().await.expect("kill failed");
            },
        }
    }
}
