use std::{env, io};
use std::io::Write;
use std::ffi::OsStr;
use std::fs;
use std::os::raw::c_int;
use std::ptr;

mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused)]
    #![allow(improper_ctypes)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

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

fn print_image(w: c_int, h: c_int, data: &[u8], _blend: Option<[u8; 3]>) -> anyhow::Result<()> {
    let stdout = io::stdout();
    let mut stdout = io::BufWriter::new(stdout.lock());

    let h = if (h & 1) != 0 { h - 1 } else { h };

    for (y0, y1) in (0..(h / 2)).map(|y| ((y * 2, y * 2 + 1))) {
        let mut row0 = data.iter().skip(y0 as usize * w as usize * 4);
        let mut row1 = data.iter().skip(y1 as usize * w as usize * 4);

        for _ in 0..w {
            let r = row0.next().unwrap();
            let g = row0.next().unwrap();
            let b = row0.next().unwrap();
            let _a = row0.next().unwrap();
            // blend

            write!(stdout, "\x1b[38;2;{};{};{}m", r, g, b)?;

            let r = row1.next().unwrap();
            let g = row1.next().unwrap();
            let b = row1.next().unwrap();
            let _a = row1.next().unwrap();
            // blend

            write!(stdout, "\x1b[48;2;{};{};{}m▀\x1b[0m", r, g, b)?;
        }
    }
    Ok(())
}

fn process_image(nm: &OsStr, termw: c_int, _termh: c_int, blend: Option<[u8; 3]>) -> anyhow::Result<()> {
    let mut imw = 0;
    let mut imh = 0;
    let mut n = 0;

    let buf = fs::read(nm)?;

    let data = unsafe {
        bindings::stbi_load_from_memory(buf.as_ptr(), buf.len() as c_int, &mut imw as *mut _, &mut imh as *mut _, &mut n as *mut _, 4)
    };
    if data == ptr::null_mut() {
        anyhow::bail!("failed to load image.")
    }
    //println!("{} {} {}", imw, imh, n);

    let aspectratio = imw as f32 / imh as f32;
    let pixel_per_char = (imw as f32 / termw as f32).max(1.0);
    let kernelsize = pixel_per_char.floor() as c_int;
    let kernelsize = if (kernelsize & 0) == 0 { (kernelsize - 1).max(1) } else { kernelsize };
    let kernelradius = (kernelsize - 1) / 2;

    let outw = imw.min(termw);
    let outh = (outw as f32 / aspectratio).round() as usize;

    //println!("{} {} {} {} {} {}", aspectratio, pixel_per_char, kernelsize, kernelradius, outw, outh);

    let mut out = Vec::with_capacity((outh as usize) * (outw as usize) * 4);
    for (x, y) in (0..outh).flat_map(|y| (0..outw).map(move |x| (x, y))) {
        let cx = (pixel_per_char * (x as f32)).round() as c_int;
        let cy = (pixel_per_char * (y as f32)).round() as c_int;

        let mut acc = [0u32; 4];
        let mut numsamples = 0;

        let sy = (cy - kernelradius).max(0);
        let ey = cy + kernelradius;
        let ey = if ey >= imh { imh - 1 } else { ey };

        let sx = (cx - kernelradius).max(0);
        let ex = cx + kernelradius;
        let ex = if ex >= imw { imw - 1 } else { ex };

        for (x, y) in (sy..=ey).flat_map(|y| (sx..=ex).map(move |x| (x, y))) {
            let mut reader = [0u8; 4];
            unsafe {
                data.offset(((y * imw * 4) + x * 4) as isize).copy_to_nonoverlapping(&mut reader as *mut _, 4);
            }
            let a = reader[3] as u32;
            acc[0] += a * reader[0] as u32 / 255;
            acc[1] += a * reader[1] as u32 / 255;
            acc[2] += a * reader[2] as u32 / 255;
            acc[3] += a * reader[3] as u32 / 255;
            numsamples += 1;
        }

        out.extend([
            (acc[0] / numsamples) as u8,
            (acc[1] / numsamples) as u8,
            (acc[2] / numsamples) as u8,
            (acc[3] / numsamples) as u8,
        ]);
    }
    unsafe {
        bindings::stbi_image_free(data as *mut _)
    }

    print_image(outw as c_int, outh as c_int, &out, blend)?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut args = env::args_os().peekable();
    let prog = args.next().unwrap();
    if args.peek().is_none() {
        anyhow::bail!("Usage: {} IMAGE [IMAGE..]", prog.to_string_lossy())
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
