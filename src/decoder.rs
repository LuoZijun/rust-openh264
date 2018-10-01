
use crate::openh264_sys::*;

use std::ptr;
use std::mem;
use std::slice;
// use std::time;
// use std::thread;
use std::io::Write;


pub struct I420Frame<'y, 'u, 'v> {
    pub y: &'y [u8],
    pub u: &'u [u8],
    pub v: &'v [u8],
    pub width: usize,
    pub height: usize,
    pub stride: [usize; 2],
}

impl<'y, 'u, 'v> I420Frame<'y, 'u, 'v> {
    pub fn save<F: Write>(&mut self, output: &mut F) {
        let width = self.width;
        let height = self.height;
        for i in 0..height {
            let start = i * self.stride[0];
            let end = start + width;
            output.write_all(&self.y[start..end]).unwrap();
        }

        let width = self.width / 2;
        let height = self.height / 2;
        for i in 0..height {
            let start = i * self.stride[1];
            let end = start + width;
            output.write_all(&self.u[start..end]).unwrap();
        }
        for i in 0..height {
            let start = i * self.stride[1];
            let end = start + width;
            output.write_all(&self.v[start..end]).unwrap();
        }
    }
}


pub struct Decoder {
    inner: *mut ISVCDecoder,
    dst_info: SBufferInfo,
    yuv_buffer: [*mut u8; 3],
}


impl Decoder {
    pub fn new() -> Result<Self, ()> {
        let mut decoder: *mut ISVCDecoder = ptr::null_mut();

        let param: SDecodingParam = SDecodingParam {
            pFileNameRestructed: ptr::null_mut(),
            uiCpuLoad: 0,
            uiTargetDqLayer: 0,
            eEcActiveIdc: ERROR_CON_IDC::ERROR_CON_SLICE_COPY,
            bParseOnly: false,
            sVideoProperty: SVideoProperty {
                size: mem::size_of::<SVideoProperty>() as u32,
                eVideoBsType: VIDEO_BITSTREAM_TYPE::VIDEO_BITSTREAM_DEFAULT,
            },
        };

        unsafe {
            if WelsCreateDecoder(&mut decoder) != 0 {
                panic!("Create Decoder failed.");
            }
            assert!(!decoder.is_null());
            
            if (**decoder).Initialize.unwrap()(decoder, &param) != 0 {
                panic!("Decoder initialization failed.");
            }
        }

        unsafe {
            let set_option = (**decoder).SetOption.unwrap();
            // WELS_LOG_ERROR WELS_LOG_WARNING
            set_option(decoder,
                      DECODER_OPTION::DECODER_OPTION_TRACE_LEVEL,
                      mem::transmute::<&openh264_sys::_bindgen_ty_3, _>(&WELS_LOG_WARNING),
            );
            set_option(decoder,
                      DECODER_OPTION::DECODER_OPTION_ERROR_CON_IDC,
                      mem::transmute::<&i32, _>(&(ERROR_CON_IDC::ERROR_CON_SLICE_COPY as i32)),
            );
        }

        Ok(Decoder {
            inner: decoder,
            dst_info: unsafe { mem::zeroed() },
            yuv_buffer: [ ptr::null_mut(), ptr::null_mut(), ptr::null_mut() ]
        })
    }

    pub fn decode(&mut self, bytes: &[u8]) -> Option<I420Frame> {
        // let mut yuv_buffer: [*mut u8; 3] = [ ptr::null_mut(), ptr::null_mut(), ptr::null_mut() ];
        let mut yuv_buffer = self.yuv_buffer;

        let ret = unsafe {
            let decode_frame_no_delay = (**(self.inner)).DecodeFrameNoDelay.unwrap();
            decode_frame_no_delay(self.inner,
                         bytes.as_ptr(),
                         bytes.len() as i32,
                         yuv_buffer.as_mut_ptr(),
                         &mut self.dst_info)
        } as usize;

        if ret != 0 {
            return None;
        }

        let frame_system_buffer = unsafe { self.dst_info.UsrData.sSystemBuffer };
        
        let width = frame_system_buffer.iWidth as usize;
        let height = frame_system_buffer.iHeight as usize;
        if width == 0 || height == 0 || frame_system_buffer.iFormat == 0 || self.dst_info.iBufferStatus != 1 {
            return None;
        }

        let stride = [ frame_system_buffer.iStride[0] as usize, frame_system_buffer.iStride[1] as usize ];
        (self.dst_info.uiInBsTimeStamp) += 40;

        let ysize = width.max(stride[0]) * height.max(stride[1]);
        let uvsize = ysize / 4;

        let y_buffer = unsafe { slice::from_raw_parts(yuv_buffer[0], ysize) };
        let u_buffer = unsafe { slice::from_raw_parts(yuv_buffer[1], uvsize) };
        let v_buffer = unsafe { slice::from_raw_parts(yuv_buffer[2], uvsize) };

        Some(I420Frame {
            y: y_buffer,
            u: u_buffer,
            v: v_buffer,
            width: width,
            height: height,
            stride: stride,
        })
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            assert_eq!((**(self.inner)).Uninitialize.unwrap()(self.inner), 0);
            WelsDestroyDecoder(self.inner);
        }
    }
}