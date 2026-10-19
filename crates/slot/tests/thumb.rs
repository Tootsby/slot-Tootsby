use slot::thumb;
use slot_retro::{GBA_H, GBA_W};

fn decode(png_bytes: &[u8]) -> (Vec<u8>, u32, u32) {
    let mut reader = png::Decoder::new(png_bytes).read_info().expect("read info");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("decode");
    assert_eq!(info.color_type, png::ColorType::Rgb);
    buf.truncate(info.buffer_size());
    (buf, info.width, info.height)
}

#[test]
fn a_thumbnail_keeps_the_frames_colours() {
    let mut frame = vec![0u8; (GBA_W * GBA_H * 4) as usize];
    for px in frame.chunks_exact_mut(4) {
        px.copy_from_slice(&[0x20, 0x40, 0xd0, 0xff]);
    }
    let encoded = thumb::png(&frame).expect("encode");
    let (rgb, w, h) = decode(&encoded);
    assert_eq!((w, h), (GBA_W, GBA_H));
    assert_eq!(&rgb[..3], &[0xd0, 0x40, 0x20]);
}

#[test]
fn a_short_frame_is_not_a_thumbnail() {
    assert!(thumb::png(&[]).is_none());
    assert!(thumb::png(&vec![0u8; (GBA_W * GBA_H * 4) as usize - 4]).is_none());
}

#[test]
fn crop_picture_extracts_the_centred_gb_frame_from_a_gba_canvas() {
    let mut frame = vec![0u8; (GBA_W * GBA_H * 4) as usize];
    for y in 8..(8 + 144) {
        for x in 40..(40 + 160) {
            let idx = ((y * GBA_W + x) * 4) as usize;
            frame[idx..idx + 4].copy_from_slice(&[0x10, 0x80, 0xf0, 0xff]);
        }
    }
    let full = thumb::png(&frame).expect("encode full");
    let cropped = thumb::crop_picture(&full, (160, 144)).expect("crop gb");
    let (rgb, w, h) = decode(&cropped);
    assert_eq!((w, h), (160, 144));
    assert!(
        rgb.chunks_exact(3).all(|px| px == [0xf0, 0x80, 0x10]),
        "cropped GB screenshot included black border pixels"
    );
}
