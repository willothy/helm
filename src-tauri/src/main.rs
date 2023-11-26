// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    os::unix::prelude::OsStrExt,
    process::{Child, Command, Stdio},
};

use anyhow::Result;

use helm_shared::LoginResult;
use pam::{Client, Conversation, PasswordConv};
use tauri::Manager;
use users::os::unix::UserExt;

// This is a hack to get around the fact that we can't pass the X server PID
// to the signal handler.
static mut X_PID: libc::pid_t = 0;
const START_X: bool = true;
const DISPLAY: &str = ":2";
const VT: &str = "vt01";

fn init_env<'a>(client: &mut Client<'a, impl Conversation>, user: &users::User) -> Result<()> {
    let xauthority = user.home_dir().join(".Xauthority");
    client.set_env("XAUTHORITY", &xauthority.to_string_lossy())?;
    client.set_env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/bin")?;

    Ok(())
}

fn login<'a>(
    window: tauri::Window,
    username: &str,
    password: &str,
) -> Result<(Client<'a, impl Conversation>, Child)> {
    let mut client = Client::with_password("system-auth")
        .map_err(|e| anyhow::anyhow!("{e}: Failed to initialize client"))?;
    client
        .conversation_mut()
        .set_credentials(username, password);
    client
        .authenticate()
        .map_err(|e| anyhow::anyhow!("{e}: Failed to authenticate"))?;
    client
        .open_session()
        .map_err(|e| anyhow::anyhow!("{e}: Failed to open session"))?;

    let user = users::get_user_by_name(username)
        .ok_or_else(|| anyhow::anyhow!("User {} does not exist", username))?;

    init_env(&mut client, &user)?;
    window.set_closable(true)?;
    window.close()?;

    let child = Command::new(user.shell())
        .arg("-c")
        // .arg("exec /bin/bash --login .xinitrc")
        .arg("exec awesome")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // let child_pid = unsafe { libc::fork() };
    // if child_pid == 0 {
    //     std::env::set_current_dir(user.home_dir())?;
    //     use std::ffi::CString;
    //     let shell = CString::new(&*user.shell().to_string_lossy())?;
    //
    //     unsafe {
    //         libc::execl(
    //             shell.as_ptr(),
    //             shell.as_ptr(),
    //             CString::new("-c")?.as_ptr(),
    //             CString::new("exec /bin/bash --login .xinitrc")?.as_ptr(),
    //             std::ptr::null::<()>(),
    //         );
    //         println!("Failed to exec");
    //         libc::exit(1);
    //     };
    // }

    Ok((client, child))
}

fn logout<'a, C: Conversation>(mut client: Client<'a, C>) {
    client.close_on_drop = true;
}

fn start_x_server(display: &str, vt: &str) -> Result<libc::pid_t> {
    let x_server_pid = unsafe { libc::fork() };
    if x_server_pid == 0 {
        use std::ffi::CString;
        unsafe {
            let bash = CString::new("/bin/bash")?;
            let cmd = CString::new(format!("/usr/bin/X {display} {vt}").as_bytes())?;
            libc::execl(
                bash.as_ptr(),
                bash.as_ptr(),
                CString::new("-c")?.as_ptr(),
                cmd.as_ptr(),
                std::ptr::null::<()>(),
            );
            libc::exit(1);
        };
    } else {
        unsafe { libc::sleep(1) };
    }
    Ok(x_server_pid)
}

fn stop_x_server() {
    unsafe {
        if X_PID != 0 {
            libc::kill(X_PID, libc::SIGTERM);
        }
    }
}

fn signal_handler(_: libc::c_int) {
    stop_x_server();
}

#[tauri::command]
fn try_login(window: tauri::Window, username: &str, password: &str) -> LoginResult {
    // Tauri seems to have a bug where it doesn't properly handle
    // returning a Result from a command handler, so we have to
    // manually handle the Result here with the `LoginResult` struct.
    let (mut client, mut child) = match login(window, username, password) {
        Ok(session) => session,
        Err(e) => {
            return LoginResult {
                success: false,
                message: Some(e.to_string()),
            }
        }
    };

    client.set_env("DISPLAY", DISPLAY).ok();

    // Wait for the child process to exit
    child.wait().ok();

    logout(client);

    if START_X {
        stop_x_server();
    }

    LoginResult {
        success: true,
        message: Some(child.id().to_string()),
    }
}

fn main() {
    if START_X {
        unsafe {
            libc::signal(libc::SIGSEGV, signal_handler as usize);
            libc::signal(libc::SIGTRAP, signal_handler as usize);
        }
        // Start the X server
        let x_server_pid = match start_x_server(DISPLAY, VT) {
            Ok(pid) => pid,
            Err(e) => {
                return;
            }
        };
        unsafe {
            X_PID = x_server_pid;
        };
    }

    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![try_login])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|_, _| {});
}
