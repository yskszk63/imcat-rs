#![cfg_attr(target_os = "wasi", feature(wasi_ext))]
use std::io::{self, Read, Write};
use std::os::raw::c_int;
use std::ptr;

mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused)]
    #![allow(improper_ctypes)]
    pub type stbi_uc = ::std::os::raw::c_uchar;
    extern "C" {
        #[doc = ""]
        pub fn stbi_load_from_memory(
            buffer: *const stbi_uc,
            len: ::std::os::raw::c_int,
            x: *mut ::std::os::raw::c_int,
            y: *mut ::std::os::raw::c_int,
            channels_in_file: *mut ::std::os::raw::c_int,
            desired_channels: ::std::os::raw::c_int,
        ) -> *mut stbi_uc;
    }
    extern "C" {
        pub fn stbi_image_free(retval_from_stbi_load: *mut ::std::os::raw::c_void);
    }
}

fn print_image<W>(
    output: &mut W,
    w: c_int,
    h: c_int,
    data: &[u8],
    _blend: Option<[u8; 3]>,
) -> anyhow::Result<()>
where
    W: io::Write,
{
    let mut output = io::BufWriter::new(output);

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

            write!(output, "\x1b[38;2;{};{};{}m", r, g, b)?;

            let r = row1.next().unwrap();
            let g = row1.next().unwrap();
            let b = row1.next().unwrap();
            let _a = row1.next().unwrap();
            // blend

            write!(output, "\x1b[48;2;{};{};{}m▀\x1b[0m", r, g, b)?;
        }
    }
    output.flush()?;
    Ok(())
}

pub fn imcat<R, W>(
    image: &mut R,
    output: &mut W,
    termw: c_int,
    _termh: c_int,
    blend: Option<[u8; 3]>,
) -> anyhow::Result<()>
where
    R: Read,
    W: Write,
{
    let mut imw = 0;
    let mut imh = 0;
    let mut n = 0;

    let mut buf = vec![];
    image.read_to_end(&mut buf)?;

    let data = unsafe {
        bindings::stbi_load_from_memory(
            buf.as_ptr(),
            buf.len() as c_int,
            &mut imw as *mut _,
            &mut imh as *mut _,
            &mut n as *mut _,
            4,
        )
    };
    if data == ptr::null_mut() {
        anyhow::bail!("failed to load image.")
    }
    //println!("{} {} {}", imw, imh, n);

    let aspectratio = imw as f32 / imh as f32;
    let pixel_per_char = (imw as f32 / termw as f32).max(1.0);
    let kernelsize = pixel_per_char.floor() as c_int;
    let kernelsize = if (kernelsize & 0) == 0 {
        (kernelsize - 1).max(1)
    } else {
        kernelsize
    };
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
                data.offset(((y * imw * 4) + x * 4) as isize)
                    .copy_to_nonoverlapping(&mut reader as *mut _, 4);
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
    unsafe { bindings::stbi_image_free(data as *mut _) }

    print_image(output, outw as c_int, outh as c_int, &out, blend)?;

    Ok(())
}

#[cfg(target_os = "wasi")]
#[no_mangle]
pub extern fn _initialize() {
    println!("OK");
}

#[cfg(target_os = "wasi")]
#[no_mangle]
pub extern  fn wasi_imcat(
    image: c_int,
    output: c_int,
    termw: c_int,
    termh: c_int,
) -> c_int {
    use std::fs;
    use std::os::wasi::io::{RawFd, FromRawFd};

    let mut image = unsafe {
        fs::File::from_raw_fd(image as RawFd)
    };
    let mut output = unsafe {
        fs::File::from_raw_fd(output as RawFd)
    };

    if let Err(err) = imcat(&mut image, &mut output, termw, termh, None) {
        eprintln!("{}", err);
        1
    } else {
        0
    }
}
