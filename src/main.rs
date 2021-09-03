use std::{env, io};
use std::fs;
use std::os::raw::c_int;
use imcat_rs as imcat;

nix::ioctl_read_bad!(tiocgwinsz, nix::libc::TIOCGWINSZ, nix::pty::Winsize);

fn set_console_mode() {
    // TODO windows
}

fn get_terminal_size() -> anyhow::Result<(u16, u16)> {
    let mut winsize = nix::pty::Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe {
        tiocgwinsz(nix::libc::STDOUT_FILENO, &mut winsize)?;
    }
    Ok((winsize.ws_col, winsize.ws_row))
}

fn process_image(nm: &str, termw: c_int, termh: c_int, blend: Option<[u8; 3]>) -> anyhow::Result<()> {
    let stdout = io::stdout();

    if nm == "-" {
        let stdin = io::stdin();
        imcat::imcat(&mut stdin.lock(), &mut stdout.lock(), termw, termh, blend)?;
    } else {
        let mut fp = fs::File::open(nm)?;
        imcat::imcat(&mut fp, &mut stdout.lock(), termw, termh, blend)?;
    };

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut args = env::args().peekable();
    let prog = args.next().unwrap();
    if let Some("--help") | None = args.peek().cloned().as_deref() {
        anyhow::bail!("Usage: {} IMAGE [IMAGE..]", prog)
    }

    // Parse environment variable for terminal background colour.
    let blend = if let Ok(bg) = env::var("IMCATBG") {
        let bg = bg.parse::<u32>()?;
        Some([
            ((bg >> 16) & 0xFF) as u8,
            ((bg >> 8) & 0xFF) as u8,
            ((bg >> 0) & 0xFF) as u8,
        ])
    } else {
        None
    };

    // Step 0: Windows cmd.exe needs to be put in proper console mode.
    set_console_mode();

    // Step 1: figure out the width and height of terminal.
    let (width, height) =   get_terminal_size()?;
    //println!("{} {}", width, height);

    for nm in args {
        process_image(&nm, width as c_int, height as c_int, blend)?;
    }

    Ok(())
}
