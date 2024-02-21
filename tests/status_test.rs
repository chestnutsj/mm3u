
 



#[cfg(test)]
mod tests {
    use mm3u::status::Status;
    use tempfile::tempdir;
    #[test]
    fn test_status() {
        // 创建临时文件
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_status.txt");
        let filename = file_path.to_str().unwrap();

        // 创建一个测试用的 Status 实例
        let mut ss = Status::new(filename).expect("Failed to create Status");
        
        ss.set_page(1024, 1024*10+1).expect("Failed to set page");
        
        // 更新位图
        ss.update(2).expect("Failed to update");


        // 读取文件内容验证写入是否成功
        let file_content = std::fs::read(filename).unwrap();

        assert_eq!(file_content.len(),  std::mem::size_of::<usize>()*2 + 11); // 检查文件长度是否正确，page_size(4字节) + page_len(4字节) + bitmap(每个bool占1字节)
    }
}