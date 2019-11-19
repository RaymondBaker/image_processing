#![allow(unused_imports)]
extern crate image;


use std::process::Command;
use image::{GenericImageView, GenericImage, DynamicImage, save_buffer, ColorType};
use std::io::prelude::*;
use std::fs::File;
use std::fmt::Display;

struct ImageBuf {
    buf: Vec<u8>,
    width: u32,
    height: u32,
}

// NOTE: Dont use filter of size 2
// Returns ( image_buf, width, height )
fn median_filt(img: &[f32], img_width: u32, img_height: u32, pix_size: u32, filt_size: u32) -> Vec<f32> {

    let img_width = img_width as i32;
    let img_height = img_height as i32;
    let pix_size = pix_size as i32;
    let filt_size = filt_size as i32;

    let filt_edge = filt_size/2;

    let mut out : Vec<f32> = vec![0.0; (img_width * img_height * pix_size) as usize ];
    let mut pix_arr : Vec<f32> = vec![0.0; (filt_size*filt_size) as usize ];

    let row_width = img_width * pix_size;

    // Ignores edge pixels
    for x in filt_edge .. img_width - filt_edge {
        for y in filt_edge .. img_height - filt_edge {
            // apply filter
            // Get all pixels in the filter
            for color_off in 0 .. pix_size {
                let mut i = 0;
                for xx in -filt_edge ..= filt_edge {
                    for yy in -filt_edge ..= filt_edge {
                            pix_arr[i] = img[( (x+xx)*pix_size+color_off + (y+yy)*row_width ) as usize];
                            i += 1;
                    }
                }
                pix_arr.sort_by(|a, b| a.partial_cmp(b).unwrap());
                out[(x*pix_size+color_off + y * row_width) as usize] = pix_arr[ (filt_size/2) as usize ];
            }
            
        }
    }
    return out;
}

// NOTE: only odd sized filters will work
fn spatial_filt(img: &[f32], img_width: u32, img_height: u32, pix_size: u32,
                filt: &[f32], filt_size: u32) -> Vec<f32> {

    let img_width = img_width as i32;
    let img_height = img_height as i32;
    let pix_size = pix_size as i32;
    let filt_size = filt_size as i32;

    let filt_edge = filt_size/2;

    let mut out : Vec<f32> = vec![0.0; (img_width * img_height * pix_size) as usize ];
    let mut pix_arr : Vec<f32> = vec![0.0; (filt_size*filt_size) as usize ];

    let row_width = img_width * pix_size;

    // Ignores edge pixels
    for x in filt_edge .. img_width - filt_edge {
        for y in filt_edge .. img_height - filt_edge {
            // apply filter
            // Get all pixels in the filter
            for color_off in 0 .. pix_size {
                let mut i = 0;
                for xx in -filt_edge ..= filt_edge {
                    for yy in -filt_edge ..= filt_edge {
                            // NOTE: Naive implementation could use values from the last iteration
                            pix_arr[i] = img[( (x+xx)*pix_size+color_off + (y+yy)*row_width ) as usize] *
                                filt[((xx+filt_edge)+(yy+filt_edge)*filt_size) as usize];
                            i += 1;
                    }
                }
                out[(x*pix_size+color_off + y * row_width) as usize] = pix_arr.iter().sum();
            }
            
        }
    }
    return out;

}

fn f32_slice_to_u8(slice: &[f32]) -> Vec<u8> {
    let mut out : Vec<u8> = vec![ 0; slice.len()];
    for i in 0 .. slice.len() {
        out[i] = clamp(slice[i], 0.0, 255.0) as u8;
    }
    return out;
}

fn u8_slice_to_f32(slice: &[u8]) -> Vec<f32> {
    let mut out : Vec<f32> = vec![ 0.0; slice.len()];
    for i in 0 .. slice.len() {
        out[i] = slice[i] as f32;
    }
    return out;
}

fn edge_gradient(hor_edges: &[f32], vert_edges: &[f32], img_width: u32, img_height: u32) -> Vec<f32> {
    let mut out : Vec<f32> = vec![0.0; (img_width * img_height) as usize ];

    for x in 0 .. img_width {
        for y in 0 .. img_height {
            let cur_ind = (x + y * img_width) as usize;
            out[cur_ind] = clamp(
                // magnitude
                (hor_edges[cur_ind].powf(2.0) + vert_edges[cur_ind].powf(2.0)).sqrt(),
                0.0, 255.0);
        }
    }

    return out;
}


fn clamp<T>(val: T, min: T, max: T) -> T where
    T: PartialOrd {

    if val > max {
        return max;
    }
    else if val < min {
        return min;
    }
    return val;
}


fn print_arr<T>(in_data: &[T], width: u32) where
    T: Display {

    let mut x = 0;
    for data in in_data {
        print!("{},", data);
        x+=1;
        if x >= width {
            println!("");
            x= 0;
        }
    }
}

fn threshold(img: &[f32], boundary: f32) -> Vec<f32>{
    let mut out : Vec<f32> = vec![0.0; img.len() ];
    for i in 0 .. img.len() {
        out [i] = if img[i] > boundary {
            255.0
        } else {
            0.0
        }
    }

    return out;

}

fn cartoonify(img: &[f32], edge_grad: &[f32], img_width: u32, img_height: u32, pix_size: u32) -> Vec<f32>{
    let mut out : Vec<f32> = vec![0.0; (img_width * img_height * pix_size) as usize ];

    let row_width = pix_size * img_width;

    for x in 0 .. img_width {
        for y in 0 .. img_height {
            for color_off in 0 .. pix_size {

                let color_ind = (x * pix_size + y * row_width + color_off) as usize;
                let gray_ind = (x + y * img_width) as usize;

                out [color_ind] = if edge_grad[gray_ind] > 0.0 {
                    0.0
                } else {
                    img[color_ind]
                }
            }
        }
    }

    return out;
}

fn zero_crossings(img: &[f32], img_width: u32, img_height: u32, thresh: f32) -> Vec<f32>{
    let mut out : Vec<f32> = vec![0.0; (img_width * img_height) as usize ];
    // Horizontal Pass
    for x in 1 .. img_width - 1 {
        for y in 0 .. img_height {
            let left  = img[(x-1 + y * img_width) as usize];
            let right = img[(x+1 + y * img_width) as usize];
            out[(x + y * img_width) as usize] = if left.min(right) < 0.0 && left.max(right) > 0.0 
            && left.max(right) - left.min(right) > thresh { 
                255.0
            } else {
                0.0
            }
        }
    }

    // Vertical Pass
    for x in 0 .. img_width {
        for y in 1 .. img_height - 1 {
            let top  = img[(x + (y-1) * img_width) as usize];
            let bottom = img[(x + (y+1) * img_width) as usize];
            out[(x + y * img_width) as usize] = if top.min(bottom) < 0.0 && top.max(bottom) > 0.0
                && top.max(bottom) - top.min(bottom) > thresh { 
                255.0
            } else {
                0.0
            }
        }
    }
    return out;
}

fn main() {
    let mut stderr = std::io::stderr();
    let file_name = "/home/raymond/Programming/Rust/image_processing/me.jpg";

    let img = match image::open(file_name) {
        Ok(img) => img,
        Err(e)  => {
            writeln!(stderr, "Couldn't open file \"{:?}\": {:?}", file_name, e);
            return;
        },
    };

   
    let (img_width, img_height) = img.dimensions();
    let img_buf = img.to_rgb().into_raw();
    let img_buf = u8_slice_to_f32(&img_buf);
    let gray_img_buf = img.to_luma().into_raw();
    let gray_img_buf = u8_slice_to_f32(&gray_img_buf);
    let pixel_size = 3;
    
    
    /* TEST IMAGE
    let img_buf: [u8; 48] = [   100, 50, 70, 100, 50, 70, 100, 50, 70, 100, 50, 70,
                                100, 50, 70, 100, 50, 70, 100, 50, 70, 100, 50, 70,
                                100, 50, 70, 100, 50, 70, 100, 50, 70, 100, 50, 70,
                                100, 50, 70, 100, 50, 70, 100, 50, 70, 100, 50, 70,];
    let img_width = 4;
    let img_height = 4;
    let pixel_size = 3;
    */

    let med_result = median_filt(&img_buf, img_width, img_height, pixel_size, 5);
    let med_result = median_filt(&med_result, img_width, img_height, pixel_size, 3);


    /*
    let low_pass = spatial_filt(&gray_img_buf, img_width, img_height, 1,
                                                                      &[ 1.0/9.0,1.0/9.0,1.0/9.0,
                                                                         1.0/9.0,1.0/9.0,1.0/9.0,
                                                                         1.0/9.0,1.0/9.0,1.0/9.0 ],3);

    // Sobel
    */


    let gaussian = spatial_filt(&gray_img_buf, img_width, img_height, 1,
                                                                      &[ 1.0/16.0,2.0/16.0,1.0/16.0,
                                                                         2.0/16.0,4.0/16.0,2.0/16.0,
                                                                         1.0/16.0,2.0/16.0,1.0/16.0, ],3);
    let laplacian = spatial_filt(&gaussian, img_width, img_height, 1,
                                                                      &[ -1.0,-1.0,-1.0,
                                                                         -1.0, 8.0,-1.0,
                                                                         -1.0,-1.0,-1.0 ],3);

    let vert_edges = spatial_filt(&gaussian, img_width, img_height, 1,
                                                                      &[ -1.0,0.0,1.0,
                                                                         -2.0,0.0,2.0,
                                                                         -1.0,0.0,1.0 ],3);
    let hor_edges = spatial_filt(&gaussian, img_width, img_height, 1,
                                                                      &[ 1.0,2.0,1.0,
                                                                         0.0,0.0,0.0,
                                                                       -1.0,-2.0,-1.0 ],3);

    let edge_grad = edge_gradient(&hor_edges, &vert_edges, img_width, img_height);
    let edge_grad = threshold(&edge_grad,80.0);

    let edges = zero_crossings(&laplacian, img_width, img_height, 0.0);
    //let cartoon = cartoonify(&med_result, &edges, img_width, img_height, pixel_size);
    let cartoon = cartoonify(&med_result, &edge_grad, img_width, img_height, pixel_size);


    let vert_edges = f32_slice_to_u8(&vert_edges);
    let hor_edges = f32_slice_to_u8(&hor_edges);
    let edge_grad = f32_slice_to_u8(&edge_grad);
    let edges = f32_slice_to_u8(&edges);
    let laplacian = f32_slice_to_u8(&laplacian);
    let cartoon = f32_slice_to_u8(&cartoon);

    //let thresh = threshold(&edge_grad, img_width, img_height, 60);
    //let cartoon = cartoonify(&med_result, &thresh, img_width, img_height, pixel_size);

    /* NOTE: Test print out
    let mut file_buf = File::create("in.dat").unwrap();
    for data in &img.to_rgb().into_raw() {
        write!(file_buf, "{},", data);
    }

    let mut file_buf = File::create("out.dat").unwrap();
    for data in &med_result {
        write!(file_buf, "{},", data);
    }
    */


    //save_buffer("median.png", &med_result, img_width, img_height, image::RGB(8)).unwrap();
    save_buffer("vert_edges.png", &vert_edges, img_width, img_height, image::Gray(8)).unwrap();
    save_buffer("hor_edges.png", &hor_edges, img_width, img_height, image::Gray(8)).unwrap();
    save_buffer("gradient.png", &edge_grad, img_width, img_height, image::Gray(8)).unwrap();
    //save_buffer("thresh.png", &thresh, img_width, img_height, image::Gray(8)).unwrap();
    save_buffer("cartoon.png", &cartoon, img_width, img_height, image::RGB(8)).unwrap();
    save_buffer("laplacian.png", &laplacian, img_width, img_height, image::Gray(8)).unwrap();
    save_buffer("edges.png", &edges, img_width, img_height, image::Gray(8)).unwrap();
}
