use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
struct Rgba(u8, u8, u8, u8);

pub fn ensure_runtime_icons(data_dir: &Path) -> io::Result<(PathBuf, PathBuf, PathBuf)> {
    let dir = data_dir.join("assets");
    fs::create_dir_all(&dir)?;

    let app = dir.join("app.ico");
    let stopped = dir.join("tray-stopped.ico");
    let running = dir.join("tray-running.ico");

    write_icon_if_missing(&app, Rgba(255, 145, 0, 255), Rgba(24, 24, 24, 255))?;
    write_icon_if_missing(&running, Rgba(255, 145, 0, 255), Rgba(24, 24, 24, 255))?;
    write_icon_if_missing(&stopped, Rgba(250, 250, 250, 255), Rgba(72, 72, 72, 255))?;

    Ok((app, stopped, running))
}

fn write_icon_if_missing(path: &Path, fill: Rgba, glyph: Rgba) -> io::Result<()> {
    if path.is_file() {
        return Ok(());
    }
    fs::write(path, build_ico(fill, glyph))
}

fn build_ico(fill: Rgba, glyph: Rgba) -> Vec<u8> {
    const SIZES: [u32; 3] = [16, 32, 48];
    let images = SIZES
        .iter()
        .map(|&size| build_dib(size, fill, glyph))
        .collect::<Vec<_>>();

    let header_len = 6 + 16 * images.len();
    let mut out = Vec::with_capacity(header_len + images.iter().map(Vec::len).sum::<usize>());
    push_u16(&mut out, 0);
    push_u16(&mut out, 1);
    push_u16(&mut out, images.len() as u16);

    let mut offset = header_len as u32;
    for (&size, image) in SIZES.iter().zip(&images) {
        out.push(if size >= 256 { 0 } else { size as u8 });
        out.push(if size >= 256 { 0 } else { size as u8 });
        out.push(0);
        out.push(0);
        push_u16(&mut out, 1);
        push_u16(&mut out, 32);
        push_u32(&mut out, image.len() as u32);
        push_u32(&mut out, offset);
        offset += image.len() as u32;
    }

    for image in images {
        out.extend_from_slice(&image);
    }
    out
}

fn build_dib(size: u32, fill: Rgba, glyph: Rgba) -> Vec<u8> {
    let xor_len = (size * size * 4) as usize;
    let mask_stride = ((size + 31) / 32) * 4;
    let mask_len = (mask_stride * size) as usize;
    let mut dib = Vec::with_capacity(40 + xor_len + mask_len);

    push_u32(&mut dib, 40);
    push_i32(&mut dib, size as i32);
    push_i32(&mut dib, (size * 2) as i32);
    push_u16(&mut dib, 1);
    push_u16(&mut dib, 32);
    push_u32(&mut dib, 0);
    push_u32(&mut dib, xor_len as u32);
    push_i32(&mut dib, 0);
    push_i32(&mut dib, 0);
    push_u32(&mut dib, 0);
    push_u32(&mut dib, 0);

    for y_bottom in 0..size {
        let y = size - 1 - y_bottom;
        for x in 0..size {
            let pixel = icon_pixel(size, x, y, fill, glyph);
            dib.extend_from_slice(&[pixel.2, pixel.1, pixel.0, pixel.3]);
        }
    }
    dib.resize(40 + xor_len + mask_len, 0);
    dib
}

fn icon_pixel(size: u32, x: u32, y: u32, fill: Rgba, glyph: Rgba) -> Rgba {
    let cx = (size as i32 - 1) / 2;
    let cy = cx;
    let dx = x as i32 - cx;
    let dy = y as i32 - cy;
    let radius = (size as i32 * 44) / 100;
    if dx * dx + dy * dy > radius * radius {
        return Rgba(0, 0, 0, 0);
    }

    let sx0 = size * 10 / 32;
    let sx1 = size * 22 / 32;
    let top = size * 8 / 32;
    let mid = size * 15 / 32;
    let bot = size * 23 / 32;
    let t = (size / 8).max(1);

    let top_bar = y >= top && y < top + t && x >= sx0 && x <= sx1;
    let mid_bar = y >= mid && y < mid + t && x >= sx0 && x <= sx1;
    let bot_bar = y >= bot && y < bot + t && x >= sx0 && x <= sx1;
    let left = x >= sx0 && x < sx0 + t && y >= top && y <= mid;
    let right = x + t > sx1 && x <= sx1 && y >= mid && y <= bot;

    if top_bar || mid_bar || bot_bar || left || right {
        glyph
    } else {
        fill
    }
}

fn push_u16(out: &mut Vec<u8>, value: u16) { out.extend_from_slice(&value.to_le_bytes()); }
fn push_u32(out: &mut Vec<u8>, value: u32) { out.extend_from_slice(&value.to_le_bytes()); }
fn push_i32(out: &mut Vec<u8>, value: i32) { out.extend_from_slice(&value.to_le_bytes()); }
