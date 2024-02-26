#[cfg(test)]
mod tests {
    use mm3u::download::http::DownTask;
 
    #[test]
    fn test_resource() {
        let temp_dir = tempdir().unwrap();
        
        let ubuntu22="https://releases.ubuntu.com/22.04.4/ubuntu-22.04.4-desktop-amd64.iso?_gl=1*10dhexa*_gcl_au*OTc5NTI1NzMzLjE3MDg5MzcxODA.&_ga=2.257993807.454263020.1708937173-961396022.1704180734";
        let mut task = DownTask::new( ubuntu22, temp_dir, "ubuntu2204");
        task.start().await?;
    }
}