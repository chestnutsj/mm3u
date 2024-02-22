use std::path::{PathBuf,Path};

use alloc::sync;
use anyhow::{Error, Ok, Result};
use futures_util::future::ok;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
use reqwest::{IntoUrl,Url};
use futures_util::StreamExt;
use tokio::fs::File;
use crate::status::{self, Status};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

struct FileD {
    offset :usize,
    data: Vec<u8>
}

struct DownTask {
    source: Url,
    output_dir: PathBuf,
    name: String,
    status: Status,
    target: File,
}
static LIMITE_PATE_SIZE:u64 = 1024;

impl DownTask {
    pub async fn new<U: IntoUrl, P: AsRef<Path>>(url :U, dir:P,name: &str) -> Result<Self> {
       
       let mut status_name= PathBuf::from(dir.as_ref());
       status_name.push( name);
       status_name.set_extension(".mm3u");
    


       let task = DownTask{
        source: url.into_url()?,
        output_dir: dir.as_ref().to_path_buf(),
        name: name.into(),
        status: Status::new(status_name)?,
       };

       Ok(task)
    }

    pub async fn start(&mut self)->Result<()> {
        
        if !self.status.is_init() {
            let (range,len) =self.check_request_range().await ?;
            if range && len > (LIMITE_PATE_SIZE *LIMITE_PATE_SIZE)  {

            }

        }

        Ok(())
    }



    pub async fn check_request_range(&self) -> Result<(bool, u64)> {
    
        let mut range = false;
        let req = reqwest::Client::new().head(self.source.as_str());
        let rep = req.send().await?;
        if !rep.status().is_success() {
            return Err(Error::msg("request fail"));
        }
        let headers = rep.headers();
        if headers
            .get(ACCEPT_RANGES)
            .map(|val| (val.to_str().ok()?.eq("bytes")).then(|| ()))
            .flatten()
            .is_some()
        {
            range = true;
        }
        let content_length = headers
        .get(CONTENT_LENGTH)
        .map(|val| val.to_str().ok())
        .flatten()
        .map(|val| val.parse().ok())
        .flatten().unwrap_or_default();
    
        Ok((range, content_length))
    }

    async fn download_easy(&self)->Result<()> {
        let req = reqwest::Client::new().get(self.source.as_str());
        let rep = req.send().await?;
        if !rep.status().is_success() {
            return Err(Error::msg("request fail"));
        }
        let mut stream = rep.bytes_stream();
        while let Some(chunk) = stream.next().await {
            
        }


        Ok(())
    }

}




async fn download_single_per<U: IntoUrl>(url: U, (start, end): (u64, u64), is_partial:bool) -> Result<()> {
    let req = reqwest::Client::new().get(url);

    let req = if is_partial {
        if end == u64::MAX {
            req.header(RANGE, format!("bytes={}-{}", start, ""))
        } else {
            req.header(RANGE, format!("bytes={}-{}", start, end))
        }
    } else {
        req
    };

    let rep = req.send().await?;
    if !rep.status().is_success() {
        return Err(Error::msg("request fail"));
    }
    let mut stream = rep.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let mut chunk = chunk?;
        offset += chunk.len() as u64;
    
    }
    Ok(())
} 
