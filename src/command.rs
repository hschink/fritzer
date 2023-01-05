use async_trait::async_trait;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use serde_xml_rs::from_str;
use std::error::Error;
use url::Url;

use crate::connection::SessionInfo;

#[async_trait]
pub trait Login {
    async fn get_session_info(
        &self,
        client: &reqwest::Client,
        url: &Url,
    ) -> Result<Option<SessionInfo>, Box<dyn Error>>;
    async fn connect_with_sid(
        &self,
        client: &reqwest::Client,
        url: &Url,
        sid: &str,
    ) -> Result<Option<SessionInfo>, Box<dyn Error>>;
    async fn connect_with_credentials(
        &self,
        client: &reqwest::Client,
        url: &Url,
        username: &str,
        password: &str,
    ) -> Result<Option<SessionInfo>, Box<dyn Error>>;
}

pub struct FritzboxLogin;
#[async_trait]
impl Login for FritzboxLogin {
    async fn get_session_info(
        &self,
        client: &reqwest::Client,
        url: &Url,
    ) -> Result<Option<SessionInfo>, Box<dyn Error>> {
        let request_url = url.join("/login_sid.lua?version=2").unwrap();
        let request = client.get(request_url).build().unwrap();
        let res = client.execute(request).await?;
        let body = res.text().await?;

        Ok(Some(from_str::<SessionInfo>(&body).unwrap()))
    }

    async fn connect_with_sid(
        &self,
        client: &reqwest::Client,
        url: &Url,
        sid: &str,
    ) -> Result<Option<SessionInfo>, Box<dyn Error>> {
        let body = format!("sid={}", sid);
        let request_url = url.join("/login_sid.lua?version=2").unwrap();
        let request = client
            .post(request_url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .build()
            .unwrap();
        let res = client.execute(request).await?;
        let body = res.text().await?;

        Ok(Some(from_str::<SessionInfo>(&body).unwrap()))
    }

    async fn connect_with_credentials(
        &self,
        client: &reqwest::Client,
        url: &Url,
        username: &str,
        response: &str,
    ) -> Result<Option<SessionInfo>, Box<dyn Error>> {
        let body = format!("username={}&response={}", username, response);
        let request_url = url.join("/login_sid.lua?version=2").unwrap();
        let request = client
            .post(request_url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .build()
            .unwrap();
        let res = client.execute(request).await?;
        let body = res.text().await?;

        Ok(Some(from_str::<SessionInfo>(&body).unwrap()))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Device {
    pub ain: String,
    pub name: String,
}

#[async_trait]
pub trait SwitchOperator {
    async fn get_switches(
        &self,
        client: &reqwest::Client,
        url: &Url,
        sid: &str,
    ) -> Result<Vec<Device>, Box<dyn Error>>;
}

pub struct FritzboxSwitchOperator;
#[async_trait]
impl SwitchOperator for FritzboxSwitchOperator {
    async fn get_switches(
        &self,
        client: &reqwest::Client,
        url: &Url,
        sid: &str,
    ) -> Result<Vec<Device>, Box<dyn Error>> {
        let command = format!(
            "/webservices/homeautoswitch.lua?switchcmd=getswitchlist&sid={}",
            sid
        );
        let request_url = url.join(&command).unwrap();
        let res = client.get(request_url).send().await;
        let body = res
            .expect("Could not read AINs for switches. Response has no body.")
            .text()
            .await;
        let text = body.expect("Could not read AINs for switches. Body has no content.");
        let text = text
            .strip_suffix("\n")
            .expect("Cannot strip trailing newline.");

        let ains: Vec<&str> = text.split(",").collect();
        let mut switches = Vec::new();

        for (_, ain) in ains.iter().enumerate() {
            let command = format!(
                "/webservices/homeautoswitch.lua?switchcmd=getswitchname&sid={}&ain={}",
                sid, ain
            );
            let request_url = url.join(&command).unwrap();
            let res = client.get(request_url).send().await;
            let body = res
                .expect("Could not read switch name for ains. Response has no body.")
                .text()
                .await;
            let name = body.expect("Could not read switch name.");
            let name = name
                .strip_suffix("\n")
                .expect("Cannot strip trailing newline.");
            let device = Device {
                ain: ain.to_string(),
                name: name.to_string(),
            };

            switches.push(device);
        }

        Ok(switches)
    }
}
