#![feature(duration_as_u128)]

extern crate openh264;

use openh264::Encoder;
use openh264::I420Picture;


use std::time;
use std::io::Write;
use std::io::Read;
use std::fs::{ self, OpenOptions, };


fn main() {
    let w = 320u32;
    let h = 240u32;

    let mut encoder = Encoder::new(w, h, 1200).expect("Ooops ...");
    let mut yuv_file = fs::File::open("c.yuv").unwrap();
    let mut h264_file = {
        let _ = fs::remove_file("cc.h264");
        OpenOptions::new().create_new(true).write(true).open("cc.h264").unwrap()
    };

    let ysize = (w * h) as usize;
    let uvsize = ysize / 4;

    for i in 0..4 {
        let mut y: Vec<u8> = vec![0u8; ysize ];
        let mut u: Vec<u8> = vec![0u8; uvsize ];
        let mut v: Vec<u8> = vec![0u8; uvsize ];
        
        yuv_file.read_exact(&mut y).unwrap();
        yuv_file.read_exact(&mut u).unwrap();
        yuv_file.read_exact(&mut v).unwrap();

        let picture = I420Picture {
            y, u, v,
            width: w,
            height: h,
            stride: [w, w / 2, w / 2, 0],
            timestamp: 30,
        };

        encoder.encode(picture, &mut h264_file);
    }
}