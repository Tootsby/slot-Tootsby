use slot_retro::{GBA_H, GBA_W};

pub fn png(xrgb8888: &[u8]) -> Option<Vec<u8>> {
    let n = (GBA_W * GBA_H) as usize;
    if xrgb8888.len() < n * 4 {
        return None;
    }
    let mut rgb = Vec::with_capacity(n * 3);
    for px in xrgb8888[..n * 4].chunks_exact(4) {
        rgb.extend_from_slice(&[px[2], px[1], px[0]]);
    }

    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, GBA_W, GBA_H);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().ok()?;
    writer.write_image_data(&rgb).ok()?;
    writer.finish().ok()?;
    Some(out)
}

pub fn crop_picture(png_bytes: &[u8], (w, h): (u32, u32)) -> Option<Vec<u8>> {
    if (w, h) == (GBA_W, GBA_H) {
        return Some(png_bytes.to_vec());
    }
    let dec = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgb || info.width < w || info.height < h {
        return Some(png_bytes.to_vec());
    }
    let ox = ((info.width - w) / 2) as usize;
    let oy = ((info.height - h) / 2) as usize;
    let stride = info.width as usize * 3;
    let row_bytes = w as usize * 3;
    let mut cropped = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h as usize {
        let start = (oy + y) * stride + ox * 3;
        cropped.extend_from_slice(&buf[start..start + row_bytes]);
    }
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().ok()?;
    writer.write_image_data(&cropped).ok()?;
    writer.finish().ok()?;
    Some(out)
}
