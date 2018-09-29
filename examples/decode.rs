extern crate openh264;
extern crate mp4;

use openh264::Decoder;
use mp4::track::H264VideoTrack;
use mp4::track::VideoCodec;
use mp4::nalu::Nalu;

use std::io::Write;
use std::io::Read;
use std::io::Seek;
use std::fs::{ self, OpenOptions, };


fn decode_nalu<F: Write>(decoder: &mut Decoder, nalu: &Nalu, output: &mut F) {
    let bytes = nalu.as_bytes();

    match decoder.decode(bytes) {
        Some(mut i420_frame) => {
            i420_frame.save(output);
        },
        None => {

        },
    }
}

fn main() {
    let mut rawvideo_file = {
        let _ = fs::remove_file("rawvideo.yuv");
        OpenOptions::new().create_new(true).write(true).open("rawvideo.yuv").unwrap()
    };

    let mut decoder = Decoder::new().expect("Ooops ...");
    let mut mp4_file = fs::File::open("a.mp4").unwrap();  
    let mp4_ctx = mp4::parse::parse(&mut mp4_file).unwrap();
    
    for video_track in mp4_ctx.video_tracks {
        if video_track.codec() == VideoCodec::H264 {
            println!("{}", video_track);

            let avc_config = video_track.avc_config_record().unwrap();

            for sps in avc_config.sps.iter() {
                let nalu = Nalu::new(sps.clone());
                decode_nalu(&mut decoder, &nalu, &mut rawvideo_file);
            }
            for pps in avc_config.pps.iter() {
                let nalu = Nalu::new(pps.clone());
                decode_nalu(&mut decoder, &nalu, &mut rawvideo_file);
            }

            for sample in video_track.samples() {
                for nalu in sample.nalus(&mut mp4_file) {
                    decode_nalu(&mut decoder, &nalu, &mut rawvideo_file);
                }
            }
        }
    }

    println!("ffplay -f rawvideo -pixel_format yuv420p -video_size 1280x720 rawvideo.yuv");
    println!("vlc --demux rawvideo --rawvid-fps 5 --rawvid-width 1280 --rawvid-height 720 --rawvid-chroma I420 rawvideo.yuv");
}