use clap::{Parser, Subcommand};
use libfritzer::{command::Device, Fritzbox};
use log::{debug, info, warn, Level};
use std::{
    env,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use url::Url;

#[derive(Parser, Debug)]
#[command(author = "fritzer", version = "0.1", about = "Use FRITZ!Box AHA interface", long_about = None)]
struct Args {
    /// Url of the FRITZ!Box
    #[arg(short, long)]
    url: Url,

    /// Path to the session file (default: ~/.fritzer.sid)
    #[arg(short, long, value_name = "FILE")]
    sid_path: Option<PathBuf>,

    /// FRITZ!Box user (default: last logged in user)
    #[arg(long)]
    username: Option<String>,

    /// Path to the file containing the password
    #[arg(short, long)]
    password: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Commands related to switches
    Switch {
        /// lists switches
        #[arg(short, long)]
        list: bool,
    },
}

async fn get_stored_sid(path: &Path) -> Option<String> {
    let mut sid = String::new();

    match path.exists() {
        true => {
            info!("Reading SID from file...");

            let file = File::open(path);
            let result = file.unwrap().read_to_string(&mut sid);

            match result {
                Ok(_) => Some(sid),
                Err(_) => None,
            }
        }
        false => None,
    }
}

async fn get_password(arg_password: &Option<String>) -> String {
    if arg_password.is_none() {
        rpassword::prompt_password("Your password: ").unwrap()
    } else {
        arg_password.as_ref().unwrap().to_string()
    }
}

async fn store_sid(fritzbox: &Fritzbox, path: &Path) -> Result<(), std::io::Error> {
    let session_info = fritzbox.session_info.as_ref().unwrap();
    let file = File::create(path);

    file.unwrap().write_all(session_info.sid.as_bytes())
}

async fn connect_to_fritzbox_with_credentials(
    fritzbox: &mut Fritzbox,
    username: &Option<String>,
    password: &Option<String>,
    path_to_stored_sid: &Path,
) {
    let session_info = fritzbox.session_info.as_ref().unwrap();
    let user = session_info
        .users
        .users
        .iter()
        .find(|u| u.last.is_some() && u.last.unwrap() == 1);
    let username = match user {
        Some(u) => u.username.clone(),
        None => {
            if username.is_none() {
                panic!("No username available.");
            }

            username.as_ref().unwrap().clone()
        }
    };
    let password = get_password(password).await;

    let result = fritzbox
        .connect_with_credentials(&username, &password)
        .await;

    if result.is_err() {
        panic!("Unable to connect to Fritzbox!");
    }

    let result = store_sid(&fritzbox, &path_to_stored_sid).await;

    if result.is_err() {
        warn!("Unable to cache SID.");
    }
}

async fn connect_to_fritzbox(
    url: &Url,
    username: &Option<String>,
    password: &Option<String>,
    sid_path: &Option<PathBuf>,
) -> Fritzbox {
    let mut fritzbox = Fritzbox::new(url.clone());

    let result = fritzbox.update_session_info().await;

    if result.is_err() {
        panic!(
            "Unable to receive session information from Fritzbox at {}!",
            url
        );
    }

    debug!("Session info: {:?}", fritzbox.session_info);
    let backup_path_to_stored_sid =
        PathBuf::from(format!("{}/.fritzer.sid", env::var("HOME").unwrap()));
    let path_to_stored_sid = sid_path.as_ref().unwrap_or(&backup_path_to_stored_sid);
    let stored_sid = get_stored_sid(&path_to_stored_sid).await;

    match stored_sid {
        None => {
            info!("No cached SID available. Request new SID...");

            connect_to_fritzbox_with_credentials(
                &mut fritzbox,
                username,
                password,
                &path_to_stored_sid,
            )
            .await;
        }
        Some(sid) => {
            let result = fritzbox.connect_with_sid(&sid).await;

            if result.is_err() {
                debug!(
                    "Could not validate SID due to the following error {:?}",
                    result
                );

                connect_to_fritzbox_with_credentials(
                    &mut fritzbox,
                    username,
                    password,
                    &path_to_stored_sid,
                )
                .await;
            } else {
                let is_connected = result.unwrap();

                if is_connected {
                    info!("Cached SID still valid. Re-use...");
                } else {
                    info!("Cached SID invalid. Request new SID...");

                    connect_to_fritzbox_with_credentials(
                        &mut fritzbox,
                        username,
                        password,
                        &path_to_stored_sid,
                    )
                    .await;
                }
            }
        }
    };

    fritzbox
}

async fn list_devices(devices: &Vec<Device>) {
    println!("| {0: <2} | {1: <12} | {2: <10} |", "Nr", "AIN", "Name");
    println!("+----+--------------+------------+");
    for (i, device) in devices.iter().enumerate() {
        println!(
            "| {0: <2} | {1: <12} | {2: <10} |",
            i, device.ain, device.name
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(Level::Debug).unwrap();

    let args = Args::parse();
    let fritzbox =
        connect_to_fritzbox(&args.url, &args.username, &args.password, &args.sid_path).await;
    let session_info = fritzbox.session_info.as_ref().unwrap();

    debug!("The SID {:?}", session_info.sid);

    match &args.command {
        Some(Commands::Switch { list }) => {
            if *list {
                debug!("List switches...");

                let switches = fritzbox.get_switches().await.unwrap();

                list_devices(&switches).await;
            }
        }
        None => {}
    }

    Ok(())
}
