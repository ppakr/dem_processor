use crate::cli::Args;
use anyhow::Result;
use esri_ascii_grid::ascii_file::EsriASCIIReader;
use image::{GrayImage, Luma, RgbImage, Rgb};
use std::fs::File;
use std::path::Path;
use std::f32::consts::PI;

/// Processes an ASCII grid file and generates output based on the specified mode (grayscale or hillshade).
pub fn process_asc_file(path: &Path, args: &Args) -> Result<()> {
    // Convert the ASCII grid file to a grayscale image.
    let image = ascii_to_image(path)?;

    // Handle different processing modes based on user input.
    match args.mode.as_str() {
        "grayscale" => {
            // Save the grayscale image to the output directory.
            let output_path = args.output_dir.join(format!("{}.png", path.file_stem().unwrap().to_string_lossy()));
            image.save(&output_path)?;
            println!("🍤 Saved grayscale image to {:?}", output_path);
        }
        "hillshade" => {
            // Generate a colormap from the grayscale image.
            let color_img = grayscale_to_colormap(&image);
            // Apply hillshading to the grayscale image.
            let hillshade = apply_hillshade(&image, 30.0, 315.0, 45.0);
            // Blend the colormap with the hillshade.
            let blended = blend_colormap_with_hillshade(&color_img, &hillshade);
            // Save the hillshaded image to the output directory.
            let output_path = args.output_dir.join(format!("{}_hillshade.png", path.file_stem().unwrap().to_string_lossy()));
            blended.save(&output_path)?;
            println!("🧋 Saved hillshaded image to {:?}", output_path);
        }
        _ => println!("💔 Unsupported mode: {}", args.mode),
    }

    Ok(())
}

/// Converts an ASCII grid file to a grayscale image.
fn ascii_to_image(path: &Path) -> Result<GrayImage> {
    let file = File::open(path)?;
    let mut reader: EsriASCIIReader<File, f64, f64> = EsriASCIIReader::from_file(file)?;

    let header = reader.header;
    let nodata = header.no_data_value().unwrap_or(f64::NAN);
    let rows = header.num_rows();
    let cols = header.num_cols();

    let mut values = vec![vec![0.0; cols]; rows];
    let mut min_val = f64::MAX;
    let mut max_val = f64::MIN;

    // Read the grid values and track the min and max values for normalization.
    for cell in reader.into_iter() {
        if let Ok((row, col, value)) = cell {
            values[row][col] = value;
            if value != nodata {
                min_val = min_val.min(value);
                max_val = max_val.max(value);
            }
        }
    }

    let mut img = GrayImage::new(cols as u32, rows as u32);
    // Normalize the values to the range [0, 255] and create the grayscale image.
    for row in 0..rows {
        for col in 0..cols {
            let value = values[row][col];
            let pixel = if value == nodata {
                0
            } else {
                ((value - min_val) / (max_val - min_val) * 255.0).round() as u8
            };
            img.put_pixel(col as u32, (rows - 1 - row) as u32, Luma([pixel]));
        }
    }

    Ok(img)
}

/// Converts a grayscale image to a colormap image.
fn grayscale_to_colormap(gray: &GrayImage) -> RgbImage {
    let (width, height) = gray.dimensions();
    let mut rgb_img = RgbImage::new(width, height);

    // Map grayscale values to RGB colors.
    for (x, y, pixel) in gray.enumerate_pixels() {
        let v = pixel[0] as f32 / 255.0;
        let r = (v * 255.0) as u8;
        let g = ((1.0 - (v - 0.5).abs()) * 255.0) as u8;
        let b = ((1.0 - v) * 128.0) as u8;
        rgb_img.put_pixel(x, y, Rgb([r, g, b]));
    }

    rgb_img
}

/// Converts degrees to radians.
fn deg2rad(deg: f32) -> f32 {
    deg * (PI / 180.0)
}

/// Applies hillshading to a grayscale image.
fn apply_hillshade(gray: &GrayImage, cell_size: f32, azimuth_deg: f32, altitude_deg: f32) -> RgbImage {
    let (width, height) = gray.dimensions();
    let mut rgb_img = RgbImage::new(width, height);

    // Convert azimuth and altitude angles to radians.
    let az_rad = deg2rad(360.0 - azimuth_deg + 90.0) % (2.0 * PI);
    let alt_rad = deg2rad(altitude_deg);

    // Helper function to get pixel values with boundary clamping.
    let get = |x: i32, y: i32| -> f32 {
        let cx = x.clamp(0, width as i32 - 1) as u32;
        let cy = y.clamp(0, height as i32 - 1) as u32;
        gray.get_pixel(cx, cy)[0] as f32
    };

    // Compute hillshade values for each pixel.
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let z1 = get(x - 1, y - 1);
            let z2 = get(x, y - 1);
            let z3 = get(x + 1, y - 1);
            let z4 = get(x - 1, y);
            let z5 = get(x + 1, y);
            let z6 = get(x - 1, y + 1);
            let z7 = get(x, y + 1);
            let z8 = get(x + 1, y + 1);

            // Calculate slope and aspect.
            let dzdx = ((z3 + 2.0 * z5 + z8) - (z1 + 2.0 * z4 + z6)) / (8.0 * cell_size);
            let dzdy = ((z6 + 2.0 * z7 + z8) - (z1 + 2.0 * z2 + z3)) / (8.0 * cell_size);

            let slope = (dzdx * dzdx + dzdy * dzdy).sqrt().atan();
            let aspect = if dzdx != 0.0 {
                let mut a = (dzdy / -dzdx).atan();
                if dzdx > 0.0 {
                    a += PI;
                } else if dzdy < 0.0 {
                    a += 2.0 * PI;
                }
                a
            } else if dzdy > 0.0 {
                PI / 2.0
            } else {
                3.0 * PI / 2.0
            };

            // Calculate hillshade intensity.
            let hs = (alt_rad.cos() * slope.cos()
                + alt_rad.sin() * slope.sin() * (az_rad - aspect).cos()).max(0.0);

            let shade = (255.0 * hs).round() as u8;
            rgb_img.put_pixel(x as u32, y as u32, Rgb([shade, shade, shade]));
        }
    }

    rgb_img
}

/// Blends a colormap image with a hillshade image.
fn blend_colormap_with_hillshade(color: &RgbImage, shade: &RgbImage) -> RgbImage {
    let (width, height) = color.dimensions();
    let mut blended = RgbImage::new(width, height);

    // Blend each pixel by modulating the colormap with the hillshade intensity.
    for y in 0..height {
        for x in 0..width {
            let c = color.get_pixel(x, y);
            let s = shade.get_pixel(x, y)[0] as f32 / 255.0;
            let r = (c[0] as f32 * s).clamp(0.0, 255.0) as u8;
            let g = (c[1] as f32 * s).clamp(0.0, 255.0) as u8;
            let b = (c[2] as f32 * s).clamp(0.0, 255.0) as u8;
            blended.put_pixel(x, y, Rgb([r, g, b]));
        }
    }

    blended
}
