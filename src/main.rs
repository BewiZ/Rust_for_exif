use little_exif::{
    metadata::Metadata,
    exif_tag::ExifTag,
    filetype::FileExtension,
    endian::Endian,
    rational::uR64,
};
use std::{
    str::FromStr,
    fs::File,
    path::Path,
    io::{Read, Cursor, Write},
    convert::TryInto,
};
use quick_xml::{
    Reader,
    events::Event,
};
use flate2::read::ZlibDecoder;


/// 从 PNG 文件中提取 EXIF 数据或保存 XMP 数据到 .xmp.xml 文件
/// 
/// # 参数
/// - `path`: PNG 文件路径
/// 
/// # 返回值
/// - 如果找到 EXIF 数据则返回 Some(Vec<u8>)，否则返回 None
fn extract_exif_from_png(path: &Path) -> Option<Vec<u8>> {
    // PNG 文件签名
    const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n"; // PNG 文件有固定的8字节签名头
    let mut f = File::open(path).ok()?; // 打开文件
    let mut buf = Vec::new(); // 创建缓冲区
    f.read_to_end(&mut buf).ok()?; // 读取整个文件内容
    
    // 验证 PNG 文件签名
    if buf.len() < 8 || &buf[..8] != PNG_SIG { // 如果文件长度小于8字节或前8字节不是 PNG 签名，则返回None
        return None;
    }

    let mut ihdr_info = None; // 用于存储 IHDR 信息

    // 遍历 PNG 数据块
    let mut pos = 8usize; // 跳过 PNG 签名头
    while pos + 8 <= buf.len() {
        // 读取数据块长度
        let len = {
            let slice = &buf[pos..pos+4];
            u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]) as usize
        }; // 数据块长度是4字节的大端整数
        let typ = &buf[pos+4..pos+8]; // 数据块类型是接下来的4字节
        pos += 8; // 移动到数据块数据部分
        
        // 检查数据边界
        if pos + len + 4 > buf.len() { // 如果数据块长度超出文件边界，则停止解析
            break;
        }
        let data = &buf[pos..pos+len]; // 数据块数据部分
        pos += len; // 移动到下一个数据块
        let _crc = &buf[pos..pos+4]; // CRC 校验码（当前未使用）
        pos += 4; // 移动到下一个数据块起始位置

         if typ == b"IHDR" && len >= 13 {
            let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
            let bit_depth = data[8];
            let color_type = data[9];
            let compression_method = data[10];
            let filter_method = data[11];
            let interlace_method = data[12];
            
            ihdr_info = Some((width, height, bit_depth, color_type, 
                             compression_method, filter_method, interlace_method));
            
            println!("PNG IHDR 信息:");
            println!("  宽度: {} 像素", width);
            println!("  高度: {} 像素", height);
            println!("  位深度: {} 位", bit_depth);
            println!("  颜色类型: {}", match color_type {
                0 => "灰度",
                2 => "真彩色",
                3 => "索引颜色",
                4 => "灰度+Alpha",
                6 => "真彩色+Alpha",
                _ => "未知",
            });
            println!("  压缩方法: {}", compression_method);
            println!("  过滤方法: {}", filter_method);
            println!("  交错方法: {}", if interlace_method == 0 { "无交错" } else { "Adam7" });
        }

        // 处理 eXIf 数据块（标准 EXIF 数据）
        if typ == b"eXIf" { // 如果数据块类型是 eXIf，则将其数据部分作为 EXIF 数据返回
            return Some(data.to_vec());
        }

        // 处理 iTXt 数据块（包含 XMP 或 EXIF 文本数据）
        // iTXt 数据块的格式结构如下：
        //     Keyword (关键字): 一个以空字符 (\0) 结尾的 ASCII 字符串。
        //     Null separator (空字符分隔符): 一个字节，值为 0。
        //     Compression flag (压缩标志): 一个字节，0 表示未压缩，1 表示使用 zlib 压缩。
        //     Compression method (压缩方法): 一个字节，通常为 0。
        //     Language tag (语言标签)
        //     Null separator
        //     Translated keyword (翻译后的关键字)
        //     Null separator
        //     Text (文本内容)
        if typ == b"iTXt" { // 如果数据块类型是 iTXt，则解析其内容
            // Some 只有当key_end又值才会执行以下部分
            if let Some(key_end) = data.iter().position(|&b| b == 0) { // 查找key_end关键字结束符"\0"
                if data.len() > key_end + 2 {
                    let compression_flag = data[key_end + 1]; // 获取压缩标志（0 表示未压缩，1 表示 zlib 压缩）
                    let rest = &data[key_end + 3..]; // 跳过关键字、空字符和压缩标志
                    if let Some(lang_end_rel) = rest.iter().position(|&b| b == 0) { // 查找语言标签结束位置
                        let translated_start = key_end + 3 + lang_end_rel + 1; // 翻译后的关键字起始位置
                        if translated_start <= data.len() { // 确保翻译后的关键字存在
                            if let Some(trans_end_rel) = data[translated_start..].iter().position(|&b| b == 0) { // 查找翻译后的关键字结束位置
                                let text_start = translated_start + trans_end_rel + 1; // 文本内容起始位置
                                if text_start <= data.len() { // 确保文本内容存在

                                    // 获取关键字
                                    let key = &data[..key_end];
                                    // key 为 ASCII码，转换字符串为 XML:com.adobe.xmp
                                    // println!("{}", std::str::from_utf8(key).unwrap_or("无效的 UTF-8"));
                                    
                                    // println!("{}", std::str::from_utf8(data).unwrap_or("无效的 UTF-8"));
                                    
                                    // 处理 Adobe XMP 数据
                                    // 如果关键字是 XML:com.adobe.xmp，则保存 XMP 数据到 .xmp.xml 文件
                                    if key.eq_ignore_ascii_case(b"XML:com.adobe.xmp") {
                                        let text_bytes = &data[text_start..];
                                        let xmp_bytes = if compression_flag == 1 {
                                            match decompress_zlib(text_bytes) {
                                                Ok(d) => d,
                                                Err(_) => Vec::new(),
                                            }
                                        } else {
                                            text_bytes.to_vec()
                                        };
                                        // 保存 XMP 数据到文件
                                        if !xmp_bytes.is_empty() {
                                            if let Some(parent) = path.parent() {
                                                let out_path = parent.join(format!("{}.xmp.xml", path.file_stem().and_then(|s| s.to_str()).unwrap_or("extracted")));
                                                let _ = std::fs::write(&out_path, &xmp_bytes);
                                            }
                                        }
                                    }
                                    
                                    // 处理 EXIF 数据
                                    if key.eq_ignore_ascii_case(b"raw profile type exif") || key.eq_ignore_ascii_case(b"exif") {
                                        let text_bytes = &data[text_start..];
                                        if compression_flag == 1 {
                                            if let Ok(decompressed) = decompress_zlib(text_bytes) {
                                                return Some(decompressed);
                                            }
                                        } else {
                                            return Some(text_bytes.to_vec());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// 解压缩 Zlib 数据
/// 接受一个字节数切片 compressed 作为输入参数
/// - 成功时返回解压后的 Vec<u8> 字节向量，失败时返回 std::io::Error 错误
fn decompress_zlib(compressed: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = ZlibDecoder::new(Cursor::new(compressed));
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

/// 在字节切片中查找子切片
/// 
/// # 参数
/// - `haystack`: 要搜索的字节切片
/// - `needle`: 要查找的字节切片
/// 
/// # 返回值
/// - 如果找到则返回起始位置，否则返回 None
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// 显示单个 EXIF 标签的信息 For PNG
/// 
/// ### 参数
/// - `tag`: EXIF 标签
/// - `endian`: 字节序信息
fn display_tag_info(tag: &ExifTag, endian: &Endian) {
    let tag_name = get_human_readable_tag_name(tag);
    let tag_value = get_tag_value_string(tag, endian);
    println!("{}: {}", tag_name, tag_value);
}

/// 显示 JPEG 完整的 EXIF 元数据信息
/// ### 参数
/// - `metadata`: 元数据对象
/// - `endian`: 字节序信息
fn display_exif_metadata(metadata: &Metadata, endian: &Endian) {
    println!("\n=== EXIF 信息 ===");
    display_summary_info(metadata, endian);

    println!("\n--- 所有 EXIF 标签 ---");
    let mut tag_count = 0;
    for tag in metadata {
        display_tag_info(&tag, endian);
        tag_count += 1;
    }
    println!("\n总标签数: {}", tag_count);
}

/// 获取人类可读的标签名称 For PNG
/// # 参数
/// - `tag`: EXIF 标签
/// # 返回值
/// - 对应的中文标签名称
fn get_human_readable_tag_name(tag: &ExifTag) -> String {
    match tag { // 列举 build_tag_enum! 中所有的 EXIF 标签
        ExifTag::Make(_) => "制造商".to_string(),
        ExifTag::Model(_) => "相机型号".to_string(),
        ExifTag::Software(_) => "使用软件".to_string(),
        ExifTag::Artist(_) => "作者".to_string(),
        ExifTag::Copyright(_) => "版权信息".to_string(),
        ExifTag::ISO(_) => "ISO 感光度".to_string(),
        ExifTag::FNumber(_) => "光圈值".to_string(),
        ExifTag::FocalLength(_) => "焦距".to_string(),
        ExifTag::ExposureTime(_) => "曝光时间".to_string(),
        ExifTag::ImageWidth(_) => "图片宽度".to_string(),
        ExifTag::ImageHeight(_) => "图片高度".to_string(),
        ExifTag::XResolution(_) => "水平分辨率".to_string(),
        ExifTag::YResolution(_) => "垂直分辨率".to_string(),
        ExifTag::BitsPerSample(_) => "每样本位数".to_string(),
        ExifTag::Compression(_) => "压缩".to_string(),
        ExifTag::PhotometricInterpretation(_) => "光度解释".to_string(),
        ExifTag::CellWidth(_) => "单元宽度".to_string(),
        ExifTag::CellHeight(_) => "单元高度".to_string(),
        ExifTag::ImageDescription(_) => "图片描述".to_string(),
        ExifTag::Orientation(_) => "图片方向".to_string(),
        ExifTag::SamplesPerPixel(_) => "每像素样本数".to_string(),
        ExifTag::RowsPerStrip(_) => "每带行数".to_string(),
        ExifTag::PlanarConfiguration(_) => "平面配置".to_string(),
        ExifTag::ResolutionUnit(_) => "分辨率单位".to_string(),
        ExifTag::TransferFunction(_) => "传递函数".to_string(),
        ExifTag::ModifyDate(_) => "修改时间".to_string(),
        ExifTag::WhitePoint(_) => "白点".to_string(),
        ExifTag::PrimaryChromaticities(_) => "主要色度".to_string(),
        ExifTag::ColorMap(_) => "颜色映射表".to_string(),
        ExifTag::YCbCrCoefficients(_) => "YCbCr 系数".to_string(),
        ExifTag::YCbCrSubSampling(_) => "YCbCr 子采样".to_string(),
        ExifTag::YCbCrPositioning(_) => "YCbCr 定位".to_string(),
        ExifTag::ReferenceBlackWhite(_) => "参考黑白".to_string(),
        ExifTag::GPSLatitude(_) => "GPS 纬度".to_string(),
        ExifTag::GPSLatitudeRef(_) => "纬度方向".to_string(),
        ExifTag::GPSLongitude(_) => "GPS 经度".to_string(),
        ExifTag::GPSLongitudeRef(_) => "经度方向".to_string(),
        ExifTag::GPSAltitude(_) => "GPS 海拔".to_string(),
        ExifTag::GPSAltitudeRef(_) => "海拔方向".to_string(),
        ExifTag::GPSTimeStamp(_) => "GPS 时间戳".to_string(),
        ExifTag::GPSSatellites(_) => "GPS 卫星".to_string(),
        ExifTag::GPSStatus(_) => "GPS 状态".to_string(),
        ExifTag::GPSMeasureMode(_) => "GPS 测量模式".to_string(),
        ExifTag::GPSDOP(_) => "GPS 精度".to_string(),
        ExifTag::GPSSpeedRef(_) => "GPS 速度参考".to_string(),
        ExifTag::GPSSpeed(_) => "GPS 速度".to_string(),
        ExifTag::GPSTrackRef(_) => "GPS 方向参考".to_string(),
        ExifTag::GPSTrack(_) => "GPS 方向".to_string(),
        ExifTag::GPSImgDirectionRef(_) => "GPS 图像方向参考".to_string(),
        ExifTag::GPSImgDirection(_) => "GPS 图像方向".to_string(),
        ExifTag::GPSMapDatum(_) => "GPS 地图基准".to_string(),
        ExifTag::GPSDestLatitudeRef(_) => "GPS 目的地纬度参考".to_string(),
        ExifTag::GPSDestLatitude(_) => "GPS 目的地纬度".to_string(),
        ExifTag::GPSDestLongitudeRef(_) => "GPS 目的地经度参考".to_string(),
        ExifTag::GPSDestLongitude(_) => "GPS 目的地经度".to_string(),
        ExifTag::GPSDestBearingRef(_) => "GPS 目的地方位参考".to_string(),
        ExifTag::GPSDestBearing(_) => "GPS 目的地方位".to_string(),
        ExifTag::GPSDestDistanceRef(_) => "GPS 目的地距离参考".to_string(),
        ExifTag::GPSDestDistance(_) => "GPS 目的地距离".to_string(),
        ExifTag::GPSProcessingMethod(_) => "GPS 处理方法".to_string(),
        ExifTag::GPSAreaInformation(_) => "GPS 区域信息".to_string(),
        ExifTag::GPSDateStamp(_) => "GPS 日期标记".to_string(),
        ExifTag::GPSDifferential(_) => "GPS 差分".to_string(),
        ExifTag::GPSHPositioningError(_) => "GPS 水平定位误差".to_string(),
        ExifTag::InteroperabilityIndex(_) => "互操作性索引".to_string(),
        ExifTag::InteroperabilityVersion(_) => "互操作性版本".to_string(),
        ExifTag::CreateDate(_) => "文件创建时间".to_string(),
        ExifTag::DateTimeOriginal(_) => "拍摄时间".to_string(),
        ExifTag::UserComment(_) => "用户注释".to_string(),
        ExifTag::ExposureProgram(_) => "曝光程序".to_string(),
        ExifTag::SpectralSensitivity(_) => "光谱灵敏度".to_string(),
        ExifTag::OECF(_) => "光电转换函数".to_string(),
        ExifTag::SensitivityType(_) => "灵敏度类型".to_string(),
        ExifTag::StandardOutputSensitivity(_) => "标准输出灵敏度".to_string(),
        ExifTag::RecommendedExposureIndex(_) => "推荐曝光指数".to_string(),
        ExifTag::ISOSpeed(_) => "ISO 速度".to_string(),
        ExifTag::ISOSpeedLatitudeyyy(_) => "ISO 速度纬度 yyy".to_string(),
        ExifTag::ISOSpeedLatitudezzz(_) => "ISO 速度纬度 zzz".to_string(),
        ExifTag::ExifVersion(_) => "EXIF 版本".to_string(),
        ExifTag::OffsetTime(_) => "偏移时间".to_string(),
        ExifTag::OffsetTimeOriginal(_) => "原始偏移时间".to_string(),
        ExifTag::OffsetTimeDigitized(_) => "数字化偏移时间".to_string(),
        ExifTag::ComponentsConfiguration(_) => "组件配置".to_string(),
        ExifTag::CompressedBitsPerPixel(_) => "每像素压缩位数".to_string(),
        ExifTag::ShutterSpeedValue(_) => "快门速度值".to_string(),
        ExifTag::ApertureValue(_) => "光圈值".to_string(),
        ExifTag::BrightnessValue(_) => "亮度值".to_string(),
        ExifTag::ExposureCompensation(_) => "曝光补偿".to_string(),
        ExifTag::MaxApertureValue(_) => "最大光圈值".to_string(),
        ExifTag::SubjectDistance(_) => "主体距离".to_string(),
        ExifTag::MeteringMode(_) => "测光模式".to_string(),
        ExifTag::LightSource(_) => "光源".to_string(),
        ExifTag::Flash(_) => "闪光灯".to_string(),
        ExifTag::SubjectArea(_) => "主体区域".to_string(),
        ExifTag::MakerNote(_) => "制造商注释".to_string(),
        ExifTag::SubSecTime(_) => "亚秒时间".to_string(),
        ExifTag::SubSecTimeOriginal(_) => "原始亚秒时间".to_string(),
        ExifTag::SubSecTimeDigitized(_) => "数字化亚秒时间".to_string(),
        ExifTag::AmbientTemperature(_) => "环境温度".to_string(),
        ExifTag::Humidity(_) => "湿度".to_string(),
        ExifTag::Pressure(_) => "压力".to_string(),
        ExifTag::WaterDepth(_) => "水深".to_string(),
        ExifTag::Acceleration(_) => "加速度".to_string(),
        ExifTag::CameraElevationAngle(_) => "相机仰角".to_string(),
        ExifTag::FlashpixVersion(_) => "Flashpix 版本".to_string(),
        ExifTag::ColorSpace(_) => "色彩空间".to_string(),
        ExifTag::ExifImageWidth(_) => "EXIF 图像宽度".to_string(),
        ExifTag::ExifImageHeight(_) => "EXIF 图像高度".to_string(),
        ExifTag::RelatedSoundFile(_) => "相关音频文件".to_string(),
        ExifTag::FlashEnergy(_) => "闪光灯能量".to_string(),
        ExifTag::SpatialFrequencyResponse(_) => "空间频率响应".to_string(),
        ExifTag::FocalPlaneXResolution(_) => "焦平面 X 分辨率".to_string(),
        ExifTag::FocalPlaneYResolution(_) => "焦平面 Y 分辨率".to_string(),
        ExifTag::FocalPlaneResolutionUnit(_) => "焦平面分辨率单位".to_string(),
        ExifTag::SubjectLocation(_) => "主体位置".to_string(),
        ExifTag::ExposureIndex(_) => "曝光指数".to_string(),
        ExifTag::SensingMethod(_) => "感应方法".to_string(),
        ExifTag::FileSource(_) => "文件来源".to_string(),
        ExifTag::SceneType(_) => "场景类型".to_string(),
        ExifTag::CFAPattern(_) => "CFA 模式".to_string(),
        ExifTag::CustomRendered(_) => "自定义渲染".to_string(),
        ExifTag::ExposureMode(_) => "曝光模式".to_string(),
        ExifTag::WhiteBalance(_) => "白平衡".to_string(),
        ExifTag::DigitalZoomRatio(_) => "数字变焦比率".to_string(),
        ExifTag::FocalLengthIn35mmFormat(_) => "35mm 等效焦距".to_string(),
        ExifTag::SceneCaptureType(_) => "场景捕捉类型".to_string(),
        ExifTag::GainControl(_) => "增益控制".to_string(),
        ExifTag::Contrast(_) => "对比度".to_string(),
        ExifTag::Saturation(_) => "饱和度".to_string(),
        ExifTag::Sharpness(_) => "锐度".to_string(),
        ExifTag::DeviceSettingDescription(_) => "设备设置描述".to_string(),
        ExifTag::SubjectDistanceRange(_) => "主体距离范围".to_string(),
        ExifTag::ImageUniqueID(_) => "图像唯一 ID".to_string(),
        ExifTag::OwnerName(_) => "所有者名称".to_string(),
        ExifTag::SerialNumber(_) => "序列号".to_string(),
        ExifTag::LensInfo(_) => "镜头信息".to_string(),
        ExifTag::LensMake(_) => "镜头制造商".to_string(),
        ExifTag::LensModel(_) => "镜头型号".to_string(),
        ExifTag::LensSerialNumber(_) => "镜头序列号".to_string(),
        ExifTag::CompositeImage(_) => "合成图像".to_string(),
        ExifTag::CompositeImageCount(_) => "合成图像数量".to_string(),
        ExifTag::CompositeImageExposureTimes(_) => "合成图像曝光时间".to_string(),
        ExifTag::Gamma(_) => "伽马".to_string(),
        _ => "未定义标签".to_string(),
    }
}


fn return_ori_val_16(vec: &[u16]) -> String {
    if let Some(&value) = vec.first() {
        return format!("{}", value);
    } else {
        return "None".to_string();
    }
}

fn return_ori_val_32(vec: &[u32]) -> String {
    if let Some(&value) = vec.first() {
        return format!("{}", value);
    } else {
        return "None".to_string();
    }
}

/// 获取标签值的字符串表示
/// ### 参数
/// - `tag`: EXIF 标签
/// - `_endian`: 字节序信息（当前未使用）
/// ### 返回值
/// - 格式化后的标签值字符串
fn get_tag_value_string(tag: &ExifTag, _endian: &Endian) -> String {
    // 处理字符串类型的标签
    if let 
        ExifTag::Make(s) | 
        ExifTag::Model(s) | 
        ExifTag::Software(s) |
        ExifTag::Artist(s) | 
        ExifTag::Copyright(s) | 
        ExifTag::ImageDescription(s) |
        ExifTag::CreateDate(s) | 
        ExifTag::DateTimeOriginal(s) | 
        ExifTag::ModifyDate(s) |
        ExifTag::GPSLatitudeRef(s) | 
        ExifTag::GPSLongitudeRef(s) |
        ExifTag::GPSSatellites(s) |
        ExifTag::GPSStatus(s) |
        ExifTag::GPSMeasureMode(s) |
        ExifTag::GPSSpeedRef(s) |
        ExifTag::GPSTrackRef(s) |
        ExifTag::GPSImgDirectionRef(s) |
        ExifTag::GPSMapDatum(s) |
        ExifTag::GPSDestLatitudeRef(s) |
        ExifTag::GPSDestLongitudeRef(s) |
        ExifTag::GPSDestBearingRef(s) |
        ExifTag::GPSDestDistanceRef(s) |
        ExifTag::GPSDateStamp(s) |
        ExifTag::InteroperabilityIndex(s) |
        ExifTag::OffsetTime(s) |
        ExifTag::OffsetTimeOriginal(s) |
        ExifTag::OffsetTimeDigitized(s) |
        ExifTag::SpectralSensitivity(s) |
        ExifTag::RelatedSoundFile(s) |
        ExifTag::OwnerName(s) |
        ExifTag::LensMake(s) |
        ExifTag::LensModel(s) |
        ExifTag::LensSerialNumber(s) |
        ExifTag::ImageUniqueID(s) = tag {
        let trimmed = s.trim_matches('\0').trim();
        return if trimmed.is_empty() { "空值".to_string() } else { trimmed.to_string() };
    }

    // 处理注释类型标签
    if let
        ExifTag::UserComment(v) |
        ExifTag::MakerNote(v) |
        ExifTag::OECF(v) |
        ExifTag::GPSProcessingMethod(v) |
        ExifTag::GPSAreaInformation(v) |
        ExifTag::InteroperabilityVersion(v) |
        ExifTag::OECF(v) |
        ExifTag::ExifVersion(v) |
        ExifTag::FileSource(v) |
        ExifTag::SceneType(v) |
        ExifTag::CFAPattern(v) |
        ExifTag::DeviceSettingDescription(v) |
        ExifTag::CompositeImageExposureTimes(v) = tag {
        if let Ok(s) = std::str::from_utf8(v) {
            let trimmed = s.trim_matches('\0').trim();
            return if trimmed.is_empty() { "空值".to_string() } else { trimmed.to_string() };
        } else {
            return format!("字节数据: {:x?}", &v[..12.min(v.len())]);
        }
    }

    // 处理整数部分
    if let
        ExifTag::GPSVersionID(vec) |
        ExifTag::GPSAltitudeRef(vec) = tag {
        if let Some(&value) = vec.first() {
            return match tag {
                ExifTag::GPSVersionID(_) => {
                    // GPS版本ID通常以4个字节表示，如[2, 2, 0, 0]
                    if vec.len() >= 4 {
                        format!("{}.{}.{}.{}", vec[0], vec[1], vec[2], vec[3])
                    } else {
                        format!("GPS版本: {:?}", vec)
                    }
                }
                ExifTag::GPSAltitudeRef(_) => {
                    // GPS海拔参考：0=高于海平面，1=低于海平面
                    match value {
                        0 => "高于海平面".to_string(),
                        1 => "低于海平面".to_string(),
                        _ => format!("未知海拔参考: {}", value)
                    }
                }
                _ => format!("{}", value)
            };
        } else {
            return "空值".to_string();
        }
    }

    if let
        ExifTag::Orientation(vec) |
        ExifTag::ResolutionUnit(vec) |
        ExifTag::SamplesPerPixel(vec) |
        ExifTag::PlanarConfiguration(vec) |
        ExifTag::Compression(vec) |
        ExifTag::PhotometricInterpretation(vec) |
        ExifTag::CellWidth(vec) |
        ExifTag::CellHeight(vec) |
        ExifTag::BitsPerSample(vec) |
        ExifTag::TransferFunction(vec) |
        ExifTag::YCbCrSubSampling(vec) |
        ExifTag::YCbCrPositioning(vec) |
        ExifTag::ExposureProgram(vec) |
        ExifTag::SensitivityType(vec) |
        ExifTag::MeteringMode(vec) |
        ExifTag::LightSource(vec) |
        ExifTag::Flash(vec) |
        ExifTag::ColorSpace(vec) |
        ExifTag::FocalPlaneResolutionUnit(vec) |
        ExifTag::SensingMethod(vec) |
        ExifTag::CustomRendered(vec) |
        ExifTag::ExposureMode(vec) |
        ExifTag::WhiteBalance(vec) |
        ExifTag::FocalLengthIn35mmFormat(vec) |
        ExifTag::SceneCaptureType(vec) |
        ExifTag::GainControl(vec) |
        ExifTag::Contrast(vec) |
        ExifTag::Saturation(vec) |
        ExifTag::Sharpness(vec) |
        ExifTag::SubjectDistanceRange(vec) |
        ExifTag::CompositeImage(vec) |
        ExifTag::CompositeImageCount(vec) |
        ExifTag::GPSDifferential(vec) = tag {
        if let Some(&value) = vec.first() {
            return match tag {
                ExifTag::WhiteBalance(_) => {
                    // 白平衡
                    match value {
                        0 => "自动".to_string(),
                        1 => "手动".to_string(),
                        _ => format!("未知白平衡: {}", value)
                    }
                }
                ExifTag::CompositeImageCount(_) => {
                    // 合成图像数量，有两个组件
                    if vec.len() >= 2 {
                        format!("{} 张图像中的第 {} 张", vec[1], vec[0])
                    } else {
                        format!("{:?}", vec)
                    }
                }
                ExifTag::Orientation(_) => {
                    // 方向
                    match value {
                        1 => "正常".to_string(),
                        2 => "水平翻转".to_string(),
                        3 => "旋转180°".to_string(),
                        4 => "垂直翻转".to_string(),
                        5 => "水平翻转并逆时针旋转90°".to_string(),
                        6 => "旋转90°".to_string(),
                        7 => "垂直翻转并顺时针旋转90°".to_string(),
                        8 => "顺时针旋转90°".to_string(),
                        _ => format!("未知方向: {}", value)
                    }
                }
                ExifTag::ResolutionUnit(_) => {
                    // 分辨率单位
                        match value {
                            1 => "无单位",
                            2 => "英寸（DPI）",
                            3 => "厘米（DPC）",
                            _ => "未知单位",
                        }.to_string()
                }
                ExifTag::SamplesPerPixel(vec) => {
                    // 每个像素的样本数
                    return_ori_val_16(vec)
                }
                ExifTag::PlanarConfiguration(vec) => {
                    // 平面配置
                    if let Some(&value) = vec.first() {
                        return match value {
                            1 => "连续".to_string(),
                            2 => "分离".to_string(),
                            _ => format!("未知平面配置: {}", value)
                        };
                    } else {
                        return "空值".to_string();
                    }
                }
                ExifTag::Compression(vec) => {
                    // 压缩
                    if let Some(&value) = vec.first() {
                        return match value {
                            1 => "无压缩".to_string(),
                            2 => "CCITT 1D".to_string(),
                            3 => "T4/Group 3 Fax".to_string(),
                            4 => "T6/Group 4 Fax".to_string(),
                            5 => "LZW".to_string(),
                            6 => "JPEG (旧式)".to_string(),
                            7 => "JPEG".to_string(),
                            8 => "Deflate".to_string(),
                            9 => "JBIG B&W".to_string(),
                            10 => "JBIG Color".to_string(),
                            32773 => "PackBits".to_string(),
                            _ => format!("未知压缩方式: {}", value)
                        };
                    } else {
                        return "空值".to_string();
                    }
                }
                ExifTag::PhotometricInterpretation(vec) => {
                    // 光度解释
                    if let Some(&value) = vec.first() {
                        return match value {
                            0 => "白底黑图".to_string(),
                            1 => "黑底白图".to_string(),
                            2 => "RGB".to_string(),
                            3 => "调色板".to_string(),
                            4 => "透明蒙版".to_string(),
                            5 => "CMYK".to_string(),
                            6 => "YCbCr".to_string(),
                            8 => "CIELab".to_string(),
                            _ => format!("未知光度解释: {}", value)
                        };
                    } else {
                        return "空值".to_string();
                    }
                }
                ExifTag::CellWidth(vec) => {
                    // 单元格宽度
                    if let Some(&value) = vec.first() {
                        return format!("{}", value);
                    } else {
                        return "空值".to_string();
                    }
                }
                ExifTag::CellHeight(vec) => {
                    // 单元格高度
                    return_ori_val_16(vec)
                }
                ExifTag::BitsPerSample(vec) => {
                    // 每个样本的位数
                    return_ori_val_16(vec)
                }
                ExifTag::TransferFunction(vec) => {
                    // 转移函数
                    return_ori_val_16(vec)
                }
                ExifTag::YCbCrSubSampling(vec) => {
                    // YCbCr 子采样
                    return_ori_val_16(vec)
                }
                ExifTag::YCbCrPositioning(vec) => {
                    // YCbCr 位置
                    return_ori_val_16(vec)
                }
                ExifTag::ExposureProgram(_) => {
                    // 曝光程序
                    match value {
                        0 => "未定义".to_string(),
                        1 => "手动".to_string(),
                        2 => "标准程序".to_string(),
                        3 => "光圈优先".to_string(),
                        4 => "快门优先".to_string(),
                        5 => "创意程序".to_string(),
                        6 => "动作程序".to_string(),
                        7 => "肖像模式".to_string(),
                        8 => "风景模式".to_string(),
                        _ => format!("未知曝光程序: {}", value)
                    }
                }
                ExifTag::SensitivityType(_) => {
                    // 感光度类型
                    match value {
                        0 => "未知".to_string(),
                        1 => "标准输出感光度".to_string(),
                        2 => "推荐曝光指数".to_string(),
                        3 => "ISO感光度".to_string(),
                        4 => "标准输出感光度和推荐曝光指数".to_string(),
                        5 => "标准输出感光度和ISO感光度".to_string(),
                        6 => "推荐曝光指数和ISO感光度".to_string(),
                        7 => "标准输出感光度、推荐曝光指数和ISO感光度".to_string(),
                        _ => format!("未知感光度类型: {}", value)
                    }
                }
                ExifTag::MeteringMode(_) => {
                    // 测光模式
                    match value {
                        0 => "未知".to_string(),
                        1 => "平均测光".to_string(),
                        2 => "中央重点平均测光".to_string(),
                        3 => "点测光".to_string(),
                        4 => "多点测光".to_string(),
                        5 => "模式".to_string(),
                        6 => "局部测光".to_string(),
                        255 => "其他".to_string(),
                        _ => format!("未知测光模式: {}", value)
                    }
                }
                ExifTag::LightSource(_) => {
                    match value {
                        0 => "未知".to_string(),
                        1 => "日光".to_string(),
                        2 => "荧光灯".to_string(),
                        3 => "钨丝灯".to_string(),
                        4 => "闪光灯".to_string(),
                        9 => "晴朗天气".to_string(),
                        10 => "阴天".to_string(),
                        11 => "阴影".to_string(),
                        12 => "日光色荧光灯".to_string(),
                        13 => "日光白色荧光灯".to_string(),
                        14 => "冷白荧光灯".to_string(),
                        15 => "白荧光灯".to_string(),
                        17 => "标准灯光A".to_string(),
                        18 => "标准灯光B".to_string(),
                        19 => "标准灯光C".to_string(),
                        20 => "D55".to_string(),
                        21 => "D65".to_string(),
                        22 => "D75".to_string(),
                        23 => "D50".to_string(),
                        24 => "ISO工作室钨灯".to_string(),
                        255 => "其他".to_string(),
                        _ => format!("未知光源: {}", value)
                    }
                }
                ExifTag::Flash(_) => {
                    // 闪光灯
                    match value {
                        0x00 => "未闪光".to_string(),
                        0x01 => "闪光".to_string(),
                        0x05 => "闪光但未检测到反射光".to_string(),
                        0x07 => "闪光且检测到反射光".to_string(),
                        0x09 => "强制闪光".to_string(),
                        0x0D => "强制闪光但未检测到反射光".to_string(),
                        0x0F => "强制闪光且检测到反射光".to_string(),
                        0x10 => "未闪光，强制闪光模式".to_string(),
                        0x18 => "自动，未闪光".to_string(),
                        0x19 => "自动，闪光".to_string(),
                        0x1D => "自动，闪光但未检测到反射光".to_string(),
                        0x1F => "自动，闪光且检测到反射光".to_string(),
                        0x20 => "无闪光功能".to_string(),
                        0x41 => "闪光，防红眼模式".to_string(),
                        0x45 => "闪光，防红眼模式，未检测到反射光".to_string(),
                        0x47 => "闪光，防红眼模式，检测到反射光".to_string(),
                        0x49 => "强制闪光，防红眼模式".to_string(),
                        0x4D => "强制闪光，防红眼模式，未检测到反射光".to_string(),
                        0x4F => "强制闪光，防红眼模式，检测到反射光".to_string(),
                        0x59 => "自动，闪光，防红眼模式".to_string(),
                        0x5D => "自动，闪光，防红眼模式，未检测到反射光".to_string(),
                        0x5F => "自动，闪光，防红眼模式，检测到反射光".to_string(),
                        _ => format!("未知闪光模式: 0x{:04X}", value)
                    }
                }
                ExifTag::ColorSpace(_) => {
                    // 色彩空间
                    match value {
                        1 => "sRGB".to_string(),
                        2 => "Adobe RGB".to_string(),
                        65535 => "未校准".to_string(),
                        _ => format!("未知色彩空间: {}", value)
                    }
                }
                ExifTag::FocalPlaneResolutionUnit(_) => {
                    // 分辨率单位
                    match value {
                        1 => "无单位".to_string(),
                        2 => "英寸".to_string(),
                        3 => "厘米".to_string(),
                        _ => format!("未知分辨率单位: {}", value)
                    }
                }
                ExifTag::SceneCaptureType(_) => {
                    // 场景类型
                    match value {
                        0 => "标准".to_string(),
                        1 => "风景".to_string(),
                        2 => "肖像".to_string(),
                        3 => "夜景".to_string(),
                        _ => format!("未知场景类型: {}", value)
                    }
                }
                ExifTag::GainControl(_) => {
                    // 增益控制
                    match value {
                        0 => "无".to_string(),
                        1 => "低增益".to_string(),
                        2 => "高增益".to_string(),
                        _ => format!("未知增益控制: {}", value)
                    }
                }
                ExifTag::Contrast(_) => {
                    // 对比度
                    match value {
                        0 => "正常".to_string(),
                        1 => "柔和".to_string(),
                        2 => "强烈".to_string(),
                        _ => format!("未知对比度: {}", value)
                    }
                }
                ExifTag::Saturation(_) => {
                    // 饱和度
                    match value {
                        0 => "正常".to_string(),
                        1 => "低饱和度".to_string(),
                        2 => "高饱和度".to_string(),
                        _ => format!("未知饱和度: {}", value)
                    }
                }
                ExifTag::Sharpness(_) => {
                    // 锐度
                    match value {
                        0 => "正常".to_string(),
                        1 => "柔和".to_string(),
                        2 => "强烈".to_string(),
                        _ => format!("未知锐度: {}", value)
                    }
                }
                ExifTag::SubjectDistanceRange(_) => {
                    // 主体距离范围
                    match value {
                        0 => "未知".to_string(),
                        1 => "宏观".to_string(),
                        2 => "近距离".to_string(),
                        3 => "远距离".to_string(),
                        _ => format!("未知主体距离范围: {}", value)
                    }
                }
                ExifTag::CompositeImage(values) => {
                    // 合成图像
                    match value {
                        0 => "非合成图像".to_string(),
                        1 => "合成图像".to_string(),
                        _ => format!("未知合成图像类型: {}", value)
                    }
                }
                ExifTag::CompositeImageCount(values) => {
                    // 合成图像数量
                    if values.len() >= 2 {
                        format!("{} 张图像中的第 {} 张", values[1], values[0])
                    } else {
                        "无效的合成图像数量".to_string()
                    }
                }
                ExifTag::GPSDifferential(values) => {
                    // GPS 差分
                    match value {
                        0 => "无差分".to_string(),
                        1 => "差分修正".to_string(),
                        _ => format!("未知差分状态: {}", value)
                    }
                }
                _ => format!("{}", value)
            };
        } else {
            return "空值".to_string();
        }
    }

    if let
        ExifTag::ImageWidth(vec) |
        ExifTag::ImageHeight(vec) |
        ExifTag::RowsPerStrip(vec) |
        ExifTag::ExifOffset(vec) = tag {
        if let Some(&value) = vec.first() {
            return match tag {
                ExifTag::ImageWidth(_) | ExifTag::ImageHeight(_) => format!("{:.0} px", value),
                ExifTag::RowsPerStrip(vec) => {
                    // 每行像素数
                    if let Some(&value) = vec.first() {
                        return format!("{} 行/带", value);
                    } else {
                        return "空值".to_string();
                    }
                }


                _ => format!("{}", value)
            };
        } else {
            return "空值".to_string();
        }
    }

    // 处理有理数类型的标签（光圈、焦距、曝光时间等）
    if let ExifTag::LensInfo(vec) |
        ExifTag::FNumber(vec) | 
        ExifTag::FocalLength(vec) | 
        ExifTag::ExposureTime(vec) |
        ExifTag::XResolution(vec) | 
        ExifTag::YResolution(vec) | 
        ExifTag::GPSLatitude(vec) |
        ExifTag::GPSLongitude(vec) | 
        ExifTag::GPSAltitude(vec) = tag {
        if let Some(rational) = vec.first() {
            let value = (rational.nominator as f64) / (rational.denominator as f64);
            return match tag {
                ExifTag::LensInfo(_) => {
                    // 处理镜头信息标签
                    if vec.len() >= 4 {
                        let min_focal = (vec[0].nominator as f64) / (vec[0].denominator as f64);
                        let max_focal = (vec[1].nominator as f64) / (vec[1].denominator as f64);
                        let min_aperture = (vec[2].nominator as f64) / (vec[2].denominator as f64);
                        let max_aperture = (vec[3].nominator as f64) / (vec[3].denominator as f64);
                        format!("{:.0}-{:.0}mm f/{:.1}-{:.1}", 
                            min_focal, max_focal, min_aperture, max_aperture)
                    } else {
                        format!("{:.4}", value)
                    }
                }
                ExifTag::FNumber(_) => format!("f/{:.1}", value),
                ExifTag::FocalLength(_) => format!("{:.1} mm", value),
                ExifTag::ExposureTime(_) => {
                    if (0.0..1.0).contains(&value) && value > 0.0 {
                        format!("1/{:.0} 秒", (1.0f64 / value).round())
                    } else {
                        format!("{:.1} 秒", value)
                    }
                }
                
                ExifTag::XResolution(_) | ExifTag::YResolution(_) => format!("{:.0} DPI", value),
                ExifTag::GPSLatitude(_) | ExifTag::GPSLongitude(_) => {
                    if vec.len() >= 3 {
                        let degrees = (vec[0].nominator as f64) / (vec[0].denominator as f64);
                        let minutes = (vec[1].nominator as f64) / (vec[1].denominator as f64);
                        let seconds = (vec[2].nominator as f64) / (vec[2].denominator as f64);
                        format!("{:.0}°{:.0}'{:.2}''", degrees, minutes, seconds)
                    } else {
                        format!("{:.6}", value)
                    }
                }
                ExifTag::GPSAltitude(_) => format!("{:.1} 米", value),
                _ => format!("{:.4}", value),
            };
        }
    }
    
    // 处理 ISO 感光度标签
    if let ExifTag::ISO(vec) = tag {
        if let Some(&value) = vec.first() {
            return format!("{}", value);
        }
    }

    // 处理图像尺寸标签
    if let ExifTag::ImageWidth(vec) = tag {
        if let Some(&value) = vec.first() {
            return format!("{} 像素", value);
        }
    }
    if let ExifTag::ImageHeight(vec) = tag {
        if let Some(&value) = vec.first() {
            return format!("{} 像素", value);
        }
    }

    // 处理 GPS 时间戳标签
    if let ExifTag::GPSTimeStamp(vec) = tag {
        if vec.len() >= 3 {
            let h = (vec[0].nominator as f64) / (vec[0].denominator as f64);
            let m = (vec[1].nominator as f64) / (vec[1].denominator as f64);
            let s = (vec[2].nominator as f64) / (vec[2].denominator as f64);
            return format!("{:02.0}:{:02.0}:{:.1}", h, m, s);
        }
    }

    // 处理未知标签
    if let ExifTag::UnknownINT8S(_, id, bytes) = tag {
        return format!("未知标签 (0x{:04X}): {:?}", id, bytes);
    }

    return format!("解析错误：{:?}", tag);

    // "无法解析".to_string()
}

/// 显示关键 EXIF 信息摘要
/// 
/// # 参数
/// - `metadata`: 元数据对象
/// - `endian`: 字节序信息
fn display_summary_info(metadata: &Metadata, endian: &Endian) {
    println!("\n--- 关键信息 ---");
    println!("制造商: {}", get_tag_value(metadata, &ExifTag::Make(String::new()), endian));
    println!("相机型号: {}", get_tag_value(metadata, &ExifTag::Model(String::new()), endian));
    println!("软件: {}", get_tag_value(metadata, &ExifTag::Software(String::new()), endian));
    println!("图片宽度：{}", get_tag_value(metadata, &ExifTag::ImageWidth(vec![]), endian));
    println!("图片高度: {}", get_tag_value(metadata, &ExifTag::ImageHeight(vec![]), endian));
    println!("光圈: {}", get_tag_value(metadata, &ExifTag::FNumber(vec![]), endian));
    println!("ISO: {}", get_tag_value(metadata, &ExifTag::ISO(vec![]), endian));
    let shoot_time = get_tag_value(metadata, &ExifTag::DateTimeOriginal(String::new()), endian);
    if shoot_time != "未找到" {
        println!("拍摄时间: {}", shoot_time);
    } else {
        println!("创建时间: {}", get_tag_value(metadata, &ExifTag::CreateDate(String::new()), endian));
    }
}

/// 获取特定标签的值
/// 
/// # 参数
/// - `metadata`: 元数据对象
/// - `exif_tag`: 要查找的标签类型
/// - `endian`: 字节序信息
/// 
/// # 返回值
/// - 标签值的字符串表示，如果未找到则返回 "未找到"
fn get_tag_value(metadata: &Metadata, exif_tag: &ExifTag, endian: &Endian) -> String {
    metadata.get_tag(exif_tag)
        .next()
        .map(|tag| get_tag_value_string(tag, endian))
        .unwrap_or_else(|| "未找到".to_string())
}

/// 将元数据中的所有标签写入 XMP XML 文件
/// 
/// # 参数
/// - `path`: 原始图片文件路径
/// - `metadata`: 元数据对象
/// - `endian`: 字节序信息
/// 
/// # 返回值
/// - IO 操作结果
fn write_xmp_from_metadata(path: &Path, metadata: &Metadata, endian: &Endian) -> std::io::Result<()> {
    let mut items: Vec<(String,String)> = Vec::new();
    // println!("{:#?}", &metadata);
    // 收集所有标签的名称和值
    for tag in metadata {
        let name = get_human_readable_tag_name(&tag);
        let value = get_tag_value_string(&tag, endian);
        items.push((name, value));
    }

    // 创建 XMP 文件路径
    let xml_path = path.with_extension("xmp.xml");
    let mut f = File::create(&xml_path)?;
    
    // 写入 XML 头部和根元素
    writeln!(f, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(f, r#"<ExifFromMetadata source="{}">"#, path.file_name().and_then(|s| s.to_str()).unwrap_or(""))?;
    
    // 写入每个标签
    for (k, v) in items {
        writeln!(f, r#"  <tag name="{}"><![CDATA[{}]]></tag>"#, xml_escape_attr(&k), v)?;
    }
    
    writeln!(f, "</ExifFromMetadata>")?;
    Ok(())
}

/// 直接打印 XMP XML 文件内容（用于回退显示）
/// 
/// # 参数
/// - `xmp_path`: XMP 文件路径
/// 
/// # 返回值
/// - 操作结果
fn print_xmp_and_display_all(xmp_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(xmp_path)?;
    println!("\n=== 来自 {} 的 XMP / EXIF 信息 ===\n", xmp_path.display());
    println!("{}", s);
    Ok(())
}

/// 解析 XMP XML 文件并映射为 EXIF 标签列表
/// 
/// ### 参数
/// - `xmp_path`: XMP 文件路径
/// 
/// ### 返回值
/// - 如果成功解析则返回 Some(Vec<ExifTag>)，否则返回 None
fn parse_xmp_to_exif_tags(xmp_path: &Path) -> Option<Vec<ExifTag>> {

    // std::fs::read_to_string(xmp_path): 
    //     尝试读取指定路径（xmp_path）的文件内容，返回一个字符串 Result<String, std::io::Error>
    // .ok(): 
    //     将 Result 类型转换为 Option 类型，如果成功则包含文件内容，失败则为 None
    // ?: 
    //     错误传播操作符，如果结果是 None，则立即从当前函数返回 None，如果是 Some(value)，则解包并继续执行
    let s = std::fs::read_to_string(xmp_path).ok()?;

    let mut reader = Reader::from_str(&s); // 从 s 创建一个 Reader 实例
    reader.trim_text(true); // 表示启用文本修剪功能，去除文本内容两端的空白字符

    let mut buf = Vec::new();
    let mut out: Vec<ExifTag> = Vec::new(); // 专门用于存储 ExifTag 类型的元素 

    // 解析 XML 事件
    // 调用 Reader，read_event_into 尝试读取一个事件到缓冲区 buf 中，返回类型是 Result<Event, Error>
    // Ok(ev): 只匹配 Ok，ev 将包含读取到的事件
    while let Ok(ev) = reader.read_event_into(&mut buf) {
        // println!("event: {:#?}", ev);
        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => {
                // println!("ref: {:#?}", e);
                // 检查元素是否是rdf:Description类型的元素
                // e.local_name(): 获取元素的本地名称（不带命名空间前缀）
                // as_ref(): 将引用转换为字节切片
                // ends_with(b"Description"): 检查是否以"Description"结尾
                if e.local_name().as_ref().ends_with(b"Description") {
                    // e.attributes(): 获取元素的所有属性
                    for attr_res in e.attributes() {
                        // println!("attr_res: {:?}", attr_res);
                        if let Ok(attr) = attr_res {
                            // String::from_utf8_lossy(): 安全地将字节转换为UTF-8字符串
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            // attr.unescape_value(): 解码XML属性值中的转义字符
                            let val = attr.unescape_value().unwrap_or_default().to_string();
                            // 调用map_xmp_key_value_into_tag函数处理这些键值对
                            map_xmp_key_value_into_tag(&key, &val, &mut out);
                            // println!("key: {},          val: {}", key, val);
                        }
                    }
                }
                else if e.local_name().as_ref() == (b"ISOSpeedRatings") {
                    
                }
            }
            Event::Text(e) => {
                // 处理文本内容（当前实现较简单，主要依赖属性映射）
                if let Ok(text) = e.unescape() {
                    let _parent = reader.buffer_position();
                    let _text_content = text.to_string();
                    // 可以在此处添加对文本内容的处理逻辑
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    if out.is_empty() { None } else { Some(out) }
}

/// 将 XMP 键值对映射为 EXIF 标签
/// 
/// ### 参数
/// - `key`: XMP 键名
/// - `val`: XMP 值
/// - `out`: 输出的 EXIF 标签列表
fn map_xmp_key_value_into_tag(key: &str, val: &str, out: &mut Vec<ExifTag>) {
    let k = key.split(':').last().unwrap_or(key).to_lowercase();
    let v = val.trim();
    if v.is_empty() { return; }

    // println!("{}", k);
    // 根据键名映射到对应的 EXIF 标签类型
    match k.as_str() {
        "make" => out.push(ExifTag::Make(v.to_string())),
        "model" => out.push(ExifTag::Model(v.to_string())),
        "creatortool" | "software" => out.push(ExifTag::Software(v.to_string())),
        "createdate" | "creatordate" => out.push(ExifTag::CreateDate(v.to_string())),
        "datetimeoriginal" => out.push(ExifTag::DateTimeOriginal(v.to_string())),
        "modifydate" => out.push(ExifTag::ModifyDate(v.to_string())),
        "copyright" => out.push(ExifTag::Copyright(v.to_string())),
        "artist" => out.push(ExifTag::Artist(v.to_string())),
        "imagedescription" => out.push(ExifTag::ImageDescription(v.to_string())),
        "isospeedratings" | "iso" | "recommendedexposureindex" => { // ISO 相关
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::ISO(vec![n]));
            } else if let Some(n) = extract_first_number(v) {
                out.push(ExifTag::ISO(vec![n as u16]));
            }
        }
        "fnumber" | "aperturevalue" => { // 光圈相关
            if let Some(r) = parse_fraction_to_rational(v) {
                out.push(ExifTag::FNumber(vec![r]));
            }
        }
        "focallength" => { // 焦距相关
            if let Some(r) = parse_fraction_to_rational(v) {
                out.push(ExifTag::FocalLength(vec![r]));
            }
        }
        "exposuretime" | "shutterspeedvalue" => { // 曝光时间相关
            if let Some(r) = parse_fraction_to_rational(v) {
                out.push(ExifTag::ExposureTime(vec![r]));
            }
        }
        "xresolution" => { // 分辨率相关
            if let Some(r) = parse_fraction_to_rational(v) {
                out.push(ExifTag::XResolution(vec![r]));
            }
        }
        "yresolution" => {
            if let Some(r) = parse_fraction_to_rational(v) {
                out.push(ExifTag::YResolution(vec![r]));
            }
        }
        "orientation" => { // 方向
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::Orientation(vec![n]));
            }
        }
        "resolutionunit" => { // 分辨率单位
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::ResolutionUnit(vec![n]));
            }
        }
        "exposureprogram" => { // 曝光程序
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::ExposureProgram(vec![n]));
            }
        }
        "meteringmode" => { // 测光模式
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::MeteringMode(vec![n]));
            }
        }
        "flash" => { // 闪光灯
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::Flash(vec![n]));
            }
        }
       
        "whitebalance" => { // 白平衡
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::WhiteBalance(vec![n]));
            }
        }
        "focallengthin35mmfilm" => { // 35mm等效焦距
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::FocalLengthIn35mmFormat(vec![n]));
            }
        }
        "scenecapturetype" => { // 场景捕捉类型
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::SceneCaptureType(vec![n]));
            }
        }
        "contrast" => { // 对比度
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::Contrast(vec![n]));
            }
        }
        "saturation" => { // 饱和度
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::Saturation(vec![n]));
            }
        } 
        "sharpness" => { // 锐度
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::Sharpness(vec![n]));
            }
        }
        "serialnumber" => out.push(ExifTag::SerialNumber(v.to_string())), // 序列号
        "lensinfo" => { // 镜头信息
            // 解析镜头信息，格式如 "700/10 2100/10 40/10 56/10"
            let parts: Vec<&str> = v.split_whitespace().collect();
            let mut rationals = Vec::new();
            for part in parts {
                if let Some(r) = parse_fraction_to_rational(part) {
                    rationals.push(r);
                }
            }
            if !rationals.is_empty() {
                out.push(ExifTag::LensInfo(rationals));
            }
        }
        "lensmake" => out.push(ExifTag::LensMake(v.to_string())), // 镜头制造商
        "lensmodel" => out.push(ExifTag::LensModel(v.to_string())), // 镜头型号
        "maxaperturevalue" => { // 最大光圈值
            if let Some(r) = parse_fraction_to_rational(v) {
                out.push(ExifTag::MaxApertureValue(vec![r]));
            }
        }
        
        _ => {
            // 其他未映射的字段可以在此处添加处理逻辑
            // 可以添加调试输出查看未处理的键
            println!("未处理的 XMP 键: {} = {}", key, v);
        }
    }
}

/// 从字符串中提取第一个数字
/// 
/// # 参数
/// - `s`: 输入字符串
/// 
/// # 返回值
/// - 提取的数字，如果提取失败则返回 None
fn extract_first_number(s: &str) -> Option<f64> {
    let mut num = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' || c == '-' {
            num.push(c);
        } else if !num.is_empty() {
            break;
        }
    }
    num.parse::<f64>().ok()
}

/// 将分数字符串解析为有理数
/// ### 参数
/// - `s`: 分数字符串（如 "1/125"）
/// ### 返回值
/// - 解析后的有理数，如果解析失败则返回 None
fn parse_fraction_to_rational(s: &str) -> Option<uR64> {
    let s = s.trim();

    // 处理整数格式
    if let Ok(n) = s.parse::<u32>() {
        return Some(uR64 { nominator: n, denominator: 1 });
    }
    
    // 处理分数格式（如 "1/125"）
    if s.contains('/') { // 检查字符串中是否包含斜杠
        let parts: Vec<&str> = s.split('/').collect(); // 按斜杠分割字符串，将分割结果收集到 Vec 中
        if parts.len() >= 2 {
            if let (Ok(a), Ok(b)) = (parts[0].trim().parse::<u32>(), parts[1].trim().parse::<u32>()) {
                return Some(uR64 { nominator: a, denominator: b });
            }
        }
    }
    
    // 处理小数格式
    if let Some(n) = extract_first_number(s) {
        // 使用 1000 作为分母构造近似有理数
        let denom: u32 = 1000;
        let nom_u64 = (n * denom as f64).round() as u64;
        let nom: u32 = nom_u64.try_into().unwrap_or(u32::MAX);
        return Some(uR64 { nominator: nom, denominator: denom });
    }
    
    None
}

/// 显示从 XMP 映射的 EXIF 标签列表 For PNG
/// 
/// # 参数
/// - `tags`: EXIF 标签列表
/// - `endian`: 字节序信息
fn display_exif_tags(tags: &[ExifTag], endian: &Endian) {
    // println!("\n=== IHDR 信息 ===")
    println!("\n=== EXIF 信息（来自 XMP 映射） ===");
    for tag in tags {
        // println!("{:#?}",&tag);
        display_tag_info(tag, endian);
    }
}

/// 转义 XML 属性值中的特殊字符
/// 
/// # 参数
/// - `s`: 要转义的字符串
/// 
/// # 返回值
/// - 转义后的字符串
fn xml_escape_attr(s: &str) -> String {
    s.replace("&", "&amp;")
     .replace("\"", "&quot;")
     .replace("<", "&lt;")
     .replace(">", "&gt;")
}


/// 处理XMP文件的主函数
/// ### 参数
///- xmp_path XMP文件的路径
/// ### 返回Result类型
/// - 成功时为Ok(())，失败时为Box<dyn std::error::Error>
fn handle_xmp_file(xmp_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // 检查文件是否存在
    if xmp_path.exists() {
        // 尝试解析XMP文件为EXIF标签
        if let Some(tags) = parse_xmp_to_exif_tags(&xmp_path) {
            println!("tags: {:?}", tags);
            display_exif_tags(&tags, &Endian::Little); // 如果解析成功，使用小端序(Little Endian)显示EXIF标签
        } else {
            print_xmp_and_display_all(&xmp_path)?; // 如果解析失败，打印XMP内容并显示所有信息
        }
        return Ok(());
    } else {
        eprintln!("{}", "未在 PNG 中找到 eXIf，也未生成 .xmp.xml");
        return Ok(());
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("使用方法: {} <图片路径>", args[0]);
        return Ok(());
    }
    
    // 获取图片路径
    let path_str = &args[1]; // 图片路径
    let path = Path::new(path_str);
    
    // 检查文件是否存在且为普通文件
    if !path.exists() || !path.is_file() {
        eprintln!("文件不存在或不是文件: {}", path.display());
        return Ok(());
    }
    
    // 获取文件扩展名并转换为小写
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    // 支持的图片格式列表
    let supported_formats = ["jpg", "jpeg", "png", "jxl", "webp"];
    if !supported_formats.contains(&extension.as_str()) {
        eprintln!("不支持的图片格式: {}", extension);
        return Ok(());
    }
    
    // 将扩展名转换为库支持的文件类型枚举
    let file_type = FileExtension::from_str(&extension)
        .map_err(|e| format!("无法解析文件类型: {}", e))?;

    // 尝试使用 little_exif 库直接读取 EXIF 元数据,   JPG
    if let Ok(metadata) = Metadata::new_from_path(path) {

        let endian = metadata.get_endian();
        display_exif_metadata(&metadata, &endian);
        
        // 生成 XMP 文件并显示 EXIF 信息
        let _ = write_xmp_from_metadata(path, &metadata, &endian);
        return Ok(());
    }

    // 如果库读取失败且文件是 PNG 格式，尝试从 PNG 数据块中提取 EXIF
    if extension == "png" {

        // 构建同名 XMP 文件路径
        let xmp_path = path.with_extension("xmp.xml");

        if let Some(exif_bytes) = extract_exif_from_png(path) {
            if let Ok(metadata2) = Metadata::new_from_vec(&exif_bytes, file_type) {

                let endian = metadata2.get_endian(); // 获取字节序（大端或小端）
                let _ = write_xmp_from_metadata(path, &metadata2, &endian); // 将 EXIF 数据写入 XMP 文件
                return Ok(());
            }
        }

        // EXIF 提取失败，回退到 XMP 文件
        handle_xmp_file(&xmp_path)?;
    }

    Ok(())
}