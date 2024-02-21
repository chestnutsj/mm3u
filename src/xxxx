use anyhow::{Error, Result};
use futures::future::join_all;
use futures::StreamExt;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
use reqwest::IntoUrl;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::fs::{self, File,SeekFrom};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use std::env;
use std::process;

pub async fn check_request_range<U: IntoUrl>(url: U) -> Result<(bool, u64)> {
    let mut range = false;
    let req = reqwest::Client::new().head(url);
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
    let length = headers
        .get(CONTENT_LENGTH)
        .map(|val| val.to_str().ok())
        .flatten()
        .map(|val| val.parse().ok())
        .flatten()
        .ok_or(Error::msg("get length fail"))?;
    Ok((range, length))
}

async fn download<U: IntoUrl>(url: U, (mut start, end): (u64, u64), is_partial: bool,file: Arc<Mutex<File>>) -> Result<()> {
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
        file.seek(SeekFrom::Start(start)).await?;
        start += chunk.len() as u64;
        file.write_all_buf(&mut chunk).await?;
    }
    Ok(())
} 

async fn download_retry<U: IntoUrl>(url: U, (mut start, end): (u64, u64), is_partial: bool,file: Arc<Mutex<File>>, max_retry :usize) -> Result<()> {
    let mut retry = 0;
    loop {
        match download(url,(start,end), is_partial, file).await {
            Ok(_) => break,
            Err(err) => {
                if retry >= max_retry {
                    return Err(err);
                }
                println!("Failed to download file: {}. Retrying...", err);
                retry += 1;
                tokio::time::sleep(Duration::from_secs(1)).await; // 等待一段时间后重试
            }
        }
    }
    Ok(())
}

pub async fn new_run<U: IntoUrl, P: AsRef<Path>>(url: U, path: P,  concurrency: usize,max_retry :usize ) -> Result<()> {
    let url = url.into_url()?;
    let mut handles = vec![];
    let (range, length) = check_request_range(url.clone()).await?;
    
    let file = Arc::new(Mutex::new(File::create(&path).await?));


    let is_error = if range {
        let task_length = length / concurrency;
        for i in 0..(concurrency - 1) {        // 线程数必须大于等于1
            let file = Arc::clone(&file);
            handles.push(tokio::spawn(download(
                url.clone(),
                (task_length * i, task_length * (i + 1) - 1),
                true,
                file,
            )));
        }
        { 
            let file = Arc::clone(&file);
            handles.push(tokio::spawn(
                download(url.clone(), (task_length * (concurrency - 1), u64::MAX), true, file)
            ));
        }
        
        let ret = join_all(handles).await;
        drop(file);
        ret.into_iter().flatten().any(|n| n.is_err())
    } else {
        download(url.clone(), (0, length - 1), false, file)
            .await
            .is_err()
    };
    if is_error {
        remove_file(&path).await?;
        Err(Error::msg("download file error"))
    } else {
        Ok(())
    }
}
