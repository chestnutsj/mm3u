use std::path::{PathBuf,Path};
use anyhow::{Error, Ok, Result};
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
use reqwest::{IntoUrl,Url};
use futures_util::StreamExt;
use tokio::fs::{File,OpenOptions};
use crate::status::Status;
use tokio::io::{AsyncReadExt, AsyncWriteExt,AsyncSeekExt};
use tokio::sync::mpsc;
use std::io::SeekFrom;
use tokio::task;
use std::sync::{Arc, Mutex};

pub struct Pdata{
    index: usize,
    offset : u64,
    data: Vec<u8>
}


pub struct DownTask {
    source: Url,
    output_dir: PathBuf,
    name: String,
    status: Status,
}

static LIMITE_PATE_SIZE:u64 = 1024;


impl DownTask {
    pub async fn new<U: IntoUrl, P: AsRef<Path>>(url :U, dir:P,name: &str) -> Result<Self> {
       
        // 创建输出目录
        if !Path::new(dir.as_ref()).exists() {
            tokio::fs::create_dir_all(dir.as_ref()).await?;
        }

       let mut status_name= PathBuf::from(dir.as_ref());
       status_name.push( name);
       status_name.set_extension(".mm3u");
       
       let task = DownTask{
        source: url.into_url()?,
        output_dir: dir.as_ref().to_path_buf(),
        name: name.into(),
        status:  Status::new(status_name)?,
       };
       Ok(task)
    }

    pub async fn down_chunk(&self,idx :usize, start :u64, end: u64, sender: mpsc::Sender<Pdata>) ->Result<()> {
        let req = reqwest::Client::new().get(self.source.as_str());
        let req  = if end == u64::MAX {
            req.header(RANGE, format!("bytes={}-{}", start, ""))
        } else {
            req.header(RANGE, format!("bytes={}-{}", start, end))
        };
       
        let rep = req.send().await?;
        if !rep.status().is_success() {
            return Err(Error::msg("request fail"));
        }
        let bytes = rep.bytes().await?;
        
        let pdata = Pdata {
            index:idx,
            offset:start,
            data: bytes.to_vec(),
        };

        sender.send(pdata).await?;
        Ok(())
    }

    async fn write_to_file(receiver:&mut mpsc::Receiver<Pdata> ,file :&mut File , status: &Arc<Mutex<Status>>) -> Result<()> {
        let mut status = status.lock();

        while let Some(chunk) = receiver.recv().await {
            // 写入下载的数据到文件
            file.seek(SeekFrom::Start(chunk.offset)).await?;
            file.write_all(&chunk.data).await?;
            let checksum = crc32fast::hash(&chunk.data);
            status.update(chunk.index, checksum)?;
        }
        Ok(())
    }
    

    pub async fn muti_download(&mut self, total_len : u64)->Result<()> {
        let mut file = PathBuf::from(self.output_dir.clone());
        file.push(self.name.as_str());
        let  mut rr = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(true) // 追加写入
        .open(file)
        .await?;  

        self.status.set_page(LIMITE_PATE_SIZE as usize, total_len)?;
     
        let (sender,mut receiver) = mpsc::channel::<Pdata>(10);
        let status = Arc::new(Mutex::new(self.status));

        // each 
        let write_task = task::spawn(
            async move {
                DownTask::write_to_file(&mut receiver,&mut rr , &status).await;
            }
            );
        

        write_task.await?;
        
        Ok(())
    }


    async fn download_easy(&mut self)->Result<()> {
        let req = reqwest::Client::new().get(self.source.as_str());
        let rep = req.send().await?;
        if !rep.status().is_success() {
            return Err(Error::msg("request fail"));
        }
        
        let mut file = PathBuf::from(self.output_dir.clone());
        file.push(self.name.as_str());
        let mut result = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file)
        .await?;
       
        let mut stream = rep.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            result.write_all(&chunk).await?;       
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


    async fn check_compile_file(&mut self,page_size :usize ,crc32 :&[u32]) ->Result<bool> {
        let mut file_name = PathBuf::from(self.output_dir.clone());
        file_name.push(self.name.as_str());
        let mut check_file = OpenOptions::new()
        .read(true)
        .open(file_name)
        .await?;

        let file_size = check_file.metadata().await?.len() as u64;
        
        // 创建缓冲区来读取文件的一个页
        let mut buffer = vec![0; page_size];

        let num_pages = ((file_size + page_size as u64  - 1) / page_size as u64) as usize;

        let mut all_error = true;
 
        for page_number in 0..num_pages {
           
            if crc32[page_number] != 0  {
                let start_position = page_number * page_size;
                // 计算当前页的大小
                let mut page_size_actual = page_size as usize;

                // 如果是最后一页，且不是整数页的倍数，调整当前页的大小
                if page_number == num_pages  - 1 {
                    page_size_actual = ( file_size % ( page_size as u64)) as usize;
                }
                // 从文件读取当前页的内容
            
                check_file.seek(SeekFrom::Start(start_position as u64)).await?;
                check_file.read_exact(&mut buffer[..page_size_actual]).await?;
                
                let checksum = crc32fast::hash(&buffer[..page_size_actual]);

                if crc32[page_number] != checksum {
                    self.status.update(page_number, 0)?;
                } else {
                    all_error = false;
                }  
            }
        }

        if num_pages < crc32.len() {
            for x  in num_pages .. crc32.len() {
                self.status.update(x, 0)?;
            }
        }
        Ok(all_error)
    }

 


 pub async fn start(&mut self)->Result<()> {

        let (range,len) =self.check_request_range().await ?;
    
        if !self.status.is_init() {
            
            if range && len > (LIMITE_PATE_SIZE *LIMITE_PATE_SIZE)  {
                // new muti download
               self.muti_download(len).await?;
            } else {
                self.download_easy().await?;
            }
        } else {
           let crc_res =self.status.find_crc32();
           let  page_len =   self.status.get_page_len(len);
           let mut restart = true;

            if  crc_res.len() ==  page_len {
               let check_complete =self.check_compile_file( page_len, &crc_res).await?; 
                if !check_complete {
                    restart = false;
                }
            }
            if restart {
                if range {
                    // new mutidownload
                } else {
                    self.download_easy().await?;
                }
            } else {
               // continue download
            }

        }
        Ok(())
    }

    
}




 