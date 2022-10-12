use chrono::Local;
use clap::{Arg, Command as ClapCommand};
use env_logger::Builder;
use log::LevelFilter;
use notify::{raw_watcher, RawEvent, RecursiveMode, Watcher};
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::process::Command;
use std::sync::mpsc::channel;
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::broadcast,
};

// TODO: Use anyhow for error with the .context() method
const ARGS_EXTENSION: &str = "args";

// struct Copier {
//     binary_file_path: PathBuf,
//     binary_args_file_path: PathBuf,
// }

// impl Copier {
//     fn run_cmd(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//         // Check if the args file is present
//         let _binary_args_path = match self.binary_args_file_path.metadata() {
//             Ok(stat) => stat,
//             Err(e) => {
//                 log::error!("{}: {}", self.binary_args_file_path.display(), e);
//                 process::exit(1);
//             }
//         };

//         // Open the args file
//         let reader = BufReader::new(File::open(self.binary_args_file_path.to_owned()).unwrap());

//         // Build the args string
//         let mut args = vec![] as Vec<String>;
//         for line in reader.lines() {
//             for word in line.unwrap().split_whitespace() {
//                 args.push(word.to_string());
//             }
//         }

//         log::info!(
//             "running command \"{} {}\"",
//             self.binary_file_path.display(),
//             args.as_slice().join(" ").as_str()
//         );

//         let tries = 0;
//         while tries < 1000 {
//             let child = Command::new(self.binary_file_path.to_str().unwrap())
//                 .args(args)
//                 // .stdin(Stdio::piped())
//                 // .stdout(Stdio::piped())
//                 .spawn();

//             match child {
//                 Ok(mut child) => {
//                     // let stdout = child.stdout.take().unwrap();
//                     // let mut stdout = BufReader::new(stdout);
//                     // let mut line = String::new();
//                     // while stdout.read_line(&mut line).unwrap() > 0 {
//                     //     log::info!("{}", line.trim());
//                     // }

//                     log::info!("pid is {}", child.id());

//                     let status = child.wait().unwrap();
//                     if status.success() {
//                         log::info!("{} exited with success", self.binary_file_path.display());
//                         break;
//                     } else {
//                         log::error!(
//                             "{} exited with error code {}",
//                             self.binary_file_path.display(),
//                             status.code().unwrap()
//                         );
//                     }
//                 }
//                 Err(e) => match e.kind() {
//                     // Requires feature gate and not using stable channel...
//                     // std::io::ErrorKind::ExecutableFileBusy => {
//                     //     log::error!("{} not found", self.binary_file_path.display());
//                     //     continue;
//                     // }
//                     other_error => {
//                         log::error!("{:?}", other_error);
//                         process::exit(1);
//                     }
//                 },
//             }
//             break;
//         }

//         Ok(())
//     }
// }

async fn run_cmd(
    binary_file_path: PathBuf,
    binary_args_file_path: PathBuf,
    tx_copy: tokio::sync::broadcast::Sender<u32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        "running command \"{} {}\"",
        binary_file_path.display(),
        args.as_slice().join(" ").as_str()
    );

    let tries = 0;
    while tries < 1000 {
        let child = Command::new(binary_file_path.to_str().unwrap())
            .args(args)
            // .stdin(Stdio::piped())
            // .stdout(Stdio::piped())
            .spawn();

        match child {
            Ok(mut child) => {
                log::info!("pid is {}", child.id());

                // Send the PID to a broadcast channel
                tx_copy.send(child.id()).unwrap();

                let status = child.wait().unwrap();
                if status.success() {
                    log::info!("{} exited with success", binary_file_path.display());
                    break;
                } else {
                    log::error!(
                        "{} exited with error code {}",
                        binary_file_path.display(),
                        status.code().unwrap()
                    );
                }
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
        break;
    }

    Ok(())
}

async fn signal_handler() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // An infinite stream of hangup signals.
    let mut stream = signal(SignalKind::hangup())?;

    // Print whenever a HUP signal is received
    loop {
        stream.recv().await;
        println!("got signal HUP");
    }
}

#[tokio::main]
async fn main() {
    // Setup logger with nano seconds
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S:%f"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    // Create a joined handle to store the async functions
    let mut join_handles = vec![];
    join_handles.push(tokio::spawn(signal_handler()));
    join_handles.push(tokio::spawn(run()));

    // tokio::spawn( async move run());

    // Wait for all the tasks to complete
    let results = futures::future::join_all(join_handles).await;
    for result in results {
        if let Err(e) = result {
            log::error!("{}", e);
            process::exit(1)
        }
    }
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
                             .takes_value(true)
                             .required(true)
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

    let binary_file_path = Path::new(cmd.value_of("binary-path").unwrap());
    let _binary_file_stat = match binary_file_path.metadata() {
        Ok(stat) => stat,
        Err(e) => {
            log::error!("{}: {}", binary_file_path.display(), e);
            process::exit(1);
        }
    };
    let binary_file_base = binary_file_path.file_stem().unwrap();
    let binary_dir_path = binary_file_path.parent().unwrap();
    let binary_args_file_path = if cmd.value_of("arguments-file-path").is_none() {
        binary_file_path.with_extension(ARGS_EXTENSION)
    } else {
        cmd.value_of("arguments-file-path")
            .unwrap()
            .to_string()
            .into()
    };

    // let copier = Copier {
    //     binary_file_path: binary_file_path.to_path_buf(),
    //     binary_args_file_path: binary_args_file_path,
    // };

    log::info!("binary directory {:?} exists", binary_file_path);
    log::info!("binary_file_base is {:?}", binary_file_base);
    log::info!("binary_dir_path is {:?}", binary_dir_path);

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = raw_watcher(tx).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher
        .watch(binary_dir_path, RecursiveMode::NonRecursive)
        .unwrap();

    // Create a channel
    let (tx_copy, _rx_copy) = broadcast::channel::<u32>(5);

    loop {
        let tx_copy = tx_copy.clone();
        let mut rx_copy = tx_copy.subscribe();
        let event = rx.recv();
        match event {
            Ok(RawEvent {
                path: Some(path),
                op: Ok(op),
                cookie,
            }) => {
                if path == binary_file_path {
                    println!("{:?} {:?} ({:?})", op, path, cookie);

                    if op.contains(notify::op::CREATE | notify::op::CHMOD | notify::op::WRITE) {
                        log::info!("{:?} was created", path);
                        let a = binary_file_path.to_path_buf().clone();
                        let b = binary_args_file_path.to_path_buf().clone();

                        // Code racing, the child PID is never printed :(
                        // tokio::select! {
                        //     result = rx_copy.recv() => {
                        //         log::info!("CHILD PID IS {:?}", result);
                        //     }

                        //     result = run_cmd(a, b, tx_copy) => {
                        //         log::info!("RAN {:?}", result);
                        //     }

                        // }

                        // Code without race
                        // Watch for received messages in the channel
                        // tokio::spawn(async move {
                        //     let result = rx_copy.recv().await;

                        //     let result = match result {
                        //         Ok(result) => result,
                        //         Err(e) => {
                        //             log::error!("No result received {}", e);
                        //             process::exit(1);
                        //         }
                        //     };
                        //     log::info!("CHILD PID IS {:?}", result.to_string());
                        // });

                        tokio::spawn(async move {
                            log::info!("running cmd");
                            if let Err(e) = run_cmd(a, b, tx_copy).await {
                                log::error!("failed to run cmd: {}", e);
                            };
                        });

                        // let mut join_handles = vec![];
                        // join_handles.push(tokio::spawn(async move {
                        //     let result = rx_copy.recv().await;
                        //     log::info!("CHILD PID IS {:?}", result.unwrap());
                        // }));

                        // join_handles.push(tokio::spawn(run_cmd(a, b, tx_copy)));

                        // // Wait for all the tasks to complete
                        // let results = futures::future::join_all(join_handles).await;
                        // for result in results {
                        //     if let Err(e) = result {
                        //         log::error!("{}", e);
                        //         process::exit(1)
                        //     }
                        // }
                    }

                    if op.contains(notify::op::REMOVE) {
                        // The file was deleted
                        log::info!("{:?} was removed", path);
                        log::info!("stopping context");

                        tokio::spawn(async move {
                            let result = rx_copy.recv().await;

                            let result = match result {
                                Ok(result) => result,
                                Err(e) => {
                                    log::error!("No result received {}", e);
                                    process::exit(1);
                                }
                            };
                            log::info!("CHILD PID IS {:?}", result.to_string());

                            // Kill process

                            // result.kill().unwrap();
                        });

                        // Kill the process from the channel
                        // Get the last message from the channel
                        // let mut result = tx_copy.subscribe();

                        // Check channel capacity

                        // Send message to broadcast channel to kill the process
                    }
                }
            }
            Ok(event) => println!("{:?}", event),
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
