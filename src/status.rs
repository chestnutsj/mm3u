use memmap2::{ MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::io::{Result, Read,Write};
use std::path::{Path, PathBuf};

pub struct Status {
    info :  PathBuf,
    page_size: usize,
    mmap: Option<MmapMut>,
}

static STATUS_HEADER:usize = std::mem::size_of::<usize>()+  std::mem::size_of::<usize>();

impl Status {
   // 从文件中读取状态信息
   pub fn new<P: AsRef<Path>>(filename: P) -> Result<Self> {

    let meta = std::fs::metadata(filename.as_ref());
    let file_len = match meta {
        Ok(d) => d.len(),
        Err(_) => 0,
    };
 
    let mut status = Status {
        info : filename.as_ref().to_path_buf(),
        page_size: 0,
        mmap: None,
    };
    
    if file_len > STATUS_HEADER as u64 {
        let file = OpenOptions::new().read(true).write(true).create(true).open(filename)?;
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        let mut cursor = std::io::Cursor::new(&mmap);
        let mut page_size_buf = [0; std::mem::size_of::<usize>()];
        cursor.read_exact(&mut page_size_buf)?;
        status.page_size = usize::from_ne_bytes(page_size_buf);
        status.mmap = Some(mmap);
    }

    Ok(status)
    }
    pub fn get_page_len(&self,len:u64)->usize {
        let mut page_len = len / self.page_size as u64;
        if len%self.page_size as u64 != 0 {
            page_len+=1;
        }
        return page_len as usize;
    }

    pub fn set_page(&mut self, size :usize, len :u64)->  Result<()>{
        self.page_size = size;

        let mut page_len = self.get_page_len(len);
     
        let total_len = STATUS_HEADER+ page_len *std::mem::size_of::<u32>();
        let file = OpenOptions::new().read(true).write(true).create(true).open(&self.info)?;
        file.set_len(total_len as u64) ?;
        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        mmap.fill(0);
        (&mut mmap[0..]).write_all (&self.page_size.to_ne_bytes())?;

        (&mut mmap[std::mem::size_of::<usize>()..]).write_all(&mut page_len.to_ne_bytes())?;
        mmap.flush()?;
        self.mmap = Some( mmap );
        Ok(())
    }

    pub fn update(&mut self,idx :usize, crc: u32)->Result<()>{
        match &mut self.mmap {
            Some(data) => {

                let crc_byte = crc.to_le_bytes();
                let offset = STATUS_HEADER + (idx * std::mem::size_of::<u32>());
                data[offset .. offset +std::mem::size_of::<u32>() ].copy_from_slice(&crc_byte);

                data.flush_range(offset, std::mem::size_of::<u32>())?;
            }
            None => {}
        }
        Ok(())
    }

    pub fn find_crc32(&self) -> Vec<u32> {
        if let Some(data) = &self.mmap {
            let crc32_data = &data[STATUS_HEADER..];  
            let mut result = Vec::new();
            for (_, chunk) in crc32_data.chunks_exact(4).enumerate() {
                let value: u32 = match chunk.try_into() {
                    Ok(data) => u32::from_le_bytes(data),
                    Err(_) => 0, // 发生错误时返回默认值0
                };
                result.push(value);
            }
            result
        } else {
            Vec::new()
        }
    }

    pub fn is_init(&self)->bool {
        self.page_size != 0 
    }
    pub fn get_size(&self)->usize {
        self.page_size
    }
    
}