use gcp_auth::{AuthenticationManager, Token};
pub use goauth::scopes::Scope;
use tokio::sync::OnceCell;

/// A module for managing a Google API access token
use {
    log::*,
    std::sync::{
        atomic::{AtomicBool, Ordering},
        {Arc, RwLock},
    },
    tokio::time,
};

static AUTH_MANAGER: OnceCell<AuthenticationManager> = OnceCell::const_new();

async fn authentication_manager() -> &'static AuthenticationManager {
    AUTH_MANAGER
        .get_or_init(|| async {
            AuthenticationManager::new()
                .await
                .expect("unable to initialize authentication manager")
        })
        .await
}

#[derive(Clone)]
pub struct AccessToken {
    scope: Scope,
    refresh_active: Arc<AtomicBool>,
    token: Arc<RwLock<Token>>,
}

impl AccessToken {
    pub async fn new(scope: Scope) -> Result<Self, String> {
        let token = Arc::new(RwLock::new(Self::get_token(&scope).await?));
        let access_token = Self {
            scope,
            token,
            refresh_active: Arc::new(AtomicBool::new(false)),
        };
        Ok(access_token)
    }

    async fn get_token(scope: &Scope) -> Result<Token, String> {
        let authentication_manager = authentication_manager().await;
        let scope_url = scope.url();
        let scopes = &[scope_url.as_str()];
        let token = authentication_manager
            .get_token(scopes)
            .await
            .map_err(|err| format!("Unable to get token: {}", err))?;

        info!("Got token {:?}", token);
        Ok(token)
    }

    /// Call this function regularly to ensure the access token does not expire
    pub fn refresh(&self) {
        // Check if it's time to try a token refresh
        {
            let token_r = self.token.read().unwrap();
            if !token_r.has_expired() {
                return;
            }

            #[allow(deprecated)]
            if self
                .refresh_active
                .compare_and_swap(false, true, Ordering::Relaxed)
            {
                // Refresh already pending
                return;
            }
        }

        info!("Refreshing token");

        let this = self.clone();
        tokio::spawn(async move {
            match time::timeout(
                time::Duration::from_secs(5),
                Self::get_token(&this.scope),
            )
            .await
            {
                Ok(new_token) => match new_token {
                    Ok(new_token) => {
                        let mut token_w = this.token.write().unwrap();
                        *token_w = new_token;
                    }
                    Err(err) => error!("Failed to fetch new token: {}", err),
                },
                Err(_timeout) => {
                    warn!("Token refresh timeout")
                }
            }
            this.refresh_active.store(false, Ordering::Relaxed);
            info!("Token refreshed");
        });
    }

    /// Return an access token suitable for use in an HTTP authorization header
    pub fn get(&self) -> String {
        let token_r = self.token.read().unwrap();
        format!("Bearer {}", token_r.as_str())
    }
}
