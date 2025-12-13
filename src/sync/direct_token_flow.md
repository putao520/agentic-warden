# 直接访问令牌流程

## 用户操作步骤

1. 打开浏览器，访问：
   ```
   https://developers.google.com/oauthplayground/
   ```

2. 在OAuth Playground中：
   - Step 1: 勾选 "Google Drive API v3"
   - Step 2: 勾选 "https://www.googleapis.com/auth/drive.file"
   - Step 3: 点击 "Authorize APIs"
   - 用Google账号登录并授权

3. 复制生成的访问令牌

4. 配置到工具：
   ```bash
   mkdir -p ~/.aiw
   echo '{
     "access_token": "paste_your_token_here"
   }' > ~/.aiw/google_drive.json
   ```

## 代码实现

```rust
#[derive(Deserialize)]
struct DirectTokenConfig {
    access_token: String,
}

impl DirectTokenConfig {
    fn load() -> Result<Self> {
        let config_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot find home directory"))?
            .join(".aiw/google_drive.json");

        let content = fs::read_to_string(config_path)?;
        let config: DirectTokenConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
}

pub struct DirectGoogleDriveClient {
    client: reqwest::Client,
    token: String,
}

impl DirectGoogleDriveClient {
    pub fn new() -> Result<Self> {
        let config = DirectTokenConfig::load()?;
        Ok(Self {
            client: reqwest::Client::new(),
            token: config.access_token,
        })
    }

    pub async fn upload_file(&self, file_path: &Path, filename: &str) -> Result<DriveFile> {
        let url = "https://www.googleapis.com/upload/drive/v3/files?uploadType=media";

        let mut form = multipart::Form::new();
        form.add_file("file", file_path).part_filename(filename);
        form.add_text("name", filename);
        form.add_text("parents", "appDataFolder");

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(form)
            .send()
            .await?;

        // 处理响应...
        todo!()
    }
}
```

## 优缺点

### 优点：
- 完全去OAuth复杂性
- 用户只需要一次性配置
- 不需要处理Device Flow
- 不依赖我们的Client ID

### 缺点：
- 用户需要手动获取令牌
- 令牌会过期，需要定期更新
- 对技术新手来说仍然复杂
- 需要处理令牌刷新逻辑

## 推荐做法

考虑混合方案：
1. 默认移除OAuth，直接令牌模式
2. 保留OAuth作为可选的高级功能
3. 在README中提供详细的手动获取令牌教程
4. 添加令牌自动检测和刷新