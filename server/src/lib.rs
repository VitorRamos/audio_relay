#![feature(once_cell_get_mut)]
#[allow(dead_code)]
mod aptx;

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
pub mod android {
    extern crate android_logger;
    extern crate log;
    use crate::aptx::AptxContext;
    use android_logger::{Config, FilterBuilder};
    use jni::{
        objects::{JByteArray, JClass},
        JNIEnv,
    };
    use log::error;
    use log::LevelFilter;
    use std::sync::OnceLock;

    static mut APTX_CONTEXT: OnceLock<Box<AptxContext>> = OnceLock::new();
    static mut DECODED_BUFFER: OnceLock<Vec<u8>> = OnceLock::new();

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_example_pcstream_AudioService_init_1decode_1rust(
        _env: JNIEnv,
        _: JClass,
    ) -> i32 {
        android_logger::init_once(
            Config::default()
                .with_max_level(LevelFilter::Trace) // limit log level
                .with_tag("mytag") // logs will show under mytag tag
                .with_filter(
                    // configure messages for specific crate
                    FilterBuilder::new()
                        .parse("debug,hello::crate=error")
                        .build(),
                ),
        );
        let _ctx = APTX_CONTEXT.get_or_init(|| AptxContext::new(false));
        DECODED_BUFFER.get_or_init(|| vec![0; 2048]);
        1
    }

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_example_pcstream_AudioService_decode_1rust(
        mut env: JNIEnv,
        _: JClass,
        input: JByteArray,
        output: JByteArray,
    ) {
        let ctx = APTX_CONTEXT.get_mut().expect("Fail to get context");
        let data: Vec<u8> = env
            .get_array_elements(&input, jni::objects::ReleaseMode::NoCopyBack)
            .expect("Fail to get elements")
            .iter()
            .map(|&x| x as u8)
            .collect();
        let mut written = 0;
        let mut dropped = 0;
        let mut synced = false;
        let processed = ctx.decode_sync(
            &data,
            DECODED_BUFFER.get_mut().expect("Fail to get context"),
            &mut written,
            &mut synced,
            &mut dropped,
        );
        if processed != data.len() {
            error!("Fail to process audio {} {}", processed, data.len());
        }
        let out_buffer: &Vec<i8> =
            std::mem::transmute(DECODED_BUFFER.get().expect("Fail to get context"));
        env.set_byte_array_region(output, 0, out_buffer)
            .expect("Fail to set output buffer");
    }
}
