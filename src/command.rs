use async_trait::async_trait;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use serde_xml_rs::from_str;
use std::error::Error;
use url::Url;

use crate::connection::SessionInfo;

async fn get_request(client: &reqwest::Client, url: &Url) -> Result<String, Box<dyn Error>> {
    let request = client.get(url.as_str()).build().unwrap();
    let res = client.execute(request).await?;

    Ok(res.text().await?)
}

async fn get_request_with_command_path(
    client: &reqwest::Client,
    url: &Url,
    command_path: &str,
) -> Result<String, Box<dyn Error>> {
    let request_url = url.join(command_path).unwrap();

    get_request(client, &request_url).await
}

async fn get_request_with_query(
    client: &reqwest::Client,
    url: &Url,
    command_path: &str,
    query: &str,
) -> Result<String, Box<dyn Error>> {
    let mut request_url = url.join(command_path).unwrap();

    request_url.set_query(Some(query));

    get_request(client, &request_url).await
}

async fn post_request(
    client: &reqwest::Client,
    url: &Url,
    command_path: &str,
    body: &str,
) -> Result<String, Box<dyn Error>> {
    let body = String::from(body);
    let request_url = url.join(command_path).unwrap();
    let request = client
        .post(request_url)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .build()
        .unwrap();
    let res = client.execute(request).await?;

    Ok(res.text().await?)
}

pub trait Command {
    const COMMAND_PATH: &'static str;
}

#[async_trait]
pub trait Login: Command {
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
impl Command for FritzboxLogin {
    const COMMAND_PATH: &'static str = "/login_sid.lua?version=2";
}

#[async_trait]
impl Login for FritzboxLogin {
    async fn get_session_info(
        &self,
        client: &reqwest::Client,
        url: &Url,
    ) -> Result<Option<SessionInfo>, Box<dyn Error>> {
        let response = get_request_with_command_path(client, url, Self::COMMAND_PATH).await?;

        Ok(Some(from_str::<SessionInfo>(&response).unwrap()))
    }

    async fn connect_with_sid(
        &self,
        client: &reqwest::Client,
        url: &Url,
        sid: &str,
    ) -> Result<Option<SessionInfo>, Box<dyn Error>> {
        let body = format!("sid={}", sid);
        let response = post_request(client, url, Self::COMMAND_PATH, &body).await?;

        Ok(Some(from_str::<SessionInfo>(&response).unwrap()))
    }

    async fn connect_with_credentials(
        &self,
        client: &reqwest::Client,
        url: &Url,
        username: &str,
        response: &str,
    ) -> Result<Option<SessionInfo>, Box<dyn Error>> {
        let body = format!("username={}&response={}", username, response);
        let response = post_request(client, url, Self::COMMAND_PATH, &body).await?;

        Ok(Some(from_str::<SessionInfo>(&response).unwrap()))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Device {
    pub ain: String,
    pub name: String,
}

#[async_trait]
pub trait SwitchOperator: Command {
    async fn get_switch(
        &self,
        client: &reqwest::Client,
        url: &Url,
        sid: &str,
        ain: &str,
    ) -> Result<Device, Box<dyn Error>>;

    async fn get_switches(
        &self,
        client: &reqwest::Client,
        url: &Url,
        sid: &str,
    ) -> Result<Vec<Device>, Box<dyn Error>>;
}

pub struct FritzboxSwitchOperator;
impl Command for FritzboxSwitchOperator {
    const COMMAND_PATH: &'static str = "/webservices/homeautoswitch.lua";
}

#[async_trait]
impl SwitchOperator for FritzboxSwitchOperator {
    async fn get_switch(
        &self,
        client: &reqwest::Client,
        url: &Url,
        sid: &str,
        ain: &str,
    ) -> Result<Device, Box<dyn Error>> {
        let query = format!("switchcmd=getswitchname&sid={}&ain={}", sid, ain);
        let body = get_request_with_query(client, url, Self::COMMAND_PATH, &query).await?;
        let name = body
            .strip_suffix("\n")
            .expect("Cannot strip trailing newline.");

        Ok(Device {
            ain: ain.to_string(),
            name: name.to_string(),
        })
    }

    async fn get_switches(
        &self,
        client: &reqwest::Client,
        url: &Url,
        sid: &str,
    ) -> Result<Vec<Device>, Box<dyn Error>> {
        let query = format!("switchcmd=getswitchlist&sid={}", sid);

        let body = get_request_with_query(client, url, Self::COMMAND_PATH, &query).await?;
        let text = body
            .strip_suffix("\n")
            .expect("Cannot strip trailing newline.");

        let ains: Vec<&str> = text.split(",").collect();
        let mut switches = Vec::new();

        for (_, ain) in ains.iter().enumerate() {
            let device = self.get_switch(client, url, sid, &ain).await?;

            switches.push(device);
        }

        Ok(switches)
    }
}
