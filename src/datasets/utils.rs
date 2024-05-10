use anyhow::Result;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::cmp::min;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub async fn download_with_pb(
   client: &Client,
   url: &str,
   root: &Path,
   filename: &str,
) -> Result<()> {
   println!("Downloading {} ...", url);

   let res = client.get(url).send().await?.error_for_status()?;
   let total_size = res.content_length().unwrap_or(0);

   let pb = ProgressBar::new(total_size);
   pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.green}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})\n{msg}")?
        .progress_chars("#>-"));

   std::fs::create_dir_all(root)?;
   let fullpath = root.join(filename);
   let mut file = File::create(&fullpath)?;
   let mut downloaded: u64 = 0;
   let mut stream = res.bytes_stream();

   while let Some(item) = stream.next().await {
      let chunk = item?;
      file.write_all(&chunk)?;
      let new = min(downloaded + (chunk.len() as u64), total_size);
      downloaded = new;
      pb.set_position(new);
   }

   pb.finish_with_message(format!(
      "Downloaded {} to {}\n",
      url,
      fullpath.to_str().unwrap_or(filename)
   ));

   Ok(())
}

pub fn check_exists<P: AsRef<Path>>(path: &P) -> bool {
   path.as_ref().exists()
}

pub fn extract_gz<P: AsRef<Path>>(gz_path: &P, raw_path: &P) -> Result<()> {
   let gz_file = File::open(gz_path)?;
   let mut decoder = GzDecoder::new(gz_file);
   let mut buffer = Vec::new();
   decoder.read_to_end(&mut buffer)?;

   let mut raw_file = File::create(raw_path)?;
   raw_file.write_all(&buffer)?;

   Ok(())
}
