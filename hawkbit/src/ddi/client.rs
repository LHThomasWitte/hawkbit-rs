// Copyright 2020, Collabora Ltd.
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::convert::TryInto;

use thiserror::Error;
use url::Url;

use crate::ddi::poll;

/// [Direct Device Integration](https://www.eclipse.org/hawkbit/apis/ddi_api/) client.
#[derive(Debug, Clone)]
pub struct Client {
    base_url: Url,
    client: reqwest::Client,
}

/// The method of Authorization for the client and the secret authentification token.
#[derive(Debug, Clone)]
pub enum ClientAuthorization {
    /// use a target token that is unique per target
    TargetToken(String),
    /// use a common gateway token for all targets
    GatewayToken(String),
    /// do not send an authorization header
    None,
}

/// DDI errors
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    /// URL error
    #[error("Could not parse url")]
    ParseUrlError(#[from] url::ParseError),
    /// Token error
    #[error("Invalid token format")]
    InvalidToken(#[from] reqwest::header::InvalidHeaderValue),
    /// HTTP error
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    /// Error parsing sleep field from server
    #[error("Failed to parse polling sleep")]
    InvalidSleep,
    /// IO error
    #[error("Failed to download update")]
    Io(#[from] std::io::Error),
    /// Invalid checksum
    #[cfg(feature = "hash-digest")]
    #[error("Invalid Checksum")]
    ChecksumError(crate::ddi::deployment_base::ChecksumType),
}

impl Client {
    /// Create a new DDI client.
    ///
    /// # Arguments
    /// * `url`: the URL of the hawkBit server, such as `http://my-server.com:8080`
    /// * `tenant`: the server tenant
    /// * `controller_id`: the id of the controller
    /// * `authorization`: the authorization method and secret authentification token of the controller
    pub fn new(
        url: &str,
        tenant: &str,
        controller_id: &str,
        authorization: ClientAuthorization,
    ) -> Result<Self, Error> {
        let host: Url = url.parse()?;
        let path = format!("{}/controller/v1/{}", tenant, controller_id);
        let base_url = host.join(&path)?;

        let mut headers = reqwest::header::HeaderMap::new();
        match authorization {
            ClientAuthorization::TargetToken(key_token) => {
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    format!("TargetToken {}", &key_token).try_into()?,
                );
            },
            ClientAuthorization::GatewayToken(key_token) => {
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    format!("GatewayToken {}", &key_token).try_into()?,
                );
            },
            ClientAuthorization::None => {
                // no authorization header needed
            },
        }
        
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self { base_url, client })
    }

    /// Poll the server for updates
    pub async fn poll(&self) -> Result<poll::Reply, Error> {
        let reply = self.client.get(self.base_url.clone()).send().await?;
        reply.error_for_status_ref()?;

        let reply = reply.json::<poll::ReplyInternal>().await?;
        Ok(poll::Reply::new(reply, self.client.clone()))
    }
}
