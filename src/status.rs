use memmap2::{ MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::io::{Result, Read,Write};
use std::path::{Path, PathBuf};

pub struct Status {
    info : String,
    page_size: usize,
    mmap: Option<MmapMut>,
}

static STATUS_HEADER:usize = std::mem::size_of::<usize>()*2;

impl Status {
   // 从文件中读取状态信息
   pub fn new(filename: &str) -> Result<Self> {


    let meta = std::fs::metadata(filename);

    let file_len = match meta {
        Ok(d) => d.len(),
        Err(_) => 0,
    };
 
    let mut status = Status {
        info : filename.to_string(),
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

    pub fn set_page(&mut self, size :usize, len :usize)->  Result<()>{
        self.page_size = size;
        let mut page_len = len / size;
        if len%size != 0 {
            page_len+=1;
        }

        let total_len = STATUS_HEADER+ page_len;
        let file = OpenOptions::new().read(true).write(true).create(true).open(self.info)?;
        file.set_len(total_len as u64) ?;
        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        mmap.fill(0);
        mmap.copy_from_slice(&self.page_size.to_ne_bytes());
        (&mut mmap[std::mem::size_of::<usize>()..]).write_all(&mut page_len.to_ne_bytes())?;
        mmap.flush()?;
        self.mmap = Some( mmap );
        Ok(())
    }

    pub fn update(&mut self,idx :usize)->Result<()>{
        match &mut self.mmap {
            Some(data) => {
                data[STATUS_HEADER + idx + 1] = 1;
                data.flush_range(STATUS_HEADER + idx, 2)?;
            }
            None => {}
        }
        Ok(())
    }

    pub fn find_non_zero_indexes(&self) -> Vec<usize> {
        if let Some(data) = &self.mmap {
            data.iter()
                .enumerate()
                .skip(STATUS_HEADER+1) // 从索引为 10 的位置开始迭代
                .filter(|(_, &value)| value != 0) // 过滤出值不为 0 的元素
                .map(|(index, _)| index) // 提取索引
                .collect() // 将结果收集到向量中并返回
        } else {
            Vec::new() // 如果 mmap 为 None，则返回空向量
        }
    }

  
}


