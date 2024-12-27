use std::time::{Duration, Instant};
use std::{fs, thread};

use anyhow::Result;
use image::{DynamicImage, RgbaImage};
use log::{error, info};
use serialport::SerialPort;

const SCREEN_WIDTH: u32 = 320;
const SCREEN_HEIGHT: u32 = 240;
const FRAME_DURATION: u128 = 1000 / 24; // 24 FPS

fn main() -> Result<()> {
    env_logger::init();
    info!("启动USB-Screen");

    // 自动查找并连接RP2040设备
    let mut port = find_and_open_rp2040()?;
    info!("RP2040设备已连接");

    // 选择要发送的图片
    let images = select_images()?;
    info!("已选择图片数量: {}", images.len());

    // 循环发送图片
    loop {
        for image_path in &images {
            let image = load_image(image_path)?;
            let start_time = Instant::now();

            // 发送图片到RP2040
            if let Err(err) = send_image_to_rp2040(&mut port, &image) {
                error!("发送图片失败: {:?}", err);
                return Err(err);
            }

            // 计算并等待帧时间
            let elapsed = start_time.elapsed().as_millis();
            if elapsed < FRAME_DURATION {
                thread::sleep(Duration::from_millis((FRAME_DURATION - elapsed) as u64));
            }
        }
    }
}

fn find_and_open_rp2040() -> Result<Box<dyn SerialPort>> {
    for port_info in serialport::available_ports()? {
        if port_info.port_type
            == serialport::SerialPortType::UsbPort(serialport::UsbPortInfo {
                vid: 0x2E8A, // RP2040的USB VID
                pid: 0x000A, // RP2040的USB PID
                ..
            })
        {
            return serialport::new(port_info.port_name, 115_200)
                .open()
                .map_err(|e| e.into());
        }
    }
    Err(anyhow::anyhow!("未找到RP2040设备"))
}

fn select_images() -> Result<Vec<String>> {
    let paths = fs::read_dir("./images")?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("png") {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    Ok(paths)
}

fn load_image(path: &str) -> Result<RgbaImage> {
    let img = image::open(path)?;
    let img = img.resize_exact(
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        image::imageops::FilterType::Lanczos3,
    );
    Ok(img.to_rgba8())
}

fn send_image_to_rp2040(port: &mut Box<dyn SerialPort>, image: &RgbaImage) -> Result<()> {
    let rgb565 = rgb888_to_rgb565(&image);
    port.write_all(&rgb565)?;
    Ok(())
}

fn rgb888_to_rgb565(image: &RgbaImage) -> Vec<u8> {
    let mut rgb565 = Vec::with_capacity((SCREEN_WIDTH * SCREEN_HEIGHT * 2) as usize);
    for pixel in image.pixels() {
        let r = pixel[0] as u16;
        let g = pixel[1] as u16;
        let b = pixel[2] as u16;
        let rgb565_pixel = ((r & 0b11111000) << 8) | ((g & 0b11111100) << 3) | (b >> 3);
        rgb565.extend_from_slice(&rgb565_pixel.to_be_bytes());
    }
    rgb565
}
