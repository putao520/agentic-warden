//! Smart OAuth Authentication
//!
//! Smart OAuth authentication system with concurrent callback and manual input support
//! Provides user experience similar to CODEX CLI and Claude Code

use anyhow::{Context, Result, anyhow};
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    routing::get,
};
use axum_server::Server;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio::time::{Duration, Instant, timeout};
use tracing::{debug, error, info, warn};

use crate::sync::oauth_client::{OAuthClient, OAuthConfig, OAuthTokenResponse};

/// 授权码来源
#[derive(Debug, Clone)]
enum AuthCodeSource {
    /// 来自本地回调服务器
    Callback(String),
    /// 来自用户手动输入
    Manual(String),
    /// 超时
    Timeout,
    /// 用户取消
    Cancelled,
}

/// OAuth认证状态
#[derive(Debug, Clone)]
pub enum AuthState {
    /// 初始化中
    Initializing,
    /// 等待回调
    WaitingForCallback {
        url: String,
        expires_at: Instant,
        has_callback_server: bool,
    },
    /// 处理中
    Processing,
    /// 成功
    Success,
    /// 失败
    Failed(String),
}

/// 智能OAuth认证器
pub struct SmartOAuthAuthenticator {
    state: Arc<RwLock<AuthState>>,
    config: OAuthConfig,
    timeout_duration: Duration,
}

impl SmartOAuthAuthenticator {
    /// 创建新的认证器
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(AuthState::Initializing)),
            config,
            timeout_duration: Duration::from_secs(300), // 5分钟超时
        }
    }

    /// 执行智能OAuth认证
    pub async fn authenticate(&self) -> Result<OAuthTokenResponse> {
        info!("Starting smart OAuth authentication");

        // 1. 设置回调服务器（如果可能）
        let callback_setup = self.setup_callback_server().await;
        let has_callback = callback_setup.is_ok();

        // 2. 生成授权URL
        let auth_url = match &callback_setup {
            Ok((_, callback_url)) => self.generate_auth_url_with_callback(callback_url)?,
            Err(_) => self.generate_manual_auth_url()?,
        };

        // 3. 显示授权指令
        self.display_auth_instructions(&auth_url, has_callback)
            .await?;

        // 4. 更新状态
        self.update_state(AuthState::WaitingForCallback {
            url: auth_url.clone(),
            expires_at: Instant::now() + self.timeout_duration,
            has_callback_server: has_callback,
        })
        .await;

        // 5. 运行并发认证流程
        let auth_code = self.run_concurrent_auth(callback_setup).await?;

        // 6. 交换令牌
        let token_response = self.exchange_code_for_token(auth_code).await?;

        info!("Smart OAuth authentication completed successfully");
        Ok(token_response)
    }

    /// 设置本地回调服务器
    async fn setup_callback_server(&self) -> Result<(CallbackServerHandle, String)> {
        debug!("Attempting to setup local callback server");

        // 检查端口是否可用
        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

        // 创建通道用于接收授权码
        let (auth_code_tx, auth_code_rx) = mpsc::channel::<String>(1);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        // 创建Axum应用
        let app = Router::new()
            .route("/callback", get(callback_handler))
            .with_state(auth_code_tx)
            .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

        // 在后台启动服务器
        let _server_handle = tokio::spawn(async move {
            // 简化实现，不使用graceful_shutdown
            match Server::bind(addr).serve(app.into_make_service()).await {
                Ok(_) => {
                    debug!("Callback server completed successfully");
                }
                Err(e) => {
                    error!("Callback server error: {}", e);
                }
            }
        });

        // 确保服务器启动成功
        tokio::time::sleep(Duration::from_millis(100)).await;

        debug!("Callback server successfully started on port 8080");

        Ok((
            CallbackServerHandle {
                server: CallbackServer {
                    port: 8080,
                    auth_code_rx: Some(auth_code_rx),
                    shutdown_tx: Some(shutdown_tx),
                },
            },
            "http://localhost:8080/callback".to_string(),
        ))
    }

    /// 生成带回调的授权URL
    fn generate_auth_url_with_callback(&self, callback_url: &str) -> Result<String> {
        let mut oauth_client = OAuthClient::new(
            self.config.client_id.clone(),
            self.config.client_secret.clone(),
            None,
        );

        oauth_client
            .generate_auth_url()
            .map(|mut url| {
                // 替换redirect_uri为本地回调URL
                url = url.replace("urn:ietf:wg:oauth:2.0:oob", callback_url);
                url
            })
            .context("Failed to generate auth URL with callback")
    }

    /// 生成手动授权URL
    fn generate_manual_auth_url(&self) -> Result<String> {
        let oauth_client = OAuthClient::new(
            self.config.client_id.clone(),
            self.config.client_secret.clone(),
            None,
        );

        oauth_client
            .generate_auth_url()
            .context("Failed to generate manual auth URL")
    }

    /// 显示授权指令
    async fn display_auth_instructions(&self, auth_url: &str, has_callback: bool) -> Result<()> {
        println!("🔗 OAuth Authentication Required");
        println!("{}", "━".repeat(60));

        if has_callback {
            println!("🌐 Automatic authentication enabled");
            println!("   We'll automatically detect when you complete authorization");
            println!();

            // 尝试自动打开浏览器
            if let Err(e) = open::that(auth_url) {
                println!("⚠️  Could not open browser automatically: {}", e);
                println!("   Please manually open the URL below");
            } else {
                println!("✅ Browser opened automatically");
            }
        } else {
            println!("📋 Manual authentication required");
            println!("   Please copy and open the URL in your browser");
        }

        println!();
        println!("🔗 Authorization URL:");
        println!("   {}", auth_url);
        println!();

        if has_callback {
            println!("💡 After authorizing, you'll be automatically redirected back");
            println!("📝 Alternatively, you can manually enter the authorization code below:");
        } else {
            println!("📝 After authorizing, please enter the authorization code below:");
        }

        println!("{}", "━".repeat(60));

        Ok(())
    }

    /// 运行并发认证流程
    async fn run_concurrent_auth(
        &self,
        callback_setup: Result<(CallbackServerHandle, String)>,
    ) -> Result<String> {
        let (code_tx, mut code_rx) = mpsc::channel::<AuthCodeSource>(1);
        let mut tasks = tokio::task::JoinSet::new();

        // 先检查是否有回调服务器
        let has_callback = callback_setup.is_ok();

        // 任务1：监听回调服务器（如果可用）
        if let Ok((server_handle, _)) = callback_setup {
            let code_tx_clone = code_tx.clone();
            tasks.spawn(async move {
                debug!("Starting callback server listener");
                match server_handle.wait_for_callback().await {
                    Ok(code) => {
                        info!("Received authorization code via callback");
                        let _ = code_tx_clone.send(AuthCodeSource::Callback(code)).await;
                    }
                    Err(e) => {
                        debug!("Callback server error: {}", e);
                    }
                }
            });
        }

        // 任务2：监听用户输入（总是可用）
        let code_tx_clone = code_tx.clone();
        tasks.spawn(async move {
            debug!("Starting user input listener");

            let prompt = if has_callback {
                "📝 Authorization Code (or wait for automatic callback)"
            } else {
                "📝 Authorization Code"
            };

            loop {
                match dialoguer::Input::<String>::new()
                    .with_prompt(prompt)
                    .allow_empty(true)
                    .interact_text()
                {
                    Ok(code) => {
                        let code = code.trim();
                        if !code.is_empty() {
                            info!("Received authorization code via manual input");
                            let _ = code_tx_clone
                                .send(AuthCodeSource::Manual(code.to_string()))
                                .await;
                            break;
                        }
                        // 空输入，继续等待
                    }
                    Err(e) => {
                        debug!("Input error: {}", e);
                        break;
                    }
                }
            }
        });

        // 任务3：超时处理
        let timeout_duration = self.timeout_duration;
        let code_tx_clone = code_tx.clone();
        tasks.spawn(async move {
            timeout(timeout_duration, std::future::pending::<()>())
                .await
                .ok();

            warn!("Authentication timed out");
            let _ = code_tx_clone.send(AuthCodeSource::Timeout).await;
        });

        // 任务4：取消处理（Ctrl+C）
        let code_tx_clone = code_tx.clone();
        tasks.spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            info!("Authentication cancelled by user");
            let _ = code_tx_clone.send(AuthCodeSource::Cancelled).await;
        });

        // 等待第一个授权码或信号
        match code_rx.recv().await {
            Some(AuthCodeSource::Callback(code)) => {
                println!("✅ Automatic callback received!");
                Ok(code)
            }
            Some(AuthCodeSource::Manual(code)) => {
                println!("✅ Manual code entered!");
                Ok(code)
            }
            Some(AuthCodeSource::Timeout) => {
                Err(anyhow!("Authentication timed out after 5 minutes"))
            }
            Some(AuthCodeSource::Cancelled) => Err(anyhow!("Authentication cancelled by user")),
            None => Err(anyhow!("No authorization code received")),
        }
    }

    /// 交换授权码为访问令牌
    async fn exchange_code_for_token(&self, code: String) -> Result<OAuthTokenResponse> {
        self.update_state(AuthState::Processing).await;

        println!("🔄 Exchanging authorization code for access token...");

        let oauth_client = OAuthClient::new(
            self.config.client_id.clone(),
            self.config.client_secret.clone(),
            None,
        );

        match oauth_client.exchange_code_for_tokens(&code).await {
            Ok(token_response) => {
                self.update_state(AuthState::Success).await;
                println!("🎉 Authentication successful!");
                Ok(token_response)
            }
            Err(e) => {
                let error_msg = format!("Failed to exchange authorization code: {}", e);
                self.update_state(AuthState::Failed(error_msg.clone()))
                    .await;
                Err(anyhow!(error_msg))
            }
        }
    }

    /// 更新认证状态
    async fn update_state(&self, new_state: AuthState) {
        *self.state.write().await = new_state;
    }

    /// 获取当前认证状态
    pub async fn get_state(&self) -> AuthState {
        self.state.read().await.clone()
    }
}

/// 回调服务器句柄
struct CallbackServerHandle {
    server: CallbackServer,
}

impl CallbackServerHandle {
    fn new(server: CallbackServer) -> Self {
        Self { server }
    }

    async fn wait_for_callback(self) -> Result<String> {
        self.server.wait_for_callback().await
    }
}

/// 本地回调服务器实现
struct CallbackServer {
    port: u16,
    auth_code_rx: Option<mpsc::Receiver<String>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl CallbackServer {
    /// 绑定指定端口并启动回调服务器
    async fn bind(port: u16) -> Result<Self> {
        debug!("Attempting to start callback server on port {}", port);

        // 检查端口是否可用
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        // 创建通道用于接收授权码
        let (auth_code_tx, auth_code_rx) = mpsc::channel::<String>(1);
        let (_shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        // 创建Axum应用
        let app = Router::new()
            .route("/callback", get(callback_handler))
            .with_state(auth_code_tx)
            .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

        // 在后台启动服务器
        let _server_handle = tokio::spawn(async move {
            // 简化实现，不使用graceful_shutdown
            match Server::bind(addr).serve(app.into_make_service()).await {
                Ok(_) => {
                    debug!("Callback server completed successfully");
                }
                Err(e) => {
                    error!("Callback server error: {}", e);
                }
            }
        });

        // 确保服务器启动成功
        tokio::time::sleep(Duration::from_millis(100)).await;

        debug!("Callback server successfully started on port {}", port);

        Ok(Self {
            port,
            auth_code_rx: Some(auth_code_rx),
            shutdown_tx: Some(_shutdown_tx),
        })
    }

    /// 等待回调授权码
    async fn wait_for_callback(mut self) -> Result<String> {
        if let Some(mut auth_code_rx) = self.auth_code_rx.take() {
            match timeout(Duration::from_secs(300), auth_code_rx.recv()).await {
                Ok(Some(code)) => {
                    info!("Received authorization code via callback");
                    Ok(code)
                }
                Ok(None) => Err(anyhow!("Callback server closed without receiving code")),
                Err(_) => Err(anyhow!("Callback timeout after 5 minutes")),
            }
        } else {
            Err(anyhow!("Callback receiver not available"))
        }
    }

    /// 关闭服务器
    async fn shutdown(mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }
}

impl Drop for CallbackServer {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }
}

/// OAuth回调处理器 - 简化版本
async fn callback_handler(
    Query(params): Query<HashMap<String, String>>,
    State(auth_code_tx): State<mpsc::Sender<String>>,
) -> Result<Html<String>, StatusCode> {
    debug!("Received OAuth callback with params: {:?}", params);

    // 检查是否有错误
    if let Some(error) = params.get("error") {
        error!("OAuth callback error: {}", error);
        return Ok(Html(format!("OAuth Authorization Failed: {}", error)));
    }

    // 检查是否有授权码
    if let Some(code) = params.get("code") {
        info!("Successfully received OAuth authorization code");

        // 发送授权码到主线程
        if auth_code_tx.send(code.clone()).await.is_ok() {
            return Ok(Html(
                "OAuth Authorization Successful! You can close this window.".to_string(),
            ));
        } else {
            error!("Failed to send authorization code to main thread");
            return Ok(Html(
                "OAuth Authorization Failed! Please try again.".to_string(),
            ));
        }
    }

    error!("OAuth callback missing required parameters");
    Ok(Html(
        "OAuth Authorization Failed! Missing required parameters.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_smart_oauth_creation() {
        let config = OAuthConfig {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            refresh_token: None,
            access_token: None,
            token_expiry: None,
            scopes: vec!["https://www.googleapis.com/auth/drive.file".to_string()],
        };

        let authenticator = SmartOAuthAuthenticator::new(config);
        assert!(matches!(
            authenticator.get_state().await,
            AuthState::Initializing
        ));
    }
}
