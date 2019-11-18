#![allow(unused_imports)]
extern crate image;


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
fn median_filt(img: &[u8], img_width: u32, img_height: u32, pix_size: u32, filt_size: u32) -> Vec<u8> {

    let img_width = img_width as i32;
    let img_height = img_height as i32;
    let pix_size = pix_size as i32;
    let filt_size = filt_size as i32;

    let filt_edge = filt_size/2;

    let mut out : Vec<u8> = vec![0; (img_width * img_height * pix_size) as usize ];
    let mut pix_arr : Vec<u8> = vec![0; (filt_size*filt_size) as usize ];

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
                pix_arr.sort();
                out[(x*pix_size+color_off + y * row_width) as usize] = pix_arr[ (filt_size/2) as usize ];
            }
            
        }
    }
    return out;
}

// NOTE: only odd sized filters will work
fn spatial_filt(img: &[u8], img_width: u32, img_height: u32, pix_size: u32,
                filt: &[f32], filt_size: u32) -> Vec<u8> {

    let img_width = img_width as i32;
    let img_height = img_height as i32;
    let pix_size = pix_size as i32;
    let filt_size = filt_size as i32;

    let filt_edge = filt_size/2;

    let mut out : Vec<u8> = vec![0; (img_width * img_height * pix_size) as usize ];
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
                            pix_arr[i] = img[( (x+xx)*pix_size+color_off + (y+yy)*row_width ) as usize] as f32 *
                                filt[((xx+filt_edge)+(yy+filt_edge)*filt_size) as usize];
                            i += 1;
                    }
                }
                out[(x*pix_size+color_off + y * row_width) as usize] = clamp(pix_arr.iter().sum(), 0.0, 255.0) as u8;
            }
            
        }
    }
    return out;

}

fn edge_gradient(hor_edges: &[u8], vert_edges: &[u8], img_width: u32, img_height: u32) -> Vec<u8> {
    let mut out : Vec<u8> = vec![0; (img_width * img_height) as usize ];


    for x in 0 .. img_width {
        for y in 0 .. img_height {
            let cur_ind = (x + y * img_width) as usize;
            out[cur_ind] = clamp(
                // magnitude
                ((hor_edges[cur_ind] as f32).powf(2.0) + (vert_edges[cur_ind] as f32).powf(2.0)).sqrt(),
                0.0, 255.0) as u8;
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

fn threshold(img: &[u8], img_width: u32, img_height: u32, boundary: u8) -> Vec<u8>{
    let mut out : Vec<u8> = vec![0; (img_width * img_height) as usize ];

    for x in 0 .. img_width {
        for y in 0 .. img_height {
            let cur_ind = (x + y * img_width) as usize;
            out [cur_ind] = if img[cur_ind] > boundary {
                255
            } else {
                0
            }
        }
    }

    return out;

}

fn cartoonify(img: &[u8], edge_grad: &[u8], img_width: u32, img_height: u32, pix_size: u32) -> Vec<u8>{
    let mut out : Vec<u8> = vec![0; (img_width * img_height * pix_size) as usize ];

    let row_width = pix_size * img_width;

    for x in 0 .. img_width {
        for y in 0 .. img_height {
            for color_off in 0 .. pix_size {

                let color_ind = (x * pix_size + y * row_width + color_off) as usize;
                let gray_ind = (x + y * img_width) as usize;

                out [color_ind] = if edge_grad[gray_ind] > 0 {
                    0
                } else {
                    img[color_ind]
                }
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
    let gray_img_buf = img.to_luma().into_raw();
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


    let low_pass = spatial_filt(&gray_img_buf, img_width, img_height, 1,
                                                                      &[ 1.0/9.0,1.0/9.0,1.0/9.0,
                                                                         1.0/9.0,1.0/9.0,1.0/9.0,
                                                                         1.0/9.0,1.0/9.0,1.0/9.0 ],3);

    // Sobel
    let vert_edges = spatial_filt(&low_pass, img_width, img_height, 1,
                                                                      &[ -1.0,0.0,1.0,
                                                                         -2.0,0.0,2.0,
                                                                         -1.0,0.0,1.0 ],3);
    let hor_edges = spatial_filt(&low_pass, img_width, img_height, 1,
                                                                      &[ 1.0,2.0,1.0,
                                                                         0.0,0.0,0.0,
                                                                       -1.0,-2.0,-1.0 ],3);

    let edge_grad = edge_gradient(&hor_edges, &vert_edges, img_width, img_height);
    let thresh = threshold(&edge_grad, img_width, img_height, 65);
    let cartoon = cartoonify(&med_result, &thresh, img_width, img_height, pixel_size);

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


    save_buffer("median.png", &med_result, img_width, img_height, image::RGB(8)).unwrap();
    save_buffer("vert_edges.png", &vert_edges, img_width, img_height, image::Gray(8)).unwrap();
    save_buffer("hor_edges.png", &hor_edges, img_width, img_height, image::Gray(8)).unwrap();
    save_buffer("gradient.png", &edge_grad, img_width, img_height, image::Gray(8)).unwrap();
    save_buffer("thresh.png", &thresh, img_width, img_height, image::Gray(8)).unwrap();
    save_buffer("cartoon.png", &cartoon, img_width, img_height, image::RGB(8)).unwrap();
}
