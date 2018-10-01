
use crate::openh264_sys::*;

use super::decoder::I420Frame;

use std::ptr;
use std::mem;
use std::slice;
use std::io::Write;
use std::ffi::c_void;


pub struct I420Picture {
    pub y: Vec<u8>,
    pub u: Vec<u8>,
    pub v: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: [u32; 4], // w >> 1
    pub timestamp: u64,
}

impl Into<SSourcePicture> for I420Picture {
    fn into(mut self) -> SSourcePicture {
        SSourcePicture {
            iColorFormat: EVideoFormatType::videoFormatI420 as i32,
            iStride: [
                self.stride[0] as i32,
                self.stride[1] as i32,
                self.stride[1] as i32,
                0,
            ],
            pData: [
                self.y.as_mut_ptr(),
                self.u.as_mut_ptr(),
                self.v.as_mut_ptr(),
                ptr::null_mut()
            ],
            iPicWidth: self.width as i32,
            iPicHeight: self.height as i32,
            uiTimeStamp: 30,
        }
    }
}

pub struct Encoder {
    inner: *mut ISVCEncoder,
    frame_info: SFrameBSInfo,
}

impl Encoder {
    pub fn new(w: u32, h: u32, bitrate: u32) -> Result<Self, ()> {
        let mut inner: *mut ISVCEncoder = ptr::null_mut();

        let param: SEncParamBase = SEncParamBase {
            iUsageType: EUsageType::SCREEN_CONTENT_REAL_TIME, // CAMERA_VIDEO_REAL_TIME
            iPicWidth: w as i32,
            iPicHeight: h as i32,
            iTargetBitrate: bitrate as i32,
            iRCMode: RC_MODES::RC_BITRATE_MODE,
            fMaxFrameRate: 25.0,
        };

        unsafe {
            if WelsCreateSVCEncoder(&mut inner) != 0 {
                panic!("Create Decoder failed.");
            }
            assert!(!inner.is_null());
            
            // InitializeExt
            if (**inner).Initialize.unwrap()(inner, &param) != 0 {
                panic!("Decoder initialization failed.");
            }
        }

        unsafe {
            let set_option = (**inner).SetOption.unwrap();
            // WELS_LOG_ERROR WELS_LOG_WARNING
            set_option(inner,
                      ENCODER_OPTION::ENCODER_OPTION_TRACE_LEVEL,
                      mem::transmute::<&openh264_sys::_bindgen_ty_3, _>(&WELS_LOG_WARNING),
            );

            let mut video_format = EVideoFormatType::videoFormatI420 as i32;

            set_option(inner,
                      ENCODER_OPTION::ENCODER_OPTION_DATAFORMAT,
                      &mut video_format as *mut i32 as *mut c_void,
            );

            set_option(inner,
                      ENCODER_OPTION::ENCODER_OPTION_SVC_ENCODE_PARAM_BASE,
                      mem::transmute::<&SEncParamBase, _>(&param),
            );
        }

        Ok(Self {
            inner: inner,
            frame_info: unsafe { mem::zeroed() },
        })
    }

    pub fn encode<O: Write>(&mut self, pic: I420Picture, mut out: O) {
        let picture: SSourcePicture = pic.into();

        let encode_frame = unsafe { (**(self.inner)).EncodeFrame.unwrap() };
        
        let ret = unsafe {
            encode_frame(self.inner, &picture, &mut self.frame_info)
        } as u32;
        
        assert_eq!(ret, CM_RETURN::cmResultSuccess as u32);
        
        // videoFrameTypeInvalid
        if self.frame_info.eFrameType != EVideoFrameType::videoFrameTypeSkip {
            for layer in &self.frame_info.sLayerInfo[..self.frame_info.iLayerNum as usize] {
                println!("{:?}", layer);
                let nals_count = layer.iNalCount as usize;

                let sizes = unsafe { slice::from_raw_parts(layer.pNalLengthInByte, nals_count) };
                let buffer = unsafe { slice::from_raw_parts(layer.pBsBuf, sizes.iter().sum::<i32>() as usize) };
                
                let mut start = 0usize;
                let mut end = 0usize;

                for size in sizes {
                    let end = start + (*size) as usize;
                    let nal = &buffer[start..end];
                    out.write_all(nal).unwrap();
                    start = end;
                }
            }
        } else {
            println!("Frame: {:?}", self.frame_info.eFrameType);
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            assert_eq!((**(self.inner)).Uninitialize.unwrap()(self.inner), 0);
            WelsDestroySVCEncoder(self.inner);
        }
    }
}