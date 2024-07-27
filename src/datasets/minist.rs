use super::utils::{check_exists, download_with_pb, extract_gz};
use anyhow::{anyhow, Result};
use image::{GrayImage, Luma};
use reqwest::Client;
use std::fs;
use std::io;
use std::path::Path;
use tokio::runtime::Runtime;

const MIRRORS: [&str; 2] = [
   "http://yann.lecun.com/exdb/mnist/",
   "https://ossci-datasets.s3.amazonaws.com/mnist/",
];

const GZ_FILENAMES: [&str; 4] = [
   "train-images-idx3-ubyte.gz",
   "train-labels-idx1-ubyte.gz",
   "t10k-images-idx3-ubyte.gz",
   "t10k-labels-idx1-ubyte.gz",
];
const RAW_FILENAMES: [&str; 4] = [
   "train-images.idx3-ubyte",
   "train-labels.idx1-ubyte",
   "t10k-images.idx3-ubyte",
   "t10k-labels.idx1-ubyte",
];

const LABEL_MAGIC_NUMBER: u32 = 2049;
const IMAGE_MAGIC_NUMBER: u32 = 2051;

pub struct Image {
   pub rows: u32,
   pub cols: u32,
   pub data: Vec<u8>,
}

impl Image {
   pub fn to_image(&self) -> GrayImage {
      let mut img = GrayImage::new(self.rows, self.cols);
      for (i, pixel) in self.data.iter().enumerate() {
         img.put_pixel(
            (i % self.cols as usize) as u32,
            (i / self.cols as usize) as u32,
            Luma([*pixel]),
         );
      }
      img
   }
}

pub struct MINIST {
   pub train_images: Vec<Image>,
   pub train_labels: Vec<u8>,
   pub test_images: Vec<Image>,
   pub test_labels: Vec<u8>,
}

// TODO: Improve error

impl MINIST {
   pub fn new<P: AsRef<Path>>(root: &P, download: bool) -> Result<MINIST> {
      let root = root.as_ref();
      if download {
         MINIST::download(root)?;
      }

      MINIST::load_data(root).map_err(|e| match e.downcast_ref::<io::Error>() {
         Some(_) => anyhow!(
            "MINIST dataset files were not found in \"{}\".",
            root.to_str().unwrap_or("")
         ),
         None => e,
      })
   }

   fn load_data(root: &Path) -> Result<MINIST> {
      let train_images_file = fs::read(root.join(RAW_FILENAMES[0]))?;
      let train_labels_file = fs::read(root.join(RAW_FILENAMES[1]))?;
      let test_images_file = fs::read(root.join(RAW_FILENAMES[2]))?;
      let test_labels_file = fs::read(root.join(RAW_FILENAMES[3]))?;

      Ok(MINIST {
         train_images: MINIST::parse_images(&train_images_file)?,
         train_labels: MINIST::parse_labels(&train_labels_file)?,
         test_images: MINIST::parse_images(&test_images_file)?,
         test_labels: MINIST::parse_labels(&test_labels_file)?,
      })
   }

   fn parse_labels<D: AsRef<[u8]>>(data: &D) -> Result<Vec<u8>> {
      let data = data.as_ref();
      let magic_number = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
      if magic_number != LABEL_MAGIC_NUMBER {
         return Err(anyhow!("Invalid label data. Magic number is not correct."));
      }

      let num_items = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
      if data[8..].len() != num_items as usize {
         return Err(anyhow!(
            "Invalid label data. Number of items is not correct."
         ));
      }
      Ok(data[8..].to_vec())
   }

   fn parse_images<D: AsRef<[u8]>>(data: &D) -> Result<Vec<Image>> {
      let data = data.as_ref();
      let magic_number = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
      if magic_number != IMAGE_MAGIC_NUMBER {
         return Err(anyhow!("Invalid label data. Magic number is not correct."));
      }

      let num_items = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
      let num_rows = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
      let num_cols = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
      let pixels_per_image = num_rows as usize * num_cols as usize;

      if data[16..].len() != num_items as usize * pixels_per_image {
         return Err(anyhow!(
            "Invalid image data. Number of items is not correct."
         ));
      }

      let images: Vec<Image> = (16..num_items as usize * pixels_per_image)
         .step_by(pixels_per_image)
         .map(|i| Image {
            rows: num_rows,
            cols: num_cols,
            data: data[i..i + pixels_per_image].to_vec(),
         })
         .collect();
      Ok(images)
   }

   fn download(root: &Path) -> Result<()> {
      let client = Client::new();
      for (gz_filename, raw_filename) in GZ_FILENAMES.iter().zip(RAW_FILENAMES.iter()) {
         let raw_path = root.join(raw_filename);
         if check_exists(&raw_path) {
            continue;
         }

         let mut downloaded = false;
         for mirror in MIRRORS.iter() {
            let url = format!("{}{}", mirror, gz_filename);
            let rt = Runtime::new().unwrap();
            let result = rt.block_on(download_with_pb(&client, &url, root, gz_filename));
            match result {
               Ok(_) => {
                  downloaded = true;
                  break;
               }
               Err(e) => {
                  println!("Failed to download (trying another mirror):\n{}\n", e);
                  continue;
               }
            }
         }
         if !downloaded {
            Err(anyhow!(
               "Failed to download {} from all mirrors.",
               gz_filename
            ))?;
         }
         extract_gz(&root.join(gz_filename), &raw_path)?;
      }
      Ok(())
   }
}
