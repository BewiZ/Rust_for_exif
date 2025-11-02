use gufo_exif::Exif;
use gufo_jpeg::Jpeg;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("使用方法: {} <图片路径>", args[0]);
        return Ok(());
    }
    
    let path = &args[1];
    println!("正在解析: {}", path);
    
    // 检查文件是否存在
    if !std::path::Path::new(path).exists() {
        eprintln!("文件不存在: {}", path);
        return Ok(());
    }
    
    // 读取文件
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("读取文件失败: {}", e);
            return Ok(());
        }
    };
    
    // 解析 JPEG
    let jpeg = match Jpeg::new(data) {
        Ok(jpeg) => jpeg,
        Err(e) => {
            eprintln!("JPEG 解析失败: {}", e);
            return Ok(());
        }
    };
    
    // 提取 EXIF 数据
    let exif_data: Vec<_> = jpeg.exif_data().collect();
    if exif_data.is_empty() {
        println!("未找到 EXIF 数据");
        return Ok(());
    }
    
    println!("找到 {} 个 EXIF 数据段", exif_data.len());
    
    // 处理第一个 EXIF 数据段
    let raw_exif = exif_data[0].to_vec();
    
    // 解析 EXIF
    let exif = match Exif::new(raw_exif) {
        Ok(exif) => exif,
        Err(e) => {
            eprintln!("EXIF 解析失败: {}", e);
            return Ok(());
        }
    };
    
    // 显示 EXIF 信息
    print_exif_info(&exif);
    
    Ok(())
}

fn print_exif_info(exif: &Exif) {
    println!("\n=== EXIF 信息 ===");
    
    // 相机基本信息
    println!("\n--- 相机信息 ---");
    if let Some(make) = exif.make() {
        println!("制造商: {}", make);
    }
    
    if let Some(model) = exif.model() {
        println!("型号: {}", model);
    }
    
    if let Some(owner) = exif.camera_owner() {
        println!("相机所有者: {}", owner);
    }
    
    // 拍摄参数
    println!("\n--- 拍摄参数 ---");
    if let Some(orientation) = exif.orientation() {
        println!("方向: {:?}", orientation);
    }
    
    if let Some(iso) = exif.iso_speed_rating() {
        println!("ISO: {}", iso);
    }
    
    if let Some(f_number) = exif.f_number() {
        println!("光圈值: f/{:.1}", f_number);
    }
    
    if let Some(focal_length) = exif.focal_length() {
        println!("焦距: {:.1} mm", focal_length);
    }
    
    if let Some((numerator, denominator)) = exif.exposure_time() {
        if denominator != 0 {
            let exposure = numerator as f32 / denominator as f32;
            if exposure < 1.0 {
                println!("曝光时间: 1/{:.0} 秒", 1.0 / exposure);
            } else {
                println!("曝光时间: {:.1} 秒", exposure);
            }
        }
    }
    
    // 软件信息
    println!("\n--- 软件信息 ---");
    if let Some(software) = exif.software() {
        println!("软件: {}", software);
    }
    
    if let Some(comment) = exif.user_comment() {
        println!("用户评论: {}", comment);
    }
    
    // GPS 信息
    println!("\n--- GPS 信息 ---");
    if let Some(location) = exif.gps_location() {
        println!("GPS 位置: {:?}", location);
    } else {
        println!("无 GPS 位置信息");
    }
    
    // 调试信息（可选）
    println!("\n--- 调试信息 ---");
    let debug_info = exif.debug_dump();
    println!("调试输出长度: {} 字符", debug_info.len());
    
    // 如果需要查看完整的调试信息，可以取消下面的注释
    // println!("完整的调试信息:\n{}", debug_info);
}