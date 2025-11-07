use little_exif::{
    metadata::Metadata,
    exif_tag::ExifTag,
    filetype::FileExtension,
    endian::Endian,
};
use little_exif::rational::uR64;
use flate2::read::ZlibDecoder;
use std::str::FromStr;
use std::fs::File;
use std::io::{Read, Cursor, Write};
use std::path::Path;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::convert::TryInto;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("使用方法: {} <图片路径>", args[0]);
        return Ok(());
    }

    let path_str = &args[1];
    let path = Path::new(path_str);
    if !path.exists() || !path.is_file() {
        eprintln!("文件不存在或不是文件: {}", path.display());
        return Ok(());
    }

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    let supported_formats = ["jpg", "jpeg", "png", "jxl", "webp"];
    if !supported_formats.contains(&extension.as_str()) {
        eprintln!("不支持的图片格式: {}", extension);
        return Ok(());
    }

    let file_type = FileExtension::from_str(&extension)
        .map_err(|e| format!("无法解析文件类型: {}", e))?;

    // 尝试用库直接读取 EXIF/Metadata
    if let Ok(metadata) = Metadata::new_from_path(path) {
        let endian = metadata.get_endian();
        // 每次运行都生成同目录的 .xmp.xml（包含从 Metadata 提取的全部 EXIF 信息）
        let _ = write_xmp_from_metadata(path, &metadata, &endian);
        display_exif_metadata(&metadata, &endian);
        return Ok(());
    }

    // 若库读取失败且为 PNG，尝试从 PNG chunk 提取 eXIf；同时 extract_exif_from_png 会保存 iTXt(XMP)
    if extension == "png" {
        if let Some(exif_bytes) = extract_exif_from_png(path) {
            if let Ok(metadata2) = Metadata::new_from_vec(&exif_bytes, file_type) {
                let endian = metadata2.get_endian();
                let _ = write_xmp_from_metadata(path, &metadata2, &endian);
                display_exif_metadata(&metadata2, &endian);
                return Ok(());
            } else {
                // 无法从 eXIf bytes 解析 EXIF，尝试读取同目录 .xmp.xml 并显示全部内容
                let xmp_path = path.with_extension("xmp.xml");
                if xmp_path.exists() {
                    if let Some(tags) = parse_xmp_to_exif_tags(&xmp_path) {
                        display_exif_tags(&tags, &Endian::Little);
                        return Ok(());
                    } else {
                        print_xmp_and_display_all(&xmp_path)?;
                        return Ok(());
                    }
                } else {
                    eprintln!("未能解析 EXIF，也未生成 .xmp.xml");
                    return Ok(());
                }
            }
        } else {
            // 未找到 eXIf，尝试读取同目录 .xmp.xml（extract_exif_from_png 在遇到 iTXt 时会写出）
            let xmp_path = path.with_extension("xmp.xml");
            if xmp_path.exists() {
                if let Some(tags) = parse_xmp_to_exif_tags(&xmp_path) {
                    display_exif_tags(&tags, &Endian::Little);
                    return Ok(());
                } else {
                    print_xmp_and_display_all(&xmp_path)?;
                    return Ok(());
                }
            } else {
                eprintln!("未在 PNG 中找到 eXIf，也未生成 .xmp.xml");
                return Ok(());
            }
        }
    }

    eprintln!("EXIF 读取失败，且不是 PNG：退出。");
    Ok(())
}

/// 提取 PNG 中的 eXIf chunk 或保存 iTXt(XMP) 到 .xmp.xml
fn extract_exif_from_png(path: &Path) -> Option<Vec<u8>> {
    const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    let mut f = File::open(path).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    if buf.len() < 8 || &buf[..8] != PNG_SIG {
        return None;
    }

    let mut pos = 8usize;
    while pos + 8 <= buf.len() {
        let len = {
            let slice = &buf[pos..pos+4];
            u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]) as usize
        };
        let typ = &buf[pos+4..pos+8];
        pos += 8;
        if pos + len + 4 > buf.len() {
            break;
        }
        let data = &buf[pos..pos+len];
        pos += len;
        let _crc = &buf[pos..pos+4];
        pos += 4;

        // 标准 eXIf chunk
        if typ == b"eXIf" {
            return Some(data.to_vec());
        }

        // iTXt: keyword\0 compression_flag(1) compression_method(1) language_tag\0 translated_keyword\0 text
        if typ == b"iTXt" {
            if let Some(key_end) = data.iter().position(|&b| b == 0) {
                if data.len() > key_end + 2 {
                    let compression_flag = data[key_end + 1];
                    let rest = &data[key_end + 3..];
                    if let Some(lang_end_rel) = rest.iter().position(|&b| b == 0) {
                        let translated_start = key_end + 3 + lang_end_rel + 1;
                        if translated_start <= data.len() {
                            if let Some(trans_end_rel) = data[translated_start..].iter().position(|&b| b == 0) {
                                let text_start = translated_start + trans_end_rel + 1;
                                if text_start <= data.len() {
                                    let key = &data[..key_end];
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
                                        if !xmp_bytes.is_empty() {
                                            if let Some(parent) = path.parent() {
                                                let out_path = parent.join(format!("{}.xmp.xml", path.file_stem().and_then(|s| s.to_str()).unwrap_or("extracted")));
                                                let _ = std::fs::write(&out_path, &xmp_bytes);
                                            }
                                        }
                                    }
                                    // raw profile type exif / exif（压缩或未压缩）
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

fn decompress_zlib(compressed: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = ZlibDecoder::new(Cursor::new(compressed));
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn display_tag_info(tag: &ExifTag, endian: &Endian) {
    let tag_name = get_human_readable_tag_name(tag);
    let tag_value = get_tag_value_string(tag, endian);
    println!("{}: {}", tag_name, tag_value);
}

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

fn get_human_readable_tag_name(tag: &ExifTag) -> String {
    match tag {
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
        ExifTag::ResolutionUnit(_) => "分辨率单位".to_string(),
        ExifTag::Orientation(_) => "图片方向".to_string(),
        ExifTag::CreateDate(_) => "文件创建时间".to_string(),
        ExifTag::DateTimeOriginal(_) => "拍摄时间".to_string(),
        ExifTag::ModifyDate(_) => "修改时间".to_string(),
        ExifTag::GPSLatitude(_) => "GPS 纬度".to_string(),
        ExifTag::GPSLatitudeRef(_) => "纬度方向".to_string(),
        ExifTag::GPSLongitude(_) => "GPS 经度".to_string(),
        ExifTag::GPSLongitudeRef(_) => "经度方向".to_string(),
        ExifTag::GPSAltitude(_) => "GPS 海拔".to_string(),
        ExifTag::GPSAltitudeRef(_) => "海拔方向".to_string(),
        ExifTag::GPSTimeStamp(_) => "GPS 时间戳".to_string(),
        ExifTag::ImageDescription(_) => "图片描述".to_string(),
        ExifTag::UserComment(_) => "用户注释".to_string(),
        ExifTag::UnknownINT8U(_, id, _) => format!("未知标签 (0x{:04X})", id),
        ExifTag::UnknownINT8S(_, id, _) => format!("未知标签 (0x{:04X})", id),
        ExifTag::UnknownINT16U(_, id, _) => format!("未知标签 (0x{:04X})", id),
        ExifTag::UnknownINT16S(_, id, _) => format!("未知标签 (0x{:04X})", id),
        ExifTag::UnknownINT32U(_, id, _) => format!("未知标签 (0x{:04X})", id),
        ExifTag::UnknownINT32S(_, id, _) => format!("未知标签 (0x{:04X})", id),
        ExifTag::UnknownSTRING(_, id, _) => format!("未知标签 (0x{:04X})", id),
        ExifTag::UnknownRATIONAL64U(_, id, _) => format!("未知标签 (0x{:04X})", id),
        ExifTag::UnknownRATIONAL64S(_, id, _) => format!("未知标签 (0x{:04X})", id),
        _ => "未定义标签".to_string(),
    }
}

fn get_tag_value_string(tag: &ExifTag, _endian: &Endian) -> String {
    if let ExifTag::Make(s) | ExifTag::Model(s) | ExifTag::Software(s) |
        ExifTag::Artist(s) | ExifTag::Copyright(s) | ExifTag::ImageDescription(s) |
        ExifTag::CreateDate(s) | ExifTag::DateTimeOriginal(s) | ExifTag::ModifyDate(s) |
        ExifTag::GPSLatitudeRef(s) | ExifTag::GPSLongitudeRef(s) = tag {
        let trimmed = s.trim_matches('\0').trim();
        return if trimmed.is_empty() { "空值".to_string() } else { trimmed.to_string() };
    }

    if let ExifTag::UserComment(v) = tag {
        if let Ok(s) = std::str::from_utf8(v) {
            let trimmed = s.trim_matches('\0').trim();
            return if trimmed.is_empty() { "空值".to_string() } else { trimmed.to_string() };
        } else {
            return format!("字节数据: {:x?}", v);
        }
    }

    if let ExifTag::FNumber(vec) | ExifTag::FocalLength(vec) | ExifTag::ExposureTime(vec) |
        ExifTag::XResolution(vec) | ExifTag::YResolution(vec) | ExifTag::GPSLatitude(vec) |
        ExifTag::GPSLongitude(vec) | ExifTag::GPSAltitude(vec) = tag {
        if let Some(rational) = vec.first() {
            let value = (rational.nominator as f64) / (rational.denominator as f64);
            return match tag {
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

    if let ExifTag::ISO(vec) = tag {
        if let Some(&value) = vec.first() {
            return format!("{}", value);
        }
    }

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
    if let ExifTag::Orientation(vec) = tag {
        if let Some(&value) = vec.first() {
            return orientation_to_str(value.into());
        }
    }
    if let ExifTag::ResolutionUnit(vec) = tag {
        if let Some(&value) = vec.first() {
            return resolution_unit_to_str(value.into());
        }
    }
    if let ExifTag::GPSTimeStamp(vec) = tag {
        if vec.len() >= 3 {
            let h = (vec[0].nominator as f64) / (vec[0].denominator as f64);
            let m = (vec[1].nominator as f64) / (vec[1].denominator as f64);
            let s = (vec[2].nominator as f64) / (vec[2].denominator as f64);
            return format!("{:02.0}:{:02.0}:{:.1}", h, m, s);
        }
    }

    if let ExifTag::UnknownINT8S(_, id, bytes) = tag {
        return format!("未知标签 (0x{:04X}): {:?}", id, bytes);
    }

    "无法解析".to_string()
}

fn display_summary_info(metadata: &Metadata, endian: &Endian) {
    println!("\n--- 关键信息 ---");
    println!("制造商: {}", get_tag_value(metadata, &ExifTag::Make(String::new()), endian));
    println!("相机型号: {}", get_tag_value(metadata, &ExifTag::Model(String::new()), endian));
    println!("软件: {}", get_tag_value(metadata, &ExifTag::Software(String::new()), endian));
    println!("ISO: {}", get_tag_value(metadata, &ExifTag::ISO(vec![]), endian));
    let shoot_time = get_tag_value(metadata, &ExifTag::DateTimeOriginal(String::new()), endian);
    if shoot_time != "未找到" {
        println!("拍摄时间: {}", shoot_time);
    } else {
        println!("创建时间: {}", get_tag_value(metadata, &ExifTag::CreateDate(String::new()), endian));
    }
}

fn get_tag_value(metadata: &Metadata, exif_tag: &ExifTag, endian: &Endian) -> String {
    metadata.get_tag(exif_tag)
        .next()
        .map(|tag| get_tag_value_string(tag, endian))
        .unwrap_or_else(|| "未找到".to_string())
}

fn orientation_to_str(value: u32) -> String {
    match value {
        1 => "正常（0°旋转）",
        2 => "水平翻转",
        3 => "180°旋转",
        4 => "垂直翻转",
        5 => "水平翻转+270°顺时针旋转",
        6 => "270°顺时针旋转",
        7 => "水平翻转+90°顺时针旋转",
        8 => "90°顺时针旋转",
        _ => "未知方向",
    }.to_string()
}

fn resolution_unit_to_str(value: u32) -> String {
    match value {
        1 => "无单位",
        2 => "英寸（DPI）",
        3 => "厘米（DPC）",
        _ => "未知单位",
    }.to_string()
}

/// 将 Metadata 中的所有标签写成简单 XML 到同目录的 .xmp.xml（每次运行都会覆盖）
fn write_xmp_from_metadata(path: &Path, metadata: &Metadata, endian: &Endian) -> std::io::Result<()> {
    let mut items: Vec<(String,String)> = Vec::new();
    for tag in metadata {
        let name = get_human_readable_tag_name(&tag);
        let value = get_tag_value_string(&tag, endian);
        items.push((name, value));
    }

    let xml_path = path.with_extension("xmp.xml");
    let mut f = File::create(&xml_path)?;
    writeln!(f, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(f, r#"<ExifFromMetadata source="{}">"#, path.file_name().and_then(|s| s.to_str()).unwrap_or(""))?;
    for (k, v) in items {
        writeln!(f, r#"  <tag name="{}"><![CDATA[{}]]></tag>"#, xml_escape_attr(&k), v)?;
    }
    writeln!(f, "</ExifFromMetadata>")?;
    Ok(())
}

/// 直接打印 .xmp.xml 的内容（用于回退显示全部 EXIF 信息）
fn print_xmp_and_display_all(xmp_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(xmp_path)?;
    println!("\n=== 来自 {} 的 XMP / EXIF 信息 ===\n", xmp_path.display());
    // 直接打印完整 XML，便于查看全部字段
    println!("{}", s);
    Ok(())
}

/// 解析 xmp.xml 并尽量映射为 ExifTag 列表
fn parse_xmp_to_exif_tags(xmp_path: &Path) -> Option<Vec<ExifTag>> {
    let s = std::fs::read_to_string(xmp_path).ok()?;
    let mut reader = Reader::from_str(&s);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut out: Vec<ExifTag> = Vec::new();

    while let Ok(ev) = reader.read_event_into(&mut buf) {
        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => {
                // 先把 rdf:Description 的属性映射
                if e.local_name().as_ref().ends_with(b"Description") {
                    for attr_res in e.attributes() {
                        if let Ok(attr) = attr_res {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let val = attr.unescape_value().unwrap_or_default().to_string();
                            map_xmp_key_value_into_tag(&key, &val, &mut out);
                        }
                    }
                }
            }
            Event::Text(e) => {
                // 有些 XMP 字段以 element 文本形式存在，处理常见 element 名
                if let Ok(text) = e.unescape() {
                    let parent = reader.buffer_position();
                    // 尝试把最近的开始标签名作为 key（简单策略）
                    // (注意：quick-xml 在这里不方便直接取得父名，故只使用属性映射作为主要手段)
                    let _ = parent;
                    let _ = text;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    if out.is_empty() { None } else { Some(out) }
}

/// 把 XMP key/value 映射为 ExifTag（尽量覆盖常见字段）
fn map_xmp_key_value_into_tag(key: &str, val: &str, out: &mut Vec<ExifTag>) {
    let k = key.split(':').last().unwrap_or(key).to_lowercase();
    let v = val.trim();
    if v.is_empty() { return; }

    match k.as_str() {
        "make" => out.push(ExifTag::Make(v.to_string())),
        "model" => out.push(ExifTag::Model(v.to_string())),
        "creatortool" | "software" => out.push(ExifTag::Software(v.to_string())),
        "createdate" | "creatordate" => out.push(ExifTag::CreateDate(v.to_string())),
        "datetimeoriginal" => out.push(ExifTag::DateTimeOriginal(v.to_string())),
        "modifydate" => out.push(ExifTag::ModifyDate(v.to_string())),
        "copyright" => out.push(ExifTag::Copyright(v.to_string())),
        "isospeedratings" | "iso" => {
            if let Ok(n) = v.parse::<u16>() {
                out.push(ExifTag::ISO(vec![n]));
            } else if let Some(n) = extract_first_number(v) {
                out.push(ExifTag::ISO(vec![n as u16]));
            }
        }
        "fnumber" => {
            if let Some(r) = parse_fraction_to_rational(v) {
                out.push(ExifTag::FNumber(vec![r]));
            }
        }
        "focallength" => {
            if let Some(r) = parse_fraction_to_rational(v) {
                out.push(ExifTag::FocalLength(vec![r]));
            }
        }
        "exposuretime" => {
            if let Some(r) = parse_fraction_to_rational(v) {
                out.push(ExifTag::ExposureTime(vec![r]));
            }
        }
        _ => {
            // 其它字段可以作为字符串类型尝试加入常见文本型 Tag
            // 例如把 tiff:Make/tiff:Model 等已处理，其他忽略
        }
    }
}

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

// 修改：返回 uR64（exif_tag_format 中的有理数元素类型）
fn parse_fraction_to_rational(s: &str) -> Option<uR64> {
    let s = s.trim();
    if s.contains('/') {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() >= 2 {
            if let (Ok(a), Ok(b)) = (parts[0].trim().parse::<u32>(), parts[1].trim().parse::<u32>()) {
                return Some(uR64 { nominator: a, denominator: b });
            }
        }
    }
    if let Some(n) = extract_first_number(s) {
        // 用 1000 作为分母构造近似有理数（确保类型为 u32）
        let denom: u32 = 1000;
        let nom_u64 = (n * denom as f64).round() as u64;
        let nom: u32 = nom_u64.try_into().unwrap_or(u32::MAX);
        return Some(uR64 { nominator: nom, denominator: denom });
    }
    None
}

/// 显示由 XMP 映射的 ExifTag 列表（作为回退）
fn display_exif_tags(tags: &[ExifTag], endian: &Endian) {
    println!("\n=== EXIF 信息（来自 XMP 映射） ===");
    for tag in tags {
        display_tag_info(tag, endian);
    }
}

fn xml_escape_attr(s: &str) -> String {
    s.replace("&", "&amp;").replace("\"", "&quot;").replace("<", "&lt;").replace(">", "&gt;")
}